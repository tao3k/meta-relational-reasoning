//! MRR downstream policy crate for Rust project harness evidence graphs.

pub use rust_lang_project_harness::{
    RustHarnessConfig, RustOwnerResponsibility, RustProjectHarnessDownstreamPolicy,
    RustVerificationProfileHint, RustVerificationStabilityPictureConfig, RustVerificationTaskKind,
    assert_rust_project_harness_clean_with_config,
    assert_rust_project_harness_downstream_policy_from_env,
    assert_rust_project_harness_verification_from_env_with_config, default_rust_harness_config,
    rust_harness_config_for_project,
};

pub mod build_gate;
pub use build_gate::assert_mrr_rust_project_harness_member_policy_from_env;
pub mod evidence;
pub mod hook_registry_build;
/// Reusable hook scenarios for Rust project harness policy checks.
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
pub use member_policy::{
    MrrRustProjectHarnessMemberPolicy, MrrRustProjectHarnessOwnerPolicy,
    mrr_workspace_member_policies,
};
pub use scenario::{
    MrrRustProjectHarnessScenario, MrrRustProjectHarnessScenarioCommand,
    MrrRustProjectHarnessScenarioPackage,
};
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
    MrrRustProjectHarnessWorkspaceEvidenceGraphEdgeKind,
    MrrRustProjectHarnessWorkspaceEvidenceGraphEdgeReceipt,
    MrrRustProjectHarnessWorkspaceEvidenceGraphNodeKind,
    MrrRustProjectHarnessWorkspaceEvidenceGraphNodeReceipt,
    MrrRustProjectHarnessWorkspaceEvidenceGraphReceipt,
    MrrRustProjectHarnessWorkspaceEvidenceGraphRequest,
    MrrRustProjectHarnessWorkspaceEvidenceGraphSummaryReceipt,
    build_mrr_workspace_evidence_graph_receipt, build_workspace_evidence_graph_receipt,
};

pub use evidence::{
    MrrRustProjectHarnessEvidenceGraphInput, MrrRustProjectHarnessEvidenceGraphSummary,
    summarize_client_db_evidence_graph,
};
pub use package_evidence_graph::{
    MrrRustProjectHarnessPackageEvidenceGraphReceipt,
    MrrRustProjectHarnessPackageEvidenceGraphRequest, build_package_evidence_graph_receipt,
};
