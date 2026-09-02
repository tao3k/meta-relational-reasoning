use std::path::PathBuf;

use mrr_asp_rust_project_policy::{
    MrrAspRustEvidenceGraphInput, MrrAspRustPackageEvidenceGraphRequest,
    build_package_evidence_graph_receipt,
};

#[test]
fn package_receipt_summarizes_client_db_evidence_graph() {
    let graph = MrrAspRustEvidenceGraphInput {
        generation_id: "gen-test".to_string(),
        project_root: PathBuf::from("/tmp/gql"),
        node_count: 0,
        edge_count: 0,
    };

    let receipt = build_package_evidence_graph_receipt(MrrAspRustPackageEvidenceGraphRequest {
        package_name: "gql-core".to_string(),
        evidence_graph: &graph,
    });

    assert_eq!(receipt.package_name, "gql-core");
    assert_eq!(receipt.evidence_graph_summary.generation_id, "gen-test");
    assert_eq!(receipt.evidence_graph_summary.node_count, 0);
    assert_eq!(receipt.evidence_graph_summary.edge_count, 0);
}
