//! Package-level evidence graph receipts for GQL Rust harness policy checks.

use serde::Serialize;

use crate::evidence::{
    MrrRustProjectHarnessEvidenceGraphInput, MrrRustProjectHarnessEvidenceGraphSummary,
    summarize_client_db_evidence_graph,
};

/// Request for building a package-level evidence graph receipt.
#[derive(Clone, Debug)]
pub struct MrrRustProjectHarnessPackageEvidenceGraphRequest<'a> {
    pub package_name: String,
    pub evidence_graph: &'a MrrRustProjectHarnessEvidenceGraphInput,
}

/// Receipt that ties a package policy crate to the workspace evidence graph.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MrrRustProjectHarnessPackageEvidenceGraphReceipt {
    pub package_name: String,
    pub evidence_graph_summary: MrrRustProjectHarnessEvidenceGraphSummary,
}

/// Builds a package evidence graph receipt without writing artifacts.
pub fn build_package_evidence_graph_receipt(
    request: MrrRustProjectHarnessPackageEvidenceGraphRequest<'_>,
) -> MrrRustProjectHarnessPackageEvidenceGraphReceipt {
    MrrRustProjectHarnessPackageEvidenceGraphReceipt {
        package_name: request.package_name,
        evidence_graph_summary: summarize_client_db_evidence_graph(request.evidence_graph),
    }
}
