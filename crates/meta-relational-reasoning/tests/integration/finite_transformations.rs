//! Kernel processes and native workers share a Rust process Host. Embedded
//! Gambit has a process-wide child reaper and is tested in the separate unit binary.
pub use meta_relational_reasoning::{
    AuthenticatedFiniteTransformationCatalog, CompiledFiniteOptimizationRows,
    ExternalRevisionIdentity, FiniteDecisionProblem, FiniteOptimizationProblem,
    FiniteOptimizationTransport, FinitePartialTransport, FiniteProblem,
    FiniteTransformationCatalog, FiniteTransport, GenerationId, KernelCheckedDecisionReduction,
    KernelCheckedFiniteCatalog, KernelCheckedOptimizationReduction,
    KernelCheckedPartialTransformation, RevisionBinding, SemanticSnapshot,
    SignedTransformationSourceGrant, TransformationAffineBound, TransformationBinding,
    TransformationCapabilities, TransformationError, TransformationExecutionBudget,
    TransformationGrantLedger, TransformationLimits, TransformationPlanCandidate,
    TransformationResourceContract, TransformationResultStore, TransformationRuntime,
    TransformationSourceGrant, TransformationStep, TransformationVerifier, TruthStatus, Value,
    ValueSchema, admit_transformation, admit_transformation_plan, execute_transformation_plan,
    transformation_grant_binding_digest, transformation_grant_catalog_digest,
    transformation_identity, transformation_value_digest,
};
#[path = "../unit/transformation_finite/mod.rs"]
mod finite;
#[path = "../unit/transformation_fixture.rs"]
mod transformation;
