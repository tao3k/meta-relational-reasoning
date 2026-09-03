//! Unified, caller-identified lineage graphs and derivation validation.
#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

pub use mrr_identity::{
    DerivationId, FactId, GenerationId, LineageEdgeId, LineageNodeId, QueryOperatorId, RuleId,
    StateId, TransitionId,
};
pub use mrr_relation::Fact;
use mrr_relation::{EvidenceCompleteness, FactProvenance, FactValidity, RelationAuthority};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Derivation {
    id: DerivationId,
    rule: RuleId,
    generation: GenerationId,
    output: Fact,
    support: Vec<FactId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LineageError {
    EmptySupport,
    SelfSupport,
    DuplicateSupport(FactId),
    GenerationMismatch {
        expected: GenerationId,
        actual: GenerationId,
    },
    AuthorityMismatch,
    ProvenanceMismatch,
    IncompleteEvidence,
    InvalidOutput,
}

impl Derivation {
    pub fn new(
        id: DerivationId,
        rule: RuleId,
        generation: GenerationId,
        output: Fact,
        support: Vec<FactId>,
    ) -> Result<Self, LineageError> {
        if support.is_empty() {
            return Err(LineageError::EmptySupport);
        }
        if support.contains(&output.id()) {
            return Err(LineageError::SelfSupport);
        }
        let mut unique_support = BTreeSet::new();
        if let Some(duplicate) = support
            .iter()
            .copied()
            .find(|fact| !unique_support.insert(*fact))
        {
            return Err(LineageError::DuplicateSupport(duplicate));
        }
        let actual_generation = output.context().generation();
        if actual_generation != generation {
            return Err(LineageError::GenerationMismatch {
                expected: generation,
                actual: actual_generation,
            });
        }
        if output.context().authority() != RelationAuthority::Rule(rule) {
            return Err(LineageError::AuthorityMismatch);
        }
        if output.context().provenance() != FactProvenance::Derivation(id) {
            return Err(LineageError::ProvenanceMismatch);
        }
        if output.context().completeness() != EvidenceCompleteness::Complete {
            return Err(LineageError::IncompleteEvidence);
        }
        if output.context().validity() != FactValidity::Valid {
            return Err(LineageError::InvalidOutput);
        }
        Ok(Self {
            id,
            rule,
            generation,
            output,
            support,
        })
    }

    #[must_use]
    pub const fn id(&self) -> DerivationId {
        self.id
    }

    #[must_use]
    pub const fn rule(&self) -> RuleId {
        self.rule
    }

    #[must_use]
    pub const fn generation(&self) -> GenerationId {
        self.generation
    }

    #[must_use]
    pub fn output(&self) -> &Fact {
        &self.output
    }

    #[must_use]
    pub fn support(&self) -> &[FactId] {
        &self.support
    }
}

/// Semantic role of one node in the unified explanation graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineageNodeKind {
    SourceFact(FactId),
    QueryOperator(QueryOperatorId),
    RuleApplication(DerivationId),
    DerivedFact(FactId),
    Transition(TransitionId),
    State(StateId),
    SafetyViolation(StateId),
    Projection(QueryOperatorId),
    Result(FactId),
}

/// Caller-identified lineage node. The lineage crate validates but never allocates its ID.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineageNode {
    id: LineageNodeId,
    kind: LineageNodeKind,
}

impl LineageNode {
    #[must_use]
    pub const fn new(id: LineageNodeId, kind: LineageNodeKind) -> Self {
        Self { id, kind }
    }

    #[must_use]
    pub const fn id(&self) -> LineageNodeId {
        self.id
    }

    #[must_use]
    pub const fn kind(&self) -> LineageNodeKind {
        self.kind
    }
}

/// Domain-neutral causal relation between lineage nodes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineageEdgeKind {
    DerivedFrom,
    SelectedBy,
    ProducedBy,
    DependsOn,
    InvalidatedBy,
}

/// Caller-identified directed lineage edge, oriented from conclusion to evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineageEdge {
    id: LineageEdgeId,
    from: LineageNodeId,
    to: LineageNodeId,
    kind: LineageEdgeKind,
}

impl LineageEdge {
    #[must_use]
    pub const fn new(
        id: LineageEdgeId,
        from: LineageNodeId,
        to: LineageNodeId,
        kind: LineageEdgeKind,
    ) -> Self {
        Self { id, from, to, kind }
    }

    #[must_use]
    pub const fn id(&self) -> LineageEdgeId {
        self.id
    }

    #[must_use]
    pub const fn from(&self) -> LineageNodeId {
        self.from
    }

    #[must_use]
    pub const fn to(&self) -> LineageNodeId {
        self.to
    }

    #[must_use]
    pub const fn kind(&self) -> LineageEdgeKind {
        self.kind
    }
}

/// Fail-closed graph-admission errors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LineageGraphError {
    DuplicateNode(LineageNodeId),
    DuplicateEdge(LineageEdgeId),
    MissingEndpoint(LineageNodeId),
    SelfEdge(LineageEdgeId),
    Cycle,
}

/// Immutable, finite, acyclic explanation graph shared by WHY and IMPACT projections.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineageGraph {
    nodes: Vec<LineageNode>,
    edges: Vec<LineageEdge>,
}

impl LineageGraph {
    pub fn admit(
        mut nodes: Vec<LineageNode>,
        mut edges: Vec<LineageEdge>,
    ) -> Result<Self, LineageGraphError> {
        nodes.sort_by_key(LineageNode::id);
        edges.sort_by_key(LineageEdge::id);
        for pair in nodes.windows(2) {
            if pair[0].id() == pair[1].id() {
                return Err(LineageGraphError::DuplicateNode(pair[0].id()));
            }
        }
        for pair in edges.windows(2) {
            if pair[0].id() == pair[1].id() {
                return Err(LineageGraphError::DuplicateEdge(pair[0].id()));
            }
        }
        let node_ids: BTreeSet<_> = nodes.iter().map(LineageNode::id).collect();
        for edge in &edges {
            if edge.from() == edge.to() {
                return Err(LineageGraphError::SelfEdge(edge.id()));
            }
            for endpoint in [edge.from(), edge.to()] {
                if !node_ids.contains(&endpoint) {
                    return Err(LineageGraphError::MissingEndpoint(endpoint));
                }
            }
        }
        if contains_cycle(&node_ids, &edges) {
            return Err(LineageGraphError::Cycle);
        }
        Ok(Self { nodes, edges })
    }

    #[must_use]
    pub fn nodes(&self) -> &[LineageNode] {
        &self.nodes
    }

    #[must_use]
    pub fn edges(&self) -> &[LineageEdge] {
        &self.edges
    }
}

/// Fail-closed WHY projection errors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WhyError {
    UnknownResult(LineageNodeId),
}

/// Immutable explanation subgraph rooted at the requested result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplanationGraph {
    root: LineageNodeId,
    graph: LineageGraph,
}

/// Reverse invalidation projection for one fact-bearing lineage node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImpactGraph {
    fact: FactId,
    source: LineageNodeId,
    directly_invalidated: Vec<LineageNodeId>,
    transitively_invalidated: Vec<LineageNodeId>,
    unaffected: Vec<LineageNodeId>,
}

impl ImpactGraph {
    #[must_use]
    pub const fn fact(&self) -> FactId {
        self.fact
    }

    #[must_use]
    pub const fn source(&self) -> LineageNodeId {
        self.source
    }

    #[must_use]
    pub fn directly_invalidated(&self) -> &[LineageNodeId] {
        &self.directly_invalidated
    }

    #[must_use]
    pub fn transitively_invalidated(&self) -> &[LineageNodeId] {
        &self.transitively_invalidated
    }

    #[must_use]
    pub fn unaffected(&self) -> &[LineageNodeId] {
        &self.unaffected
    }
}

/// Fail-closed IMPACT projection errors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImpactError {
    UnknownFact(FactId),
    AmbiguousFact(FactId),
}

impl ExplanationGraph {
    #[must_use]
    pub const fn root(&self) -> LineageNodeId {
        self.root
    }

    #[must_use]
    pub const fn graph(&self) -> &LineageGraph {
        &self.graph
    }
}

/// Returns every admitted explanation path reachable from `result`.
pub fn why(graph: &LineageGraph, result: LineageNodeId) -> Result<ExplanationGraph, WhyError> {
    project_explanation(graph, result, false)
}

/// Returns one deterministic path only when the caller explicitly requests one witness.
pub fn why_one_witness(
    graph: &LineageGraph,
    result: LineageNodeId,
) -> Result<ExplanationGraph, WhyError> {
    project_explanation(graph, result, true)
}

/// Computes direct, transitive, and unaffected lineage nodes for one changed fact.
pub fn impact(graph: &LineageGraph, fact: FactId) -> Result<ImpactGraph, ImpactError> {
    let matching: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node.kind(),
                LineageNodeKind::SourceFact(candidate)
                    | LineageNodeKind::DerivedFact(candidate)
                    if candidate == fact
            )
        })
        .map(LineageNode::id)
        .collect();
    let [source] = matching.as_slice() else {
        return Err(if matching.is_empty() {
            ImpactError::UnknownFact(fact)
        } else {
            ImpactError::AmbiguousFact(fact)
        });
    };

    let mut predecessors: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for edge in &graph.edges {
        predecessors.entry(edge.to()).or_default().push(edge.from());
    }
    let mut direct = predecessors.get(source).cloned().unwrap_or_default();
    direct.sort_unstable();
    direct.dedup();
    let direct_set: BTreeSet<_> = direct.iter().copied().collect();
    let mut affected = direct_set.clone();
    let mut pending = direct.clone();
    while let Some(node) = pending.pop() {
        for predecessor in predecessors.get(&node).into_iter().flatten() {
            if affected.insert(*predecessor) {
                pending.push(*predecessor);
            }
        }
    }
    let transitively_invalidated = affected
        .iter()
        .copied()
        .filter(|node| !direct_set.contains(node))
        .collect();
    let unaffected = graph
        .nodes
        .iter()
        .map(LineageNode::id)
        .filter(|node| *node != *source && !affected.contains(node))
        .collect();
    Ok(ImpactGraph {
        fact,
        source: *source,
        directly_invalidated: direct,
        transitively_invalidated,
        unaffected,
    })
}

fn project_explanation(
    graph: &LineageGraph,
    result: LineageNodeId,
    one_witness: bool,
) -> Result<ExplanationGraph, WhyError> {
    if !graph.nodes.iter().any(|node| node.id() == result) {
        return Err(WhyError::UnknownResult(result));
    }
    let mut selected_nodes = BTreeSet::from([result]);
    let mut selected_edges = BTreeSet::new();
    let mut pending = vec![result];
    let mut outgoing: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for edge in &graph.edges {
        outgoing.entry(edge.from()).or_default().push(edge);
    }
    while let Some(node) = pending.pop() {
        for edge in outgoing
            .get(&node)
            .into_iter()
            .flatten()
            .take(if one_witness { 1 } else { usize::MAX })
        {
            selected_edges.insert(edge.id());
            if selected_nodes.insert(edge.to()) {
                pending.push(edge.to());
            }
        }
    }
    let nodes = graph
        .nodes
        .iter()
        .copied()
        .filter(|node| selected_nodes.contains(&node.id()))
        .collect();
    let edges = graph
        .edges
        .iter()
        .copied()
        .filter(|edge| selected_edges.contains(&edge.id()))
        .collect();
    let graph = LineageGraph::admit(nodes, edges).expect("an admitted graph subset is admitted");
    Ok(ExplanationGraph {
        root: result,
        graph,
    })
}

fn contains_cycle(nodes: &BTreeSet<LineageNodeId>, edges: &[LineageEdge]) -> bool {
    let mut indegree: BTreeMap<_, usize> = nodes.iter().map(|node| (*node, 0)).collect();
    let mut outgoing: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for edge in edges {
        *indegree.get_mut(&edge.to()).expect("endpoint validated") += 1;
        outgoing.entry(edge.from()).or_default().push(edge.to());
    }
    let mut ready: Vec<_> = indegree
        .iter()
        .filter_map(|(node, degree)| (*degree == 0).then_some(*node))
        .collect();
    let mut visited = 0;
    while let Some(node) = ready.pop() {
        visited += 1;
        for target in outgoing.get(&node).into_iter().flatten() {
            let degree = indegree.get_mut(target).expect("endpoint validated");
            *degree -= 1;
            if *degree == 0 {
                ready.push(*target);
            }
        }
    }
    visited != nodes.len()
}
