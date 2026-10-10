//! Re-entry from generic storage through current MRR source admission.

use super::{
    admission::{
        AdmittedAgenticAiContext, AgenticAiContextAdmissionRequest, admit_agentic_ai_context,
    },
    error::AgenticAiContextAdmissionError,
    manifest::{AGENTIC_AI_CONTEXT_MANIFEST_SCHEMA, AgenticAiContextManifestRecord},
};
use crate::{AgenticAiContextLimits, ReasoningBundle, SemanticSnapshot, bind_query_to_catalog};

/// Storage candidate, caller-owned resource ceiling and trusted expected identity.
/// The expected digest must come from the consumer's admitted reference, not
/// from the same untrusted record being restored.
pub struct AgenticAiContextRestoreRequest<'a> {
    pub record: &'a AgenticAiContextManifestRecord,
    pub expected_digest: [u8; 32],
    pub limits: AgenticAiContextLimits,
}

/// Reconstruct selection through existing facade admission, then compare every
/// canonical metadata field and the caller's expected manifest identity.
pub fn restore_agentic_ai_context(
    bundle: &ReasoningBundle,
    snapshot: &SemanticSnapshot,
    request: AgenticAiContextRestoreRequest<'_>,
) -> Result<AdmittedAgenticAiContext, AgenticAiContextAdmissionError> {
    let record = request.record;
    if record.schema != AGENTIC_AI_CONTEXT_MANIFEST_SCHEMA {
        return Err(AgenticAiContextAdmissionError::ManifestSchemaMismatch);
    }
    check_record_budget(record, request.limits)?;
    if record.source_bundle != bundle.id() {
        return Err(AgenticAiContextAdmissionError::SourceBundleMismatch);
    }
    if record.generation != snapshot.generation() || record.snapshot_digest != *snapshot.digest() {
        return Err(AgenticAiContextAdmissionError::QueryBindingMismatch);
    }
    let query = bind_query_to_catalog(bundle, record.query, snapshot)
        .map_err(AgenticAiContextAdmissionError::QueryBinding)?;
    if record.query_binding_digest != *query.digest() {
        return Err(AgenticAiContextAdmissionError::QueryBindingMismatch);
    }
    let admitted = admit_agentic_ai_context(
        bundle,
        snapshot,
        &query,
        AgenticAiContextAdmissionRequest {
            contract: record.contract.clone(),
            roots: record.roots.clone(),
            dependencies: record.dependencies.clone(),
            limits: record.limits,
        },
    )?;
    if admitted.manifest().record() != record
        || admitted.manifest().digest() != &request.expected_digest
    {
        return Err(AgenticAiContextAdmissionError::ManifestMismatch);
    }
    Ok(admitted)
}

fn check_record_budget(
    record: &AgenticAiContextManifestRecord,
    limits: AgenticAiContextLimits,
) -> Result<(), AgenticAiContextAdmissionError> {
    let max_elements = limits.max_elements.get();
    let max_edges = limits.max_dependency_edges.get();
    if record.limits.max_elements > limits.max_elements
        || record.limits.max_dependency_edges > limits.max_dependency_edges
        || record.limits.max_rendered_bytes > limits.max_rendered_bytes
        || record.roots.len() > max_elements
        || record.contract.required.len() > max_elements
        || record.contract.temporal_receipts.len() > max_elements
        || record.dependencies.len() > max_elements
        || record.selected.len() > max_elements
    {
        return Err(AgenticAiContextAdmissionError::ManifestBudget);
    }
    let mut edges = 0_usize;
    for (_, dependencies) in &record.dependencies {
        edges = edges
            .checked_add(dependencies.len())
            .ok_or(AgenticAiContextAdmissionError::ManifestBudget)?;
        if edges > max_edges {
            return Err(AgenticAiContextAdmissionError::ManifestBudget);
        }
    }
    Ok(())
}
