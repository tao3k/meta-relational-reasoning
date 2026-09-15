//! Native AOT grammar binding interface.

mod driver;
mod enhanced_query;
#[allow(unsafe_code)]
mod ffi;
mod model;
pub(crate) mod parse_artifact;
mod parser_cst;
mod reasoning;
mod runtime;

pub use driver::{
    DriverError, DriverPhase, DriverResource, DriverStatus, DriverTransition, driver_request,
    driver_transition,
};
pub use enhanced_query::{
    EnhancedQueryOperandSpec, EnhancedQueryOperatorSpec, EnhancedQueryOperatorTableLoadError,
    EnhancedQueryRecoverySpec, EnhancedTreeSitterQueryOperatorTable,
    load_enhanced_tree_sitter_query_operator_table,
};
pub(crate) use model::NativeGrammar;
pub use model::{
    FeatureDependencySpec, FeatureSpec, IsoProfile, IsoProfileLoadError, ModuleSpec,
    ParserAuthority, ParserAuthorityLoadError, ProfileModuleSpec, ProfileSpec,
    ProfileSupplementSpec, ReleaseSpec, load_iso_profile, load_parser_authority,
};
pub use parse_artifact::{
    PARSE_ARTIFACT_SCHEMA_V1, PARSER_NATIVE_DESCRIPTOR_SCHEMA_V1, ParseArtifact,
    ParseArtifactLoadError, ParseArtifactStatus, ParseEvent, ParserKindCatalog, ParserKindCategory,
    ParserKindSpec, parse_gql_artifact,
};
pub use parser_cst::{ParserCst, ParserCstError, ParserSyntax, ParserSyntaxKind};
pub use reasoning::{ReasoningBundleLoadError, load_reasoning_bundle};
