//! Public facade for the language-neutral MRR contract graph.
#![forbid(unsafe_code)]
mod admission;
mod api;
mod counterexample;
mod truth;
pub use admission::{
    CandidateIdentities, ClosureAdmissionError, DerivationReceipt, MaterializedClosure,
    admit_closure_candidates,
};
pub use api::{
    DeductionLimits, DeductionPlan, EngineBuildError, EngineQueryError, MrrEngine, MrrEngineBuilder,
};
pub use counterexample::{
    CounterexampleFactIdentity, CounterexampleLineageError, CounterexampleLineageIdentities,
    counterexample_lineage,
};
pub use mrr_ascent::{
    ClosureError as DeductionError, ClosureReceipt, ClosureStatus, DerivationCandidate,
    DerivationReceiptDigest,
};
pub use mrr_bundle::{
    BundleError, InverseGoal, LineagePolicy, ProjectionPolicy, QueryTemplate, ReasoningBundle,
    ReasoningBundleDeclaration, RulePack, TransitionSystem as BundleTransitionSystem,
    ValidationProfile,
};
pub use mrr_identity::{
    ActionId, DerivationId, EntityId, FactId, GenerationId, LineageEdgeId, LineageNodeId, QueryId,
    QueryOperatorId, ReasoningBundleId, RelationId, RevisionId, RuleId, RulePackId, StateId,
    TransitionId,
};
pub use mrr_intent::{
    IntentBindingStatus, IntentBundleBinding, IntentProjectionError, IntentSemanticModel,
};
pub use mrr_lineage::{
    Derivation, ExplanationGraph, ImpactError, ImpactGraph, LineageEdge, LineageEdgeKind,
    LineageError, LineageGraph, LineageGraphError, LineageNode, LineageNodeKind, WhyError,
};
pub use mrr_logic::{
    GroundAtom, Rule, RuleError, WhyNotError, WhyNotIncomplete, WhyNotLimits, WhyNotReceipt,
    WhyNotStatus,
};
pub use mrr_query::{
    Aggregation, AggregationFunction, Atom, BinaryOperator, Binding, Direction, Expression, Filter,
    GraphPattern, MetaQueryIr, NodePattern, Ordering, PathPattern, PathSegment, Projection,
    PropertyKey, QueryIrError, RelationPattern, RelationalGoal, RelationalGoalError, SortDirection,
    Term, UnaryOperator, Variable,
};
pub use mrr_relation::{
    EvidenceCompleteness, Fact, FactProvenance, FactValidity, RelationAuthority,
    RelationCardinality, RelationContext, RelationError, RelationField, RelationSchema, Value,
    ValueType,
};
pub use mrr_revision::{ExternalRevisionIdentity, RevisionBinding, RevisionBindingError};
pub use mrr_transition::{
    Action, CounterexampleIr, Effect, InitialState, Invariant, Precondition, SafetyCheckReceipt,
    SafetyLimits, SafetyStatus, StatePredicate, StateSchema, StateSnapshot,
    Transition as GenerationTransition, TransitionError as GenerationTransitionError,
    TransitionModelError, TransitionStep, TransitionSystem,
};
pub use truth::{TruthStatus, conflict_truth, intent_binding_truth, safety_truth, why_not_truth};
#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
