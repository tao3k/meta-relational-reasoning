//! Public facade for the language-neutral MRR contract graph.
#![forbid(unsafe_code)]
mod admission;
#[cfg(feature = "agentic-ai-context")]
mod agentic_ai_context;
mod api;
mod binding;
mod counterexample;
mod query_result;
mod transformation;
mod transformation_async;
mod transformation_execution;
pub use transformation_async::{AsyncTransformationRuntime, execute_transformation_plan_async};
mod transformation_publication;
#[cfg(feature = "native-inference")]
mod transformation_route;
mod transformation_table;
pub use transformation_table::transport_bytes as transport_transformation_bytes;
mod truth;
mod typing;
pub use admission::{
    BundleBoundClosure, CandidateIdentities, ClosureAdmissionError,
    ClosureCandidateComparisonError, ClosureCandidateRow, ClosurePairComparisonError,
    DerivationReceipt, MaterializedClosure, admit_closure_candidates, compare_closure_candidates,
    compare_closure_pairs,
};
#[cfg(feature = "agentic-ai-context")]
pub use agentic_ai_context::{
    AGENTIC_AI_CONTEXT_MANIFEST_SCHEMA, AGENTIC_AI_CONTEXT_QUERY_SELECTION_SCHEMA,
    AdmittedAgenticAiContext, AdmittedAgenticAiContextMaterialization,
    AdmittedAgenticAiContextQuerySelection, AdmittedExactAgenticAiContextQuerySelection,
    AgenticAiContextAdmissionError, AgenticAiContextAdmissionRequest,
    AgenticAiContextComposedMaterializationRequest, AgenticAiContextCompositionError,
    AgenticAiContextCompositionReceipt, AgenticAiContextCompositionRequest,
    AgenticAiContextExactSelectionRestoreRequest, AgenticAiContextManifest,
    AgenticAiContextManifestRecord, AgenticAiContextMaterializationRequest,
    AgenticAiContextQuerySelectionRecord, AgenticAiContextQuerySelectionRequest,
    AgenticAiContextQuerySelectionRestoreRequest, AgenticAiContextRestoreRequest,
    AgenticAiContextRevisionReceipt, AgenticAiContextRevisionRequest, admit_agentic_ai_context,
    compare_agentic_ai_context_revision, restore_agentic_ai_context,
    restore_agentic_ai_context_query_selection, restore_agentic_ai_context_query_selection_exact,
    select_agentic_ai_context_from_query, select_agentic_ai_context_from_query_exact,
};
#[cfg(feature = "agentic-ai-context-tokens")]
pub use agentic_ai_context::{
    AgenticAiContextTokenBindingRequest, AgenticAiContextTokenizationError,
    AgenticAiContextTokenizationRequest, SourceBoundAgenticAiContextTokens,
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
#[cfg(feature = "agentic-ai-context")]
pub use mrr_agentic_ai_context::{
    AgenticAiContextClosure, AgenticAiContextComposer, AgenticAiContextComposition,
    AgenticAiContextCompositionGraph, AgenticAiContextCompositionGraphInput,
    AgenticAiContextCompositionNode, AgenticAiContextCompositionProducer, AgenticAiContextContract,
    AgenticAiContextElement, AgenticAiContextError, AgenticAiContextLimits,
    AgenticAiContextMaterialization, AgenticAiContextQuery, AgenticAiContextRenderedElement,
    AgenticAiContextRevision, AgenticAiContextSpan, AgenticAiContextState,
    AgenticAiContextStateInput, SemanticReuseCertificate,
};
#[cfg(feature = "agentic-ai-context-tokens")]
pub use mrr_agentic_ai_context::{
    AgenticAiContextComputationalIdentity, AgenticAiContextReuseEligibility,
    AgenticAiContextTokenLayout, AgenticAiContextTokenizer,
};
pub use mrr_bundle::{
    BundleError, EntityCatalog, EntityCatalogDigest, EntityCatalogError, InverseGoal,
    LineagePolicy, ProjectionPolicy, QueryTemplate, ReasoningBundle, ReasoningBundleDeclaration,
    RelationCatalog, RelationCatalogDigest, RelationCatalogError, RulePack,
    TransitionSystem as BundleTransitionSystem, ValidationProfile,
};
pub use mrr_deduction::{
    ClosureError as DeductionError, ClosureReceipt, ClosureStatus, DerivationCandidate,
    DerivationReceiptDigest,
};
pub use mrr_identity::{
    ActionId, DerivationId, EntityId, FactId, GenerationId, LineageEdgeId, LineageNodeId, QueryId,
    QueryOperatorId, ReasoningBundleId, RelationId, RevisionId, RuleId, RulePackId, StateId,
    TransformationId, TransitionId,
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
    SearchReasoningDigest,
};
pub use mrr_transition::{
    Action, CounterexampleIr, Effect, InitialState, Invariant, Precondition, SafetyCheckReceipt,
    SafetyLimits, SafetyStatus, StatePredicate, StateSchema, StateSnapshot,
    Transition as GenerationTransition, TransitionError as GenerationTransitionError,
    TransitionModelError, TransitionStep, TransitionSystem,
};
pub use query_result::{
    CandidateQueryResult, QueryResultAdmissionError, QueryResultAdmissionReceipt,
    QueryResultBinding, QueryResultLimits, QueryResultTransportError, QueryResultValue,
    QueryResultValueKind, VerifiedQueryResultTransport, admit_query_result_candidate,
    export_query_result_transport, verify_query_result_transport,
};
pub use truth::{TruthStatus, conflict_truth, intent_binding_truth, safety_truth, why_not_truth};
pub use typing::{ExpressionType, ParameterType, QueryType, ResultField, StaticQueryTyping};

pub use transformation::{
    TransformationAdmission, TransformationBinding, TransformationDefinition,
    TransformationEndpoint, TransformationError, TransformationEvidence, TransformationLimits,
    TransformationPlanAdmission, TransformationPlanCandidate, TransformationProfile,
    TransformationStep, TransformationVerifier, admit_transformation, admit_transformation_plan,
    decode_transformation_definition, encode_transformation_definition,
    transformation_failure_truth, transformation_identity,
};

pub use transformation_execution::{
    TransformationExecutionReceipt, TransformationRuntime, execute_transformation_plan,
    transformation_value_digest,
};
#[cfg(feature = "native-inference")]
pub use transformation_route::{TransformationRouteSearch, search_transformation_routes};

pub use transformation_publication::{PublishedTransformationResult, TransformationResultSlot};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;

#[cfg(feature = "native-inference")]
pub use mrr_search::evaluate_search_factors;

/// Advanced native Host projection of the POO Library's inert Scheme v1 ABI.
/// These process-local proof operations do not authorize external effects.
#[cfg(feature = "native-inference")]
pub use mrr_deduction::{
    NativeWorker, NativeWorkerError, TemporalHost, TemporalRuntimeError, configure_native_worker,
    shutdown_native_worker,
};

mod property_execution;
pub use property_execution::{
    AdmittedPropertyExecution, PropertyExecutionCandidate, PropertyExecutionError,
    PropertyQueryBackend,
};

mod transformation_finite;
mod transformation_store;
pub use transformation_finite::{
    FiniteProblem, FiniteTransformationCatalog, FiniteTransformationOwner, FiniteTransport,
};
pub use transformation_store::{
    AsyncTransformationPublisher, TransformationPublicationLease, TransformationResultStore,
};

mod transformation_kernel;
pub use transformation_kernel::{FiniteKernelCertificate, KernelCheckedFiniteCatalog};

mod transformation_projection;
pub use transformation_projection::{
    TransformationRouteCandidate, verify_transformation_projection,
};

mod transformation_authority;
pub use transformation_authority::{
    AuthenticatedFiniteTransformationCatalog, AuthenticatedTransformationGrant,
    SignedTransformationSourceGrant, TransformationAffineBound, TransformationAuthorityLease,
    TransformationCapabilities, TransformationExecutionBudget, TransformationGrantLedger,
    TransformationGrantOperation, TransformationResourceContract, TransformationSourceGrant,
    transformation_grant_binding_digest, transformation_grant_catalog_digest,
};

mod transformation_profiles;
pub use transformation_profiles::{
    CompiledFiniteOptimizationRows, FiniteDecisionProblem, FiniteOptimizationProblem,
    FiniteOptimizationTransport, FinitePartialTransport, KernelCheckedDecisionReduction,
    KernelCheckedOptimizationReduction, KernelCheckedPartialTransformation,
};

#[cfg(feature = "native-inference")]
mod transformation_native;
#[cfg(feature = "native-inference")]
pub use transformation_native::{
    NativeTransformationExecution, NativeTransformationExecutionRequest,
    execute_native_transformation_plan, execute_native_transformation_plan_async,
};
