//! Workspace-level evidence graph receipts for GQL Rust policy crates.

use std::path::PathBuf;

use serde::Serialize;

use crate::evidence::MrrAspRustEvidenceGraphInput;
use crate::member_policy::mrr_workspace_member_policies;
use crate::package_evidence_graph::{
    MrrAspRustPackageEvidenceGraphReceipt, MrrAspRustPackageEvidenceGraphRequest,
    build_package_evidence_graph_receipt,
};

/// Request for projecting GQL workspace evidence into a workspace graph.
#[derive(Clone, Debug)]
pub struct MrrAspRustWorkspaceEvidenceGraphRequest<'a> {
    pub workspace_label: String,
    pub workspace_root: PathBuf,
    pub member_crate_names: Vec<String>,
    pub client_db_evidence_graph: &'a MrrAspRustEvidenceGraphInput,
}

/// Workspace evidence graph receipt for GQL policy checks.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MrrAspRustWorkspaceEvidenceGraphReceipt {
    pub schema_id: &'static str,
    pub schema_version: &'static str,
    pub workspace_label: String,
    pub workspace_root: PathBuf,
    pub summary: MrrAspRustWorkspaceEvidenceGraphSummaryReceipt,
    pub members: Vec<MrrAspRustPackageEvidenceGraphReceipt>,
    pub nodes: Vec<MrrAspRustWorkspaceEvidenceGraphNodeReceipt>,
    pub edges: Vec<MrrAspRustWorkspaceEvidenceGraphEdgeReceipt>,
}

/// Aggregated counts for the GQL workspace evidence graph.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MrrAspRustWorkspaceEvidenceGraphSummaryReceipt {
    pub member_crate_count: usize,
    pub client_db_graph_node_count: usize,
    pub client_db_graph_edge_count: usize,
    pub evidence_graph_node_count: usize,
    pub evidence_graph_edge_count: usize,
}

/// One node in the GQL workspace evidence graph receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MrrAspRustWorkspaceEvidenceGraphNodeReceipt {
    pub id: String,
    pub kind: MrrAspRustWorkspaceEvidenceGraphNodeKind,
    pub label: String,
}

/// Node kind in the GQL workspace evidence graph receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MrrAspRustWorkspaceEvidenceGraphNodeKind {
    Workspace,
    MemberCrate,
    ClientDbEvidenceGraph,
}

/// One directed edge in the GQL workspace evidence graph receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MrrAspRustWorkspaceEvidenceGraphEdgeReceipt {
    pub source: String,
    pub target: String,
    pub kind: MrrAspRustWorkspaceEvidenceGraphEdgeKind,
}

/// Edge kind in the GQL workspace evidence graph receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MrrAspRustWorkspaceEvidenceGraphEdgeKind {
    Contains,
    ProjectsClientDbEvidence,
}

/// Builds a GQL workspace evidence graph receipt without writing artifacts.
pub fn build_workspace_evidence_graph_receipt(
    request: MrrAspRustWorkspaceEvidenceGraphRequest<'_>,
) -> MrrAspRustWorkspaceEvidenceGraphReceipt {
    let workspace_node_id = format!("workspace:{}", request.workspace_label);
    let client_db_graph_node_id = format!(
        "client-db-evidence-graph:{}",
        request.client_db_evidence_graph.generation_id
    );

    let mut nodes = Vec::with_capacity(request.member_crate_names.len() + 2);
    nodes.push(MrrAspRustWorkspaceEvidenceGraphNodeReceipt {
        id: workspace_node_id.clone(),
        kind: MrrAspRustWorkspaceEvidenceGraphNodeKind::Workspace,
        label: request.workspace_label.clone(),
    });
    nodes.push(MrrAspRustWorkspaceEvidenceGraphNodeReceipt {
        id: client_db_graph_node_id.clone(),
        kind: MrrAspRustWorkspaceEvidenceGraphNodeKind::ClientDbEvidenceGraph,
        label: request.client_db_evidence_graph.generation_id.clone(),
    });

    let mut edges = vec![MrrAspRustWorkspaceEvidenceGraphEdgeReceipt {
        source: workspace_node_id.clone(),
        target: client_db_graph_node_id,
        kind: MrrAspRustWorkspaceEvidenceGraphEdgeKind::ProjectsClientDbEvidence,
    }];

    let mut members = Vec::with_capacity(request.member_crate_names.len());
    for member_crate_name in request.member_crate_names {
        let member_node_id = format!("member-crate:{member_crate_name}");
        nodes.push(MrrAspRustWorkspaceEvidenceGraphNodeReceipt {
            id: member_node_id.clone(),
            kind: MrrAspRustWorkspaceEvidenceGraphNodeKind::MemberCrate,
            label: member_crate_name.clone(),
        });
        edges.push(MrrAspRustWorkspaceEvidenceGraphEdgeReceipt {
            source: workspace_node_id.clone(),
            target: member_node_id,
            kind: MrrAspRustWorkspaceEvidenceGraphEdgeKind::Contains,
        });
        members.push(build_package_evidence_graph_receipt(
            MrrAspRustPackageEvidenceGraphRequest {
                package_name: member_crate_name,
                evidence_graph: request.client_db_evidence_graph,
            },
        ));
    }

    MrrAspRustWorkspaceEvidenceGraphReceipt {
        schema_id: "gql-rust-project-harness.workspace-evidence-graph",
        schema_version: "1",
        workspace_label: request.workspace_label,
        workspace_root: request.workspace_root,
        summary: MrrAspRustWorkspaceEvidenceGraphSummaryReceipt {
            member_crate_count: members.len(),
            client_db_graph_node_count: request.client_db_evidence_graph.node_count,
            client_db_graph_edge_count: request.client_db_evidence_graph.edge_count,
            evidence_graph_node_count: nodes.len(),
            evidence_graph_edge_count: edges.len(),
        },
        members,
        nodes,
        edges,
    }
}

/// Builds the default GQL workspace evidence graph from the central policy registry.
pub fn build_mrr_workspace_evidence_graph_receipt(
    workspace_root: PathBuf,
    client_db_evidence_graph: &MrrAspRustEvidenceGraphInput,
) -> MrrAspRustWorkspaceEvidenceGraphReceipt {
    build_workspace_evidence_graph_receipt(MrrAspRustWorkspaceEvidenceGraphRequest {
        workspace_label: "gql-rust".to_string(),
        workspace_root,
        member_crate_names: mrr_workspace_member_policies()
            .iter()
            .map(|policy| policy.package_name.to_string())
            .collect(),
        client_db_evidence_graph,
    })
}
