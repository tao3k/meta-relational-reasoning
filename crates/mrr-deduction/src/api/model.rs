//! Admits a narrow closure contract and delegates inference to Scheme ASCENT.

use std::fmt;
use std::num::NonZeroUsize;

use mrr_identity::{FactId, GenerationId, RelationId, RuleId, RulePackId};
use mrr_relation::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Binds the source/derived relations and the only two rule identities this adapter executes.
pub struct ClosureConfig {
    pub(super) source_relation: RelationId,
    pub(super) derived_relation: RelationId,
    pub(super) rule_pack: RulePackId,
    pub(super) base_rule: RuleId,
    pub(super) transitive_rule: RuleId,
}

impl ClosureConfig {
    #[must_use]
    pub const fn new(
        source_relation: RelationId,
        derived_relation: RelationId,
        rule_pack: RulePackId,
        base_rule: RuleId,
        transitive_rule: RuleId,
    ) -> Self {
        Self {
            source_relation,
            derived_relation,
            rule_pack,
            base_rule,
            transitive_rule,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Hard limits checked before execution or before returning a receipt.
pub struct ClosureLimits {
    pub(super) max_input_facts: NonZeroUsize,
    pub(super) max_derived_pairs: NonZeroUsize,
    pub(super) max_results: NonZeroUsize,
}

impl ClosureLimits {
    #[must_use]
    pub const fn new(
        max_input_facts: NonZeroUsize,
        max_derived_pairs: NonZeroUsize,
        max_results: NonZeroUsize,
    ) -> Self {
        Self {
            max_input_facts,
            max_derived_pairs,
            max_results,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Declares whether every derived candidate fit in the caller's output allowance.
pub enum ClosureStatus {
    /// Every derived candidate is present in the receipt.
    Complete,
    /// Evaluation completed, but the sorted receipt was truncated to `max_results`.
    OutputTruncated,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Fail-closed rejection reasons for unsupported or unbounded inputs.
pub enum ClosureError {
    /// The Scheme solver rejected or could not execute the native request.
    #[cfg(any(feature = "native-inference", feature = "worker-inference"))]
    NativeInference(mrr_gerbil::FiniteInferenceError),
    /// The canonical MRR bundle validator rejected the input.
    BundleRejected { reason: String },
    /// Transitions require a separately materialized semantic generation before evaluation.
    TransitionsRequireMaterializedSnapshot { count: usize },
    /// A configured relation is absent from the bundle.
    RelationMissing { relation: RelationId },
    /// This adapter only owns binary transitive closure.
    RelationMustBeBinary { relation: RelationId, arity: usize },
    /// A configured rule identity is absent from the bundle.
    RuleMissing { rule: RuleId },
    /// The configured rule pack is absent from the bundle.
    RulePackMissing { rule_pack: RulePackId },
    /// A configured rule exists elsewhere but not in the selected authority unit.
    RuleNotInPack { rule: RuleId, rule_pack: RulePackId },
    /// The configured rule exists but does not encode the supported closure form.
    RuleShapeMismatch { rule: RuleId },
    /// Source facts exceed the pre-execution input limit.
    InputFactBudgetExceeded { required: usize, limit: usize },
    /// The finite node domain could exceed the configured pair capacity.
    DerivedPairBudgetExceeded { required: usize, limit: usize },
    /// A source fact is not a pair of strings.
    UnsupportedSourceFact { fact: FactId },
    /// A source fact belongs to another semantic generation.
    SourceGenerationMismatch {
        fact: FactId,
        expected: GenerationId,
        actual: GenerationId,
    },
    /// A facade-bound bundle fact belongs to another semantic snapshot generation.
    BundleGenerationMismatch {
        fact: FactId,
        expected: GenerationId,
        actual: GenerationId,
    },
    /// This narrow adapter cannot ignore seeded facts in its derived relation.
    SeededDerivedRelationUnsupported { fact: FactId },
    /// The independent deterministic witness reconstruction disagreed with `Ascent`.
    InternalWitnessMismatch,
}

impl fmt::Display for ClosureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ClosureError {}

#[derive(Clone, Debug, Eq, PartialEq)]
/// A provenance-bearing result awaiting identity allocation and lineage admission upstream.
pub struct DerivationCandidate {
    pub(super) relation: RelationId,
    pub(super) values: [Value; 2],
    pub(super) rule: RuleId,
    pub(super) generation: GenerationId,
    pub(super) support: Vec<FactId>,
}

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
/// Domain-framed SHA-256 digest of one complete or explicitly truncated closure receipt.
pub struct DerivationReceiptDigest(pub(super) [u8; 32]);

impl DerivationReceiptDigest {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for DerivationReceiptDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl fmt::Display for DerivationReceiptDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl DerivationCandidate {
    #[must_use]
    pub const fn relation(&self) -> RelationId {
        self.relation
    }

    #[must_use]
    pub fn values(&self) -> &[Value; 2] {
        &self.values
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
    pub fn support(&self) -> &[FactId] {
        &self.support
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Typed result of one bounded evaluation request.
pub struct ClosureReceipt {
    pub(super) rule_pack: RulePackId,
    pub(super) input_generation: GenerationId,
    pub(super) input_fact_ids: Vec<FactId>,
    pub(super) status: ClosureStatus,
    pub(super) candidates: Vec<DerivationCandidate>,
    pub(super) digest: DerivationReceiptDigest,
}

impl ClosureReceipt {
    #[must_use]
    pub const fn rule_pack(&self) -> RulePackId {
        self.rule_pack
    }

    #[must_use]
    pub const fn input_generation(&self) -> GenerationId {
        self.input_generation
    }

    #[must_use]
    pub fn input_fact_ids(&self) -> &[FactId] {
        &self.input_fact_ids
    }

    #[must_use]
    pub const fn status(&self) -> ClosureStatus {
        self.status
    }

    #[must_use]
    pub const fn input_fact_count(&self) -> usize {
        self.input_fact_ids.len()
    }

    #[must_use]
    pub fn candidates(&self) -> &[DerivationCandidate] {
        &self.candidates
    }

    #[must_use]
    pub const fn digest(&self) -> DerivationReceiptDigest {
        self.digest
    }
}
