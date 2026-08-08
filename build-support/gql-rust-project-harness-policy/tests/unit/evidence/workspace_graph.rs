use std::path::PathBuf;

use gql_rust_project_harness_policy::GqlRustProjectHarnessEvidenceGraphInput;
use gql_rust_project_harness_policy::workspace_evidence_graph::{
    GqlRustProjectHarnessWorkspaceEvidenceGraphEdgeKind,
    GqlRustProjectHarnessWorkspaceEvidenceGraphNodeKind,
    GqlRustProjectHarnessWorkspaceEvidenceGraphRequest, build_asp_workspace_evidence_graph_receipt,
    build_workspace_evidence_graph_receipt,
};

#[test]
fn workspace_receipt_projects_member_crates_and_client_db_graph() {
    let graph = GqlRustProjectHarnessEvidenceGraphInput {
        generation_id: "gen-workspace".to_string(),
        project_root: PathBuf::from("/tmp/gql"),
        node_count: 0,
        edge_count: 0,
    };

    let receipt = build_workspace_evidence_graph_receipt(
        GqlRustProjectHarnessWorkspaceEvidenceGraphRequest {
            workspace_label: "gql-rust".to_string(),
            workspace_root: PathBuf::from("/tmp/gql"),
            member_crate_names: vec!["gql-core".to_string()],
            client_db_evidence_graph: &graph,
        },
    );

    assert_eq!(receipt.summary.member_crate_count, 1);
    assert_eq!(receipt.summary.evidence_graph_node_count, 3);
    assert_eq!(receipt.summary.evidence_graph_edge_count, 2);
    assert!(
        receipt
            .nodes
            .iter()
            .any(|node| node.kind == GqlRustProjectHarnessWorkspaceEvidenceGraphNodeKind::Workspace)
    );
    assert!(
        receipt
            .edges
            .iter()
            .any(|edge| edge.kind == GqlRustProjectHarnessWorkspaceEvidenceGraphEdgeKind::Contains)
    );
}

#[test]
fn default_workspace_receipt_uses_central_member_policy_registry() {
    let graph = GqlRustProjectHarnessEvidenceGraphInput {
        generation_id: "gen-default-workspace".to_string(),
        project_root: PathBuf::from("/tmp/gql"),
        node_count: 0,
        edge_count: 0,
    };

    let receipt = build_asp_workspace_evidence_graph_receipt(PathBuf::from("/tmp/gql"), &graph);

    assert_eq!(
        receipt.summary.member_crate_count,
        gql_rust_project_harness_policy::gql_workspace_member_policies().len()
    );
    assert!(
        receipt
            .members
            .iter()
            .any(|member| member.package_name == "gql-core")
    );
}
