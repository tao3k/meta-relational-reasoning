use mrr_asp_rust_build_support::MrrAspRustEvidenceGraphInput;
use mrr_asp_rust_build_support::workspace_evidence_graph::{
    MrrAspRustWorkspaceEvidenceGraphEdgeKind, MrrAspRustWorkspaceEvidenceGraphNodeKind,
    MrrAspRustWorkspaceEvidenceGraphRequest, build_mrr_workspace_evidence_graph_receipt,
    build_workspace_evidence_graph_receipt,
};

#[test]
fn workspace_receipt_projects_member_crates_and_client_db_graph() {
    let root = std::env::temp_dir().join("mrr");
    let graph = MrrAspRustEvidenceGraphInput {
        generation_id: "gen-workspace".to_string(),
        project_root: root.clone(),
        node_count: 0,
        edge_count: 0,
    };

    let receipt = build_workspace_evidence_graph_receipt(MrrAspRustWorkspaceEvidenceGraphRequest {
        workspace_label: "meta-relational-reasoning".to_string(),
        workspace_root: root,
        member_crate_names: vec!["mrr-query".to_string()],
        client_db_evidence_graph: &graph,
    });

    assert_eq!(receipt.summary.member_crate_count, 1);
    assert_eq!(receipt.summary.evidence_graph_node_count, 3);
    assert_eq!(receipt.summary.evidence_graph_edge_count, 2);
    assert!(
        receipt
            .nodes
            .iter()
            .any(|node| node.kind == MrrAspRustWorkspaceEvidenceGraphNodeKind::Workspace)
    );
    assert!(
        receipt
            .edges
            .iter()
            .any(|edge| edge.kind == MrrAspRustWorkspaceEvidenceGraphEdgeKind::Contains)
    );
}

#[test]
fn default_workspace_receipt_uses_central_member_policy_registry() {
    let root = std::env::temp_dir().join("mrr");
    let graph = MrrAspRustEvidenceGraphInput {
        generation_id: "gen-default-workspace".to_string(),
        project_root: root.clone(),
        node_count: 0,
        edge_count: 0,
    };

    let receipt = build_mrr_workspace_evidence_graph_receipt(root, &graph);

    assert_eq!(
        receipt.summary.member_crate_count,
        mrr_asp_rust_build_support::mrr_workspace_member_policies().len()
    );
    assert!(
        receipt
            .members
            .iter()
            .any(|member| member.package_name == "mrr-query")
    );
}
