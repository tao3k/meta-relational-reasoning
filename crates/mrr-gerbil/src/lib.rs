//! Build-time Gerbil native projection and provenance admission.
//!
//! Executes the canonical AOT library on one Gambit owner thread. Scheme owns
//! finite inference; semantic callers retain identity and generation admission.

mod driver_cli;
mod native;
mod native_worker;
mod projection;
mod worker_errors;
mod worker_profile;
mod worker_projection;
mod worker_server;
mod worker_wire;

pub use native_worker::{NativeWorker, NativeWorkerError};
pub use worker_server::run_native_worker;

pub use driver_cli::run_driver_cli;
pub use native::{
    DriverError, DriverPhase, DriverResource, DriverStatus, DriverTransition,
    EnhancedQueryOperandSpec, EnhancedQueryOperatorSpec, EnhancedQueryOperatorTableLoadError,
    EnhancedQueryRecoverySpec, EnhancedTreeSitterQueryOperatorTable, FeatureDependencySpec,
    FeatureSpec, FiniteInferenceCandidate, FiniteInferenceError, IsoProfile, IsoProfileLoadError,
    ModuleSpec, NativeRuntimeStatus, PARSE_ARTIFACT_SCHEMA_V1, PARSER_NATIVE_DESCRIPTOR_SCHEMA_V1,
    ParseArtifact, ParseArtifactLoadError, ParseArtifactStatus, ParseEvent, ParserAuthority,
    ParserAuthorityLoadError, ParserCst, ParserCstError, ParserKindCatalog, ParserKindCategory,
    ParserKindSpec, ParserLanguage, ParserSyntax, ParserSyntaxKind, ProfileModuleSpec, ProfileSpec,
    ProfileSupplementSpec, ReasoningBundleLoadError, ReleaseSpec, TemporalHost,
    TemporalRuntimeError, driver_request, driver_transition, evaluate_finite_relations,
    load_enhanced_tree_sitter_query_operator_table, load_iso_profile, load_parser_authority,
    load_reasoning_bundle, parse_cypher_artifact, parse_gql_artifact,
};
pub use projection::{
    GrammarProjectionError, stamp_projection, validate_projection, workspace_input_fingerprint,
};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;

pub use worker_profile::{WorkerFailure, configure_native_worker, shutdown_native_worker};
