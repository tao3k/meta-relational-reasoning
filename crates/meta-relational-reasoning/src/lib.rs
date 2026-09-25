//! Public facade for the language-neutral MRR contract graph.
#![forbid(unsafe_code)]
mod admission;
mod api;
mod binding;
mod counterexample;
mod query_result;
mod truth;
mod typing;
pub use admission::{
    BundleBoundClosure, CandidateIdentities, ClosureAdmissionError, DerivationReceipt,
    MaterializedClosure, admit_closure_candidates,
};
pub use api::{
    DeductionLimits, DeductionPlan, EngineBuildError, EngineQueryError, MrrEngine, MrrEngineBuilder,
};
pub use binding::{
    CatalogBoundQuery, QueryCatalogBindingError, ResolvedProperty, bind_query_to_catalog,
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
    BundleError, EntityCatalog, EntityCatalogDigest, EntityCatalogError, InverseGoal,
    LineagePolicy, ProjectionPolicy, QueryTemplate, ReasoningBundle, ReasoningBundleDeclaration,
    RelationCatalog, RelationCatalogDigest, RelationCatalogError, RulePack,
    TransitionSystem as BundleTransitionSystem, ValidationProfile,
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
    GraphPattern, Grouping, MetaQueryIr, NodePattern, Ordering, PageValue, Parameter, PathPattern,
    PathSegment, Projection, PropertyKey, QueryIrError, QueryResult, RelationPattern,
    RelationalGoal, RelationalGoalError, ResultMode, SetQuantifier, SortDirection, Term,
    UnaryOperator, Variable,
};
pub use mrr_relation::{
    EntitySchema, EvidenceCompleteness, Fact, FactProvenance, FactValidity, FloatWidth,
    RelationAuthority, RelationConstraint, RelationContext, RelationContextError, RelationError,
    RelationField, RelationSchema, TemporalUnit, TimezonePolicy, Value, ValueKind, ValueSchema,
};
pub use mrr_revision::{
    ExternalRevisionIdentity, RevisionBinding, RevisionBindingError, SemanticSnapshot,
    SemanticSnapshotError,
};
pub use mrr_search::{
    SearchFactor, SearchFactorEdge, SearchFactorRole, SearchFrameworkError, SearchFrameworkLimits,
    SearchFrameworkReceipt, SearchFrameworkStatus, SearchInfluence, SearchObservation,
    SearchReasoningDigest, evaluate_search_factors,
};
pub use mrr_transition::{
    Action, CounterexampleIr, Effect, InitialState, Invariant, Precondition, SafetyCheckReceipt,
    SafetyLimits, SafetyStatus, StatePredicate, StateSchema, StateSnapshot,
    Transition as GenerationTransition, TransitionError as GenerationTransitionError,
    TransitionModelError, TransitionStep, TransitionSystem,
};
pub use query_result::{
    CandidateQueryResult, QueryResultAdmissionError, QueryResultAdmissionReceipt,
    QueryResultBinding, QueryResultLimits, QueryResultValue, QueryResultValueKind,
    admit_query_result_candidate,
};
pub use truth::{TruthStatus, conflict_truth, intent_binding_truth, safety_truth, why_not_truth};
pub use typing::{ExpressionType, ParameterType, QueryType, ResultField, StaticQueryTyping};
#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
