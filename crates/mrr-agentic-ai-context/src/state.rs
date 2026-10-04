//! Snapshot-bound element selection and declared-dependency revision impact.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    num::NonZeroUsize,
};

use mrr_identity::{EntityId, FactId, GenerationId, QueryId, StateId};
use mrr_relation::{EvidenceCompleteness, Fact, FactValidity};
use mrr_revision::SemanticSnapshot;
use serde::{Deserialize, Serialize};

/// Exact source fact and all declared fact-level dependencies of this element.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgenticAiContextElement {
    fact: Fact,
    dependencies: Vec<FactId>,
}

impl AgenticAiContextElement {
    #[must_use]
    pub fn new(fact: Fact, dependencies: Vec<FactId>) -> Self {
        Self {
            fact,
            dependencies: canonical_ids(dependencies),
        }
    }

    #[must_use]
    pub const fn fact(&self) -> &Fact {
        &self.fact
    }

    #[must_use]
    pub fn dependencies(&self) -> &[FactId] {
        &self.dependencies
    }
}

/// Explicit selection tied to an already admitted MRR query identity.
/// This does not execute or reinterpret the query language.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgenticAiContextQuery {
    id: QueryId,
    roots: Vec<FactId>,
}

impl AgenticAiContextQuery {
    #[must_use]
    pub fn new(id: QueryId, roots: Vec<FactId>) -> Self {
        Self {
            id,
            roots: canonical_ids(roots),
        }
    }

    #[must_use]
    pub const fn id(&self) -> QueryId {
        self.id
    }

    #[must_use]
    pub fn roots(&self) -> &[FactId] {
        &self.roots
    }
}

/// Effective scope and required evidence supplied by the contract owner.
/// Temporal receipts are existing fact references; this defines no clock or
/// temporal ontology and grants no authorization by itself.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgenticAiContextContract {
    pub actor: EntityId,
    pub task: StateId,
    pub policy_digest: [u8; 32],
    pub required: Vec<FactId>,
    pub temporal_receipts: Vec<FactId>,
    pub require_complete: bool,
}

/// Bounds for source elements, declared edges and produced presentation bytes.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgenticAiContextLimits {
    pub max_elements: NonZeroUsize,
    pub max_dependency_edges: NonZeroUsize,
    pub max_rendered_bytes: NonZeroUsize,
}

/// Typed rejection before a context state, layout or eligibility is published.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgenticAiContextError {
    ElementBudget,
    DependencyBudget,
    RenderedByteBudget,
    DuplicateElement(FactId),
    UnknownElement(FactId),
    GenerationMismatch {
        fact: FactId,
        expected: GenerationId,
        actual: GenerationId,
    },
    InvalidatedElement(FactId),
    IncompleteEvidence(FactId),
    InvalidPrecedence,
    RenderedElementMismatch,
    ComputationalIdentityMismatch,
    EmptyComputationalIdentity,
    CompositionBudget,
    CompositionGraphMismatch,
    CompositionCycle,
    CompositionOrderViolation,
    InvalidCompositionProducer,
}

impl fmt::Display for AgenticAiContextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AgenticAiContextError {}

/// Named inputs for structural validation of one snapshot-bound context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgenticAiContextStateInput {
    pub snapshot: SemanticSnapshot,
    pub query: AgenticAiContextQuery,
    pub contract: AgenticAiContextContract,
    pub elements: Vec<AgenticAiContextElement>,
    pub limits: AgenticAiContextLimits,
}

/// Structurally validated input. It is not a source/query admission receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgenticAiContextState {
    snapshot: SemanticSnapshot,
    query: AgenticAiContextQuery,
    contract: AgenticAiContextContract,
    elements: BTreeMap<FactId, AgenticAiContextElement>,
    limits: AgenticAiContextLimits,
    closure: AgenticAiContextClosure,
}

impl AgenticAiContextState {
    /// Validate source shape and compute the complete required closure once.
    pub fn validate(input: AgenticAiContextStateInput) -> Result<Self, AgenticAiContextError> {
        let AgenticAiContextStateInput {
            snapshot,
            query,
            mut contract,
            elements,
            limits,
        } = input;
        let index = index_elements(snapshot.generation(), elements, limits)?;
        validate_dependency_refs(&index)?;
        contract.required = canonical_ids(contract.required);
        contract.temporal_receipts = canonical_ids(contract.temporal_receipts);
        let closure = compute_required_closure(&query, &contract, &index)?;
        Ok(Self {
            snapshot,
            query,
            contract,
            elements: index,
            limits,
            closure,
        })
    }

    #[must_use]
    pub const fn snapshot(&self) -> &SemanticSnapshot {
        &self.snapshot
    }
    #[must_use]
    pub const fn query(&self) -> &AgenticAiContextQuery {
        &self.query
    }
    #[must_use]
    pub const fn contract(&self) -> &AgenticAiContextContract {
        &self.contract
    }
    #[must_use]
    pub const fn limits(&self) -> AgenticAiContextLimits {
        self.limits
    }
    #[must_use]
    pub fn elements(&self) -> &BTreeMap<FactId, AgenticAiContextElement> {
        &self.elements
    }

    /// Return the finite required closure computed during validation.
    #[must_use]
    pub const fn required_closure(&self) -> &AgenticAiContextClosure {
        &self.closure
    }
}

/// Selected fact identities and their weakest admitted evidence completeness.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgenticAiContextClosure {
    elements: Vec<FactId>,
    coverage: EvidenceCompleteness,
}

impl AgenticAiContextClosure {
    #[must_use]
    pub fn elements(&self) -> &[FactId] {
        &self.elements
    }
    #[must_use]
    pub const fn coverage(&self) -> EvidenceCompleteness {
        self.coverage
    }
}

/// Exact old/new differences and the transitive reverse-dependency footprint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgenticAiContextRevision {
    changed: Vec<FactId>,
    invalidated: Vec<FactId>,
    reusable: SemanticReuseCertificate,
}

/// Equality of actual selected semantic elements and their declared dependency
/// footprint. This certifies neither rendered bytes nor token IDs nor GPU state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticReuseCertificate {
    elements: Vec<FactId>,
}

impl SemanticReuseCertificate {
    #[must_use]
    pub fn elements(&self) -> &[FactId] {
        &self.elements
    }
}

impl AgenticAiContextRevision {
    #[must_use]
    pub fn between(old: &AgenticAiContextState, new: &AgenticAiContextState) -> Self {
        Self::compare(old, new, false)
    }

    /// The facade uses this when external query/catalog bindings have changed.
    #[must_use]
    pub fn invalidating_all(old: &AgenticAiContextState, new: &AgenticAiContextState) -> Self {
        Self::compare(old, new, true)
    }

    fn compare(
        old: &AgenticAiContextState,
        new: &AgenticAiContextState,
        force_global_change: bool,
    ) -> Self {
        let all: BTreeSet<_> = old
            .elements
            .keys()
            .chain(new.elements.keys())
            .copied()
            .collect();
        // A changed global binding conservatively invalidates all semantic reuse.
        let global_change = force_global_change
            || old.snapshot != new.snapshot
            || old.query != new.query
            || old.contract != new.contract;
        let changed: BTreeSet<_> = all
            .iter()
            .copied()
            .filter(|id| global_change || old.elements.get(id) != new.elements.get(id))
            .collect();
        let invalidated = reverse_dependency_impact(old, new, &changed);
        let old_selected: BTreeSet<_> = old.closure.elements.iter().copied().collect();
        let new_selected: BTreeSet<_> = new.closure.elements.iter().copied().collect();
        let reusable = SemanticReuseCertificate {
            elements: old_selected
                .intersection(&new_selected)
                .copied()
                .filter(|id| !invalidated.contains(id))
                .collect(),
        };
        Self {
            changed: changed.into_iter().collect(),
            invalidated: invalidated.into_iter().collect(),
            reusable,
        }
    }

    #[must_use]
    pub fn changed(&self) -> &[FactId] {
        &self.changed
    }
    #[must_use]
    pub fn invalidated(&self) -> &[FactId] {
        &self.invalidated
    }
    #[must_use]
    pub const fn reusable(&self) -> &SemanticReuseCertificate {
        &self.reusable
    }
}

fn canonical_ids(ids: Vec<FactId>) -> Vec<FactId> {
    ids.into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn index_elements(
    generation: GenerationId,
    elements: Vec<AgenticAiContextElement>,
    limits: AgenticAiContextLimits,
) -> Result<BTreeMap<FactId, AgenticAiContextElement>, AgenticAiContextError> {
    if elements.len() > limits.max_elements.get() {
        return Err(AgenticAiContextError::ElementBudget);
    }
    let mut index = BTreeMap::new();
    let mut edges = 0_usize;
    for element in elements {
        let id = element.fact.id();
        let actual = element.fact.context().generation();
        if actual != generation {
            return Err(AgenticAiContextError::GenerationMismatch {
                fact: id,
                expected: generation,
                actual,
            });
        }
        edges = edges
            .checked_add(element.dependencies.len())
            .ok_or(AgenticAiContextError::DependencyBudget)?;
        if edges > limits.max_dependency_edges.get() {
            return Err(AgenticAiContextError::DependencyBudget);
        }
        if index.insert(id, element).is_some() {
            return Err(AgenticAiContextError::DuplicateElement(id));
        }
    }
    Ok(index)
}

fn validate_dependency_refs(
    elements: &BTreeMap<FactId, AgenticAiContextElement>,
) -> Result<(), AgenticAiContextError> {
    for element in elements.values() {
        for dependency in &element.dependencies {
            if !elements.contains_key(dependency) {
                return Err(AgenticAiContextError::UnknownElement(*dependency));
            }
        }
    }
    Ok(())
}

fn compute_required_closure(
    query: &AgenticAiContextQuery,
    contract: &AgenticAiContextContract,
    elements: &BTreeMap<FactId, AgenticAiContextElement>,
) -> Result<AgenticAiContextClosure, AgenticAiContextError> {
    let mut selected = BTreeSet::new();
    let mut pending: Vec<_> = query
        .roots
        .iter()
        .chain(&contract.required)
        .chain(&contract.temporal_receipts)
        .copied()
        .collect();
    let mut coverage = EvidenceCompleteness::Complete;
    while let Some(id) = pending.pop() {
        if !selected.insert(id) {
            continue;
        }
        let element = elements
            .get(&id)
            .ok_or(AgenticAiContextError::UnknownElement(id))?;
        let context = element.fact.context();
        if context.validity() != FactValidity::Valid {
            return Err(AgenticAiContextError::InvalidatedElement(id));
        }
        if contract.require_complete && context.completeness() != EvidenceCompleteness::Complete {
            return Err(AgenticAiContextError::IncompleteEvidence(id));
        }
        coverage = match (coverage, context.completeness()) {
            (EvidenceCompleteness::Unknown, _) | (_, EvidenceCompleteness::Unknown) => {
                EvidenceCompleteness::Unknown
            }
            (EvidenceCompleteness::Partial, _) | (_, EvidenceCompleteness::Partial) => {
                EvidenceCompleteness::Partial
            }
            _ => EvidenceCompleteness::Complete,
        };
        pending.extend(&element.dependencies);
    }
    Ok(AgenticAiContextClosure {
        elements: selected.into_iter().collect(),
        coverage,
    })
}

fn reverse_dependency_impact(
    old: &AgenticAiContextState,
    new: &AgenticAiContextState,
    changed: &BTreeSet<FactId>,
) -> BTreeSet<FactId> {
    let mut reverse: BTreeMap<FactId, BTreeSet<FactId>> = BTreeMap::new();
    for state in [old, new] {
        for (id, element) in &state.elements {
            for dependency in &element.dependencies {
                reverse.entry(*dependency).or_default().insert(*id);
            }
        }
    }
    let mut invalidated = changed.clone();
    let mut pending: Vec<_> = changed.iter().copied().collect();
    while let Some(id) = pending.pop() {
        if let Some(dependents) = reverse.get(&id) {
            for dependent in dependents {
                if invalidated.insert(*dependent) {
                    pending.push(*dependent);
                }
            }
        }
    }
    invalidated
}
