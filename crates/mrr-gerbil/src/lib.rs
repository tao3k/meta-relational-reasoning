//! Build-time Gerbil native projection and provenance admission.
//!
//! This crate is intentionally absent from the query runtime dependency graph.
//! It consumes a Gerbil AOT artifact produced by `build.ss`; no Scheme runtime
//! is started from a query hot path.

mod driver_cli;
mod native;
mod projection;

pub use driver_cli::run_driver_cli;
pub use native::{
    DriverError, DriverPhase, DriverResource, DriverStatus, DriverTransition,
    EnhancedQueryOperandSpec, EnhancedQueryOperatorSpec, EnhancedQueryOperatorTableLoadError,
    EnhancedQueryRecoverySpec, EnhancedTreeSitterQueryOperatorTable, FeatureDependencySpec,
    FeatureSpec, IsoProfile, IsoProfileLoadError, ModuleSpec, NativeRuntimeStatus,
    PARSE_ARTIFACT_SCHEMA_V1, PARSER_NATIVE_DESCRIPTOR_SCHEMA_V1, ParseArtifact,
    ParseArtifactLoadError, ParseArtifactStatus, ParseEvent, ParserAuthority,
    ParserAuthorityLoadError, ParserCst, ParserCstError, ParserKindCatalog, ParserKindCategory,
    ParserKindSpec, ParserLanguage, ParserSyntax, ParserSyntaxKind, ProfileModuleSpec, ProfileSpec,
    ProfileSupplementSpec, ReasoningBundleLoadError, ReleaseSpec, driver_request,
    driver_transition, load_enhanced_tree_sitter_query_operator_table, load_iso_profile,
    load_parser_authority, load_reasoning_bundle, parse_cypher_artifact, parse_gql_artifact,
};
pub use projection::{
    GrammarProjectionError, stamp_projection, validate_projection, workspace_input_fingerprint,
};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
