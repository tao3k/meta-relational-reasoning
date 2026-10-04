//! Defines typed Search factors, observations and native inference receipts.

use std::fmt;
use std::num::NonZeroUsize;

use mrr_identity::{FactId, GenerationId, IdentityError, QueryOperatorId};
use mrr_lineage::{ImpactError, ImpactGraph, LineageGraph, LineageGraphError, impact};

/// Backend-neutral responsibility of one consumer-defined Search factor.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SearchFactorRole {
    Acquisition,
    Refinement,
    Reasoning,
    Projection,
}

/// One factor registered by a downstream consumer.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SearchFactor {
    pub(super) id: QueryOperatorId,
    pub(super) role: SearchFactorRole,
}

impl SearchFactor {
    #[must_use]
    pub const fn new(id: QueryOperatorId, role: SearchFactorRole) -> Self {
        Self { id, role }
    }

    /// Derives the same typed identity from a Scheme Search factor's canonical input.
    pub fn from_canonical_input(
        canonical_input: &str,
        role: SearchFactorRole,
    ) -> Result<Self, IdentityError> {
        QueryOperatorId::from_canonical_bytes(canonical_input).map(|id| Self { id, role })
    }

    #[must_use]
    pub const fn id(&self) -> QueryOperatorId {
        self.id
    }

    #[must_use]
    pub const fn role(&self) -> SearchFactorRole {
        self.role
    }
}

/// Directed influence edge from an upstream factor to a downstream factor.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SearchFactorEdge {
    pub(super) from: QueryOperatorId,
    pub(super) to: QueryOperatorId,
}

impl SearchFactorEdge {
    #[must_use]
    pub const fn new(from: QueryOperatorId, to: QueryOperatorId) -> Self {
        Self { from, to }
    }

    #[must_use]
    pub const fn from(&self) -> QueryOperatorId {
        self.from
    }

    #[must_use]
    pub const fn to(&self) -> QueryOperatorId {
        self.to
    }
}

/// One immutable factor observation supplied by the consumer runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchObservation {
    pub(super) id: FactId,
    pub(super) candidate: FactId,
    pub(super) factor: QueryOperatorId,
    pub(super) generation: GenerationId,
    pub(super) logical_position: u64,
    pub(super) causal_parents: Vec<FactId>,
}

impl SearchObservation {
    #[must_use]
    pub fn new(
        id: FactId,
        candidate: FactId,
        factor: QueryOperatorId,
        generation: GenerationId,
        logical_position: u64,
        causal_parents: Vec<FactId>,
    ) -> Self {
        Self {
            id,
            candidate,
            factor,
            generation,
            logical_position,
            causal_parents,
        }
    }

    #[must_use]
    pub const fn id(&self) -> FactId {
        self.id
    }

    #[must_use]
    pub const fn candidate(&self) -> FactId {
        self.candidate
    }

    #[must_use]
    pub const fn factor(&self) -> QueryOperatorId {
        self.factor
    }

    #[must_use]
    pub const fn generation(&self) -> GenerationId {
        self.generation
    }

    #[must_use]
    pub const fn logical_position(&self) -> u64 {
        self.logical_position
    }

    #[must_use]
    pub fn causal_parents(&self) -> &[FactId] {
        &self.causal_parents
    }
}

/// Hard pre-execution and publication budgets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SearchFrameworkLimits {
    pub(super) max_factors: NonZeroUsize,
    pub(super) max_edges: NonZeroUsize,
    pub(super) max_observations: NonZeroUsize,
    pub(super) max_influences: NonZeroUsize,
    pub(super) max_results: NonZeroUsize,
}

impl SearchFrameworkLimits {
    #[must_use]
    pub const fn new(
        max_factors: NonZeroUsize,
        max_edges: NonZeroUsize,
        max_observations: NonZeroUsize,
        max_influences: NonZeroUsize,
        max_results: NonZeroUsize,
    ) -> Self {
        Self {
            max_factors,
            max_edges,
            max_observations,
            max_influences,
            max_results,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchFrameworkStatus {
    Complete,
    OutputTruncated,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SearchFrameworkError {
    #[cfg(feature = "native-inference")]
    NativeInference(mrr_gerbil::FiniteInferenceError),
    EmptyFactors,
    DuplicateFactor(QueryOperatorId),
    DuplicateEdge(SearchFactorEdge),
    UnknownFactor(QueryOperatorId),
    SelfEdge(SearchFactorEdge),
    FactorCycle,
    DuplicateObservation(FactId),
    ObservationGenerationMismatch {
        observation: FactId,
        expected: GenerationId,
        actual: GenerationId,
    },
    DuplicateCausalParent {
        observation: FactId,
        parent: FactId,
    },
    MissingCausalParent {
        observation: FactId,
        parent: FactId,
    },
    TemporalOrderViolation {
        observation: FactId,
        parent: FactId,
    },
    CausalFactorEdgeMissing {
        observation: FactId,
        parent: FactId,
    },
    NonAcquisitionRoot(FactId),
    BudgetExceeded {
        resource: &'static str,
        required: usize,
        limit: usize,
    },
    Lineage(LineageGraphError),
    InternalPathMismatch,
}

impl fmt::Display for SearchFrameworkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SearchFrameworkError {}

/// One candidate's shortest causal influence trajectory to a factor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchInfluence {
    pub(super) candidate: FactId,
    pub(super) factor: QueryOperatorId,
    pub(super) support_event: FactId,
    pub(super) factor_path: Vec<QueryOperatorId>,
    pub(super) result_fact: FactId,
}

impl SearchInfluence {
    #[must_use]
    pub const fn candidate(&self) -> FactId {
        self.candidate
    }

    #[must_use]
    pub const fn factor(&self) -> QueryOperatorId {
        self.factor
    }

    #[must_use]
    pub const fn support_event(&self) -> FactId {
        self.support_event
    }

    #[must_use]
    pub fn factor_path(&self) -> &[QueryOperatorId] {
        &self.factor_path
    }

    #[must_use]
    pub const fn result_fact(&self) -> FactId {
        self.result_fact
    }
}

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SearchReasoningDigest(pub(super) [u8; 32]);

impl fmt::Debug for SearchReasoningDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl fmt::Display for SearchReasoningDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl SearchReasoningDigest {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Complete bounded reasoning output plus a reusable MRR lineage graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchFrameworkReceipt {
    pub(super) generation: GenerationId,
    pub(super) status: SearchFrameworkStatus,
    pub(super) total_influence_count: usize,
    pub(super) influences: Vec<SearchInfluence>,
    pub(super) lineage: LineageGraph,
    pub(super) digest: SearchReasoningDigest,
}

impl SearchFrameworkReceipt {
    #[must_use]
    pub const fn generation(&self) -> GenerationId {
        self.generation
    }

    #[must_use]
    pub const fn status(&self) -> SearchFrameworkStatus {
        self.status
    }

    #[must_use]
    pub const fn total_influence_count(&self) -> usize {
        self.total_influence_count
    }

    #[must_use]
    pub fn influences(&self) -> &[SearchInfluence] {
        &self.influences
    }

    #[must_use]
    pub const fn lineage(&self) -> &LineageGraph {
        &self.lineage
    }

    #[must_use]
    pub const fn digest(&self) -> SearchReasoningDigest {
        self.digest
    }

    pub fn impact(&self, observation: FactId) -> Result<ImpactGraph, ImpactError> {
        impact(&self.lineage, observation)
    }
}
