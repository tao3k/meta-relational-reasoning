//! Snapshot-bound element selection and declared-dependency revision impact.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    num::NonZeroUsize,
};

use mrr_identity::{EntityId, FactId, GenerationId, QueryId, StateId};
use mrr_relation::{EvidenceCompleteness, Fact};
use mrr_revision::SemanticSnapshot;
use serde::{Deserialize, Serialize};

use crate::evidence::{EvidenceAdmission, admit_fact_evidence, merge_completeness};
use crate::worklist::{self, Worklist};

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
    let pending = worklist::initial_pending(
        &query.roots,
        &contract.required,
        &contract.temporal_receipts,
    );
    let traversal = run_closure(ClosureTraversal {
        elements,
        require_complete: contract.require_complete,
        pending,
        selected: BTreeSet::new(),
        coverage: EvidenceCompleteness::Complete,
        error: None,
    });
    if let Some(error) = traversal.error {
        return Err(error);
    }
    Ok(AgenticAiContextClosure {
        elements: traversal.selected.into_iter().collect(),
        coverage: traversal.coverage,
    })
}

struct ClosureTraversal<'a> {
    elements: &'a BTreeMap<FactId, AgenticAiContextElement>,
    require_complete: bool,
    pending: Vec<FactId>,
    selected: BTreeSet<FactId>,
    coverage: EvidenceCompleteness,
    error: Option<AgenticAiContextError>,
}

fn run_closure(traversal: ClosureTraversal<'_>) -> ClosureTraversal<'_> {
    worklist::run(traversal)
}

impl Worklist for ClosureTraversal<'_> {
    fn advance(&mut self) -> bool {
        advance_closure(self)
    }
}

fn advance_closure(traversal: &mut ClosureTraversal<'_>) -> bool {
    let Some(id) = worklist::pop_identity(&mut traversal.pending) else {
        return false;
    };
    if !traversal.selected.insert(id) {
        return true;
    }
    expand_identity(
        id,
        traversal.elements.get(&id),
        traversal.require_complete,
        &mut traversal.pending,
        &mut traversal.coverage,
        &mut traversal.error,
    )
}

fn expand_identity(
    id: FactId,
    element: Option<&AgenticAiContextElement>,
    require_complete: bool,
    pending: &mut Vec<FactId>,
    coverage: &mut EvidenceCompleteness,
    error: &mut Option<AgenticAiContextError>,
) -> bool {
    let Some(element) = element else {
        *error = Some(AgenticAiContextError::UnknownElement(id));
        return false;
    };
    match admit_fact_evidence(&element.fact, require_complete) {
        EvidenceAdmission::Accepted => {}
        EvidenceAdmission::Invalid => {
            *error = Some(AgenticAiContextError::InvalidatedElement(id));
            return false;
        }
        EvidenceAdmission::Incomplete => {
            *error = Some(AgenticAiContextError::IncompleteEvidence(id));
            return false;
        }
    }
    *coverage = merge_completeness(*coverage, element.fact.context().completeness());
    worklist::append_identities(pending, &element.dependencies);
    true
}

fn reverse_dependency_impact(
    old: &AgenticAiContextState,
    new: &AgenticAiContextState,
    changed: &BTreeSet<FactId>,
) -> BTreeSet<FactId> {
    declared_dependency_impact(&old.elements, &new.elements, changed)
}

fn declared_dependency_impact(
    old: &BTreeMap<FactId, AgenticAiContextElement>,
    new: &BTreeMap<FactId, AgenticAiContextElement>,
    changed: &BTreeSet<FactId>,
) -> BTreeSet<FactId> {
    let reverse = build_reverse_index(old, new);
    let mut pending = Vec::with_capacity(changed.len());
    for id in changed {
        pending.push(*id);
    }
    run_impact(ImpactTraversal {
        reverse,
        invalidated: changed.clone(),
        pending,
    })
    .invalidated
}

fn build_reverse_index(
    old: &BTreeMap<FactId, AgenticAiContextElement>,
    new: &BTreeMap<FactId, AgenticAiContextElement>,
) -> BTreeMap<FactId, BTreeSet<FactId>> {
    let mut reverse = BTreeMap::new();
    add_reverse_dependencies(old, &mut reverse);
    add_reverse_dependencies(new, &mut reverse);
    reverse
}

fn add_reverse_dependencies(
    elements: &BTreeMap<FactId, AgenticAiContextElement>,
    reverse: &mut BTreeMap<FactId, BTreeSet<FactId>>,
) {
    for (id, element) in elements {
        for dependency in element.dependencies.iter() {
            insert_reverse_edge(reverse, *dependency, *id);
        }
    }
}

fn insert_reverse_edge(
    reverse: &mut BTreeMap<FactId, BTreeSet<FactId>>,
    dependency: FactId,
    id: FactId,
) {
    reverse.entry(dependency).or_default().insert(id);
}

struct ImpactTraversal {
    reverse: BTreeMap<FactId, BTreeSet<FactId>>,
    invalidated: BTreeSet<FactId>,
    pending: Vec<FactId>,
}

fn run_impact(traversal: ImpactTraversal) -> ImpactTraversal {
    worklist::run(traversal)
}

impl Worklist for ImpactTraversal {
    fn advance(&mut self) -> bool {
        advance_impact(self)
    }
}

fn advance_impact(traversal: &mut ImpactTraversal) -> bool {
    let Some(id) = worklist::pop_identity(&mut traversal.pending) else {
        return false;
    };
    if let Some(dependents) = traversal.reverse.get(&id) {
        for dependent in dependents {
            if traversal.invalidated.insert(*dependent) {
                traversal.pending.push(*dependent);
            }
        }
    }
    true
}
