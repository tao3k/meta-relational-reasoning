//! Bounded composition inputs and external producer contracts, separate from D.

use crate::{AgenticAiContextComposition, AgenticAiContextError, AgenticAiContextState};
use mrr_identity::FactId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// One selected fact's ordered inheritance declarations, including C4 suffix metadata.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgenticAiContextCompositionNode {
    pub element: FactId,
    pub parent_orders: Vec<Vec<FactId>>,
    pub suffix: bool,
}

/// Single-root composition candidate. Only an empty selection has no root.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgenticAiContextCompositionGraphInput {
    pub root: Option<FactId>,
    pub nodes: Vec<AgenticAiContextCompositionNode>,
}

/// Exact selected domain, known references, root reachability and acyclic inheritance.
/// This validation does not compute C4 or validate inherited suffix compatibility.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgenticAiContextCompositionGraph {
    input: AgenticAiContextCompositionGraphInput,
}

impl AgenticAiContextCompositionGraph {
    pub fn validate(
        state: &AgenticAiContextState,
        mut input: AgenticAiContextCompositionGraphInput,
    ) -> Result<Self, AgenticAiContextError> {
        let ceiling = state.limits();
        if input.nodes.len() > ceiling.max_elements.get() {
            return Err(AgenticAiContextError::CompositionBudget);
        }
        let mut orders = 0_usize;
        let mut references = 0_usize;
        for node in &input.nodes {
            orders = orders
                .checked_add(node.parent_orders.len())
                .ok_or(AgenticAiContextError::CompositionBudget)?;
            for order in &node.parent_orders {
                references = references
                    .checked_add(order.len())
                    .ok_or(AgenticAiContextError::CompositionBudget)?;
            }
            if orders > ceiling.max_dependency_edges.get()
                || references > ceiling.max_dependency_edges.get()
            {
                return Err(AgenticAiContextError::CompositionBudget);
            }
        }
        input.nodes.sort_by_key(|node| node.element);
        let ids: Vec<_> = input.nodes.iter().map(|node| node.element).collect();
        if ids != state.required_closure().elements() {
            return Err(AgenticAiContextError::CompositionGraphMismatch);
        }
        let parents = parent_index(&input)?;
        if ids.is_empty() {
            if input.root.is_some() {
                return Err(AgenticAiContextError::CompositionGraphMismatch);
            }
        } else {
            let root = input
                .root
                .filter(|root| parents.contains_key(root))
                .ok_or(AgenticAiContextError::CompositionGraphMismatch)?;
            check_reachable(root, &parents)?;
            check_acyclic(&parents)?;
        }
        Ok(Self { input })
    }

    #[must_use]
    pub const fn input(&self) -> &AgenticAiContextCompositionGraphInput {
        &self.input
    }

    /// Check producer output against membership, ancestry and local order constraints.
    /// Inherited C4 merge and suffix semantics remain producer-owned obligations.
    pub fn check_result(
        &self,
        state: &AgenticAiContextState,
        precedence: Vec<FactId>,
    ) -> Result<AgenticAiContextComposition, AgenticAiContextError> {
        let composition = AgenticAiContextComposition::from_precedence(state, precedence)?;
        if composition.precedence().first().copied() != self.input.root {
            return Err(AgenticAiContextError::CompositionOrderViolation);
        }
        let positions: BTreeMap<_, _> = composition
            .precedence()
            .iter()
            .copied()
            .enumerate()
            .map(|(position, id)| (id, position))
            .collect();
        if self
            .input
            .nodes
            .iter()
            .map(|node| node.element)
            .collect::<Vec<_>>()
            != state.required_closure().elements()
        {
            return Err(AgenticAiContextError::CompositionGraphMismatch);
        }
        for node in &self.input.nodes {
            for order in &node.parent_orders {
                let mut previous = positions[&node.element];
                for parent in order {
                    let next = positions[parent];
                    if previous >= next {
                        return Err(AgenticAiContextError::CompositionOrderViolation);
                    }
                    previous = next;
                }
            }
        }
        Ok(composition)
    }
}

/// Versioned producer declaration. Matching labels do not authenticate its code.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgenticAiContextCompositionProducer {
    pub owner: String,
    pub algorithm: String,
    pub implementation_version: String,
}

impl AgenticAiContextCompositionProducer {
    pub fn validate(&self) -> Result<(), AgenticAiContextError> {
        if [&self.owner, &self.algorithm, &self.implementation_version]
            .iter()
            .any(|label| label.is_empty() || label.trim() != label.as_str() || label.len() > 256)
        {
            return Err(AgenticAiContextError::InvalidCompositionProducer);
        }
        Ok(())
    }
}

/// External composition owner, invoked only with a bounded validated graph.
pub trait AgenticAiContextComposer {
    type Error: std::error::Error + 'static;
    fn identity(&self) -> &AgenticAiContextCompositionProducer;
    fn compose(&self, graph: &AgenticAiContextCompositionGraph)
    -> Result<Vec<FactId>, Self::Error>;
}

fn parent_index(
    input: &AgenticAiContextCompositionGraphInput,
) -> Result<BTreeMap<FactId, BTreeSet<FactId>>, AgenticAiContextError> {
    let known: BTreeSet<_> = input.nodes.iter().map(|node| node.element).collect();
    let mut index = BTreeMap::new();
    for node in &input.nodes {
        let mut parents = BTreeSet::new();
        for order in &node.parent_orders {
            for parent in order {
                if !known.contains(parent) {
                    return Err(AgenticAiContextError::UnknownElement(*parent));
                }
                parents.insert(*parent);
            }
        }
        index.insert(node.element, parents);
    }
    Ok(index)
}

fn check_reachable(
    root: FactId,
    parents: &BTreeMap<FactId, BTreeSet<FactId>>,
) -> Result<(), AgenticAiContextError> {
    let mut seen = BTreeSet::new();
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if seen.insert(node) {
            pending.extend(&parents[&node]);
        }
    }
    if seen.len() != parents.len() {
        return Err(AgenticAiContextError::CompositionGraphMismatch);
    }
    Ok(())
}

fn check_acyclic(
    parents: &BTreeMap<FactId, BTreeSet<FactId>>,
) -> Result<(), AgenticAiContextError> {
    let mut remaining: BTreeMap<_, _> = parents
        .iter()
        .map(|(id, edges)| (*id, edges.len()))
        .collect();
    let mut children: BTreeMap<FactId, Vec<FactId>> = BTreeMap::new();
    for (child, edges) in parents {
        for parent in edges {
            children.entry(*parent).or_default().push(*child);
        }
    }
    let mut ready: Vec<_> = remaining
        .iter()
        .filter_map(|(id, count)| (*count == 0).then_some(*id))
        .collect();
    let mut completed = 0;
    while let Some(node) = ready.pop() {
        completed += 1;
        for child in children.get(&node).into_iter().flatten() {
            if let Some(count) = remaining.get_mut(child) {
                *count -= 1;
                if *count == 0 {
                    ready.push(*child);
                }
            }
        }
    }
    if completed != parents.len() {
        return Err(AgenticAiContextError::CompositionCycle);
    }
    Ok(())
}
