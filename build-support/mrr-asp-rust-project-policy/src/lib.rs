//! MRR downstream policy crate for ASP Rust project evidence graphs.

pub use asp_rust::{
    AspRustConfig, AspRustDownstreamPolicy, AspRustWorkspacePolicy, RustOwnerResponsibility,
    RustVerificationProfileHint, RustVerificationStabilityPictureConfig, RustVerificationTaskKind,
    asp_rust_config_for_project, assert_asp_rust_clean_with_config,
    assert_asp_rust_downstream_policy_from_env, assert_asp_rust_verification_from_env_with_config,
    default_asp_rust_config,
};

pub mod evidence;
pub mod hook_registry_build;
/// Reusable hook scenarios for MRR ASP Rust project policy checks.
pub mod hook_scenarios;
pub mod member_policy;
pub mod package_evidence_graph;
pub mod scenario;
pub mod search_scenarios;
pub mod workspace_evidence_graph;

pub use hook_registry_build::generate_agent_semantic_hook_registry_from_env;
pub use hook_scenarios::{
    GENERIC_WRAPPER_TESTING_RESIDENT_DISPATCH_SCENARIO_ID, GQL_HOOK_SCENARIO_PACKAGE_NAME,
    gql_hook_scenario_package,
};
pub use member_policy::{MrrAspRustMemberPolicy, mrr_workspace_member_policies};
pub use scenario::{
    AspRustProjectScenario, AspRustProjectScenarioBenchmarkSpec, AspRustProjectScenarioCommand,
    AspRustProjectScenarioMeasurement, AspRustProjectScenarioMetricKind,
    AspRustProjectScenarioMetricSpec, AspRustProjectScenarioObservation,
    AspRustProjectScenarioPackage, measure_asp_rust_project_scenario,
    render_asp_rust_project_scenario_benchmark_toml,
    write_asp_rust_project_scenario_benchmark_toml,
};
pub use scenario::{asp_rust_project_scenario, asp_rust_project_scenario_package};
pub use search_scenarios::{
    GQL_SEARCH_SCENARIO_PACKAGE_NAME, LEXICAL_SEARCH_FRAME_GRAPH_ROUTER_WARM_PATH_SCENARIO_ID,
    SEARCH_DEGRADED_ROUTE_BOUNDED_SCENARIO_ID, SEARCH_GRAPH_ROUTER_NEXT_EXACT_ACTION_SCENARIO_ID,
    SEARCH_PACKAGE_LINEAR_PERFORMANCE_SCENARIO_ID, SEARCH_SOURCE_INDEX_BUSY_MISS_SCENARIO_ID,
    SEARCH_SOURCE_INDEX_COLD_REQUIRED_SCENARIO_ID,
    SEARCH_SOURCE_INDEX_OWNER_ITEM_GRAPH_CHAIN_SCENARIO_ID,
    SEARCH_SOURCE_INDEX_READ_ONLY_CLIENT_DB_SCENARIO_ID,
    SEARCH_SUBAGENT_COMPACT_RECEIPT_SCENARIO_ID, gql_search_scenario_package,
};

pub use workspace_evidence_graph::{
    MrrAspRustWorkspaceEvidenceGraphEdgeKind, MrrAspRustWorkspaceEvidenceGraphEdgeReceipt,
    MrrAspRustWorkspaceEvidenceGraphNodeKind, MrrAspRustWorkspaceEvidenceGraphNodeReceipt,
    MrrAspRustWorkspaceEvidenceGraphReceipt, MrrAspRustWorkspaceEvidenceGraphRequest,
    MrrAspRustWorkspaceEvidenceGraphSummaryReceipt, build_mrr_workspace_evidence_graph_receipt,
    build_workspace_evidence_graph_receipt,
};

pub use evidence::{
    MrrAspRustEvidenceGraphInput, MrrAspRustEvidenceGraphSummary,
    summarize_client_db_evidence_graph,
};
pub use package_evidence_graph::{
    MrrAspRustPackageEvidenceGraphReceipt, MrrAspRustPackageEvidenceGraphRequest,
    build_package_evidence_graph_receipt,
};
