//! Atomic projection of a validated safety counterexample into unified lineage.

use std::collections::BTreeSet;

use mrr_identity::{FactId, LineageEdgeId, LineageNodeId};
use mrr_lineage::{
    LineageEdge, LineageEdgeKind, LineageGraph, LineageGraphError, LineageNode, LineageNodeKind,
};
use mrr_transition::{CounterexampleIr, TransitionSystem};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CounterexampleFactRole {
    Source,
    Derived,
}

/// Caller-owned node identity and semantic role for one fact used by a counterexample.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CounterexampleFactIdentity {
    fact: FactId,
    node: LineageNodeId,
    role: CounterexampleFactRole,
}

impl CounterexampleFactIdentity {
    #[must_use]
    pub const fn source(fact: FactId, node: LineageNodeId) -> Self {
        Self {
            fact,
            node,
            role: CounterexampleFactRole::Source,
        }
    }

    #[must_use]
    pub const fn derived(fact: FactId, node: LineageNodeId) -> Self {
        Self {
            fact,
            node,
            role: CounterexampleFactRole::Derived,
        }
    }

    #[must_use]
    pub const fn fact(&self) -> FactId {
        self.fact
    }

    #[must_use]
    pub const fn node(&self) -> LineageNodeId {
        self.node
    }
}

/// Complete caller-assigned identity set for one counterexample lineage graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CounterexampleLineageIdentities {
    violation: LineageNodeId,
    states: Vec<LineageNodeId>,
    transitions: Vec<LineageNodeId>,
    facts: Vec<CounterexampleFactIdentity>,
    edges: Vec<LineageEdgeId>,
}

impl CounterexampleLineageIdentities {
    #[must_use]
    pub fn new(
        violation: LineageNodeId,
        states: Vec<LineageNodeId>,
        transitions: Vec<LineageNodeId>,
        facts: Vec<CounterexampleFactIdentity>,
        edges: Vec<LineageEdgeId>,
    ) -> Self {
        Self {
            violation,
            states,
            transitions,
            facts,
            edges,
        }
    }

    #[must_use]
    pub const fn violation(&self) -> LineageNodeId {
        self.violation
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CounterexampleLineageError {
    StateCountMismatch { required: usize, actual: usize },
    TransitionCountMismatch { required: usize, actual: usize },
    FactSetMismatch,
    EdgeCountMismatch { required: usize, actual: usize },
    UnknownAction,
    CounterexampleShapeMismatch,
    Graph(LineageGraphError),
}

/// Builds lineage only after the complete caller-owned identity binding is validated.
pub fn counterexample_lineage(
    counterexample: &CounterexampleIr,
    system: &TransitionSystem,
    identities: &CounterexampleLineageIdentities,
) -> Result<LineageGraph, CounterexampleLineageError> {
    let state_count = counterexample.states().len();
    let transition_count = counterexample.steps().len();
    if identities.states.len() != state_count {
        return Err(CounterexampleLineageError::StateCountMismatch {
            required: state_count,
            actual: identities.states.len(),
        });
    }
    if identities.transitions.len() != transition_count {
        return Err(CounterexampleLineageError::TransitionCountMismatch {
            required: transition_count,
            actual: identities.transitions.len(),
        });
    }
    if state_count != transition_count + 1 {
        return Err(CounterexampleLineageError::CounterexampleShapeMismatch);
    }

    let mut required_facts = BTreeSet::new();
    let mut action_facts = Vec::with_capacity(transition_count);
    for (index, step) in counterexample.steps().iter().enumerate() {
        if step.from() != counterexample.states()[index].id()
            || step.to() != counterexample.states()[index + 1].id()
        {
            return Err(CounterexampleLineageError::CounterexampleShapeMismatch);
        }
        let action = system
            .actions()
            .iter()
            .find(|action| action.id() == step.action())
            .ok_or(CounterexampleLineageError::UnknownAction)?;
        let mut facts = BTreeSet::new();
        facts.extend(
            action
                .precondition()
                .predicates()
                .iter()
                .map(|predicate| predicate.fact()),
        );
        facts.extend(action.effect().insertions().iter().copied());
        facts.extend(action.effect().retractions().iter().copied());
        required_facts.extend(facts.iter().copied());
        action_facts.push(facts);
    }
    let supplied_facts: BTreeSet<_> = identities.facts.iter().map(|entry| entry.fact).collect();
    if supplied_facts != required_facts || supplied_facts.len() != identities.facts.len() {
        return Err(CounterexampleLineageError::FactSetMismatch);
    }
    let required_edges =
        1 + transition_count * 2 + action_facts.iter().map(BTreeSet::len).sum::<usize>();
    if identities.edges.len() != required_edges {
        return Err(CounterexampleLineageError::EdgeCountMismatch {
            required: required_edges,
            actual: identities.edges.len(),
        });
    }

    let mut nodes = Vec::new();
    nodes.push(LineageNode::new(
        identities.violation,
        LineageNodeKind::SafetyViolation(
            counterexample
                .states()
                .last()
                .expect("shape validated")
                .id(),
        ),
    ));
    nodes.extend(
        counterexample
            .states()
            .iter()
            .zip(&identities.states)
            .map(|(state, node)| LineageNode::new(*node, LineageNodeKind::State(state.id()))),
    );
    nodes.extend(
        counterexample
            .steps()
            .iter()
            .zip(&identities.transitions)
            .map(|(step, node)| LineageNode::new(*node, LineageNodeKind::Transition(step.id()))),
    );
    nodes.extend(identities.facts.iter().map(|entry| {
        LineageNode::new(
            entry.node,
            match entry.role {
                CounterexampleFactRole::Source => LineageNodeKind::SourceFact(entry.fact),
                CounterexampleFactRole::Derived => LineageNodeKind::DerivedFact(entry.fact),
            },
        )
    }));

    let fact_node = |fact: FactId| {
        identities
            .facts
            .iter()
            .find(|entry| entry.fact == fact)
            .expect("fact set validated")
            .node
    };
    let mut edge_ids = identities.edges.iter().copied();
    let mut edges = vec![LineageEdge::new(
        edge_ids.next().expect("edge count validated"),
        identities.violation,
        *identities.states.last().expect("state count validated"),
        LineageEdgeKind::DependsOn,
    )];
    for (index, transition_facts) in action_facts.iter().enumerate() {
        let transition_node = identities.transitions[index];
        edges.push(LineageEdge::new(
            edge_ids.next().expect("edge count validated"),
            identities.states[index + 1],
            transition_node,
            LineageEdgeKind::ProducedBy,
        ));
        edges.push(LineageEdge::new(
            edge_ids.next().expect("edge count validated"),
            transition_node,
            identities.states[index],
            LineageEdgeKind::DependsOn,
        ));
        for fact in transition_facts {
            edges.push(LineageEdge::new(
                edge_ids.next().expect("edge count validated"),
                transition_node,
                fact_node(*fact),
                LineageEdgeKind::DependsOn,
            ));
        }
    }
    LineageGraph::admit(nodes, edges).map_err(CounterexampleLineageError::Graph)
}
