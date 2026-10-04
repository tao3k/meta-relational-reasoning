//! Untrusted storage re-entry for result-derived Context provenance.

use std::num::NonZeroUsize;

use serde::{Deserialize, Serialize};

use super::{
    AdmittedAgenticAiContextQuerySelection, AgenticAiContextAdmissionError,
    AgenticAiContextManifestRecord, AgenticAiContextQuerySelectionRequest,
    AgenticAiContextRestoreRequest, restore_agentic_ai_context,
    select_agentic_ai_context_from_query,
};
use crate::{
    AgenticAiContextLimits, Binding, CandidateQueryResult, CatalogBoundQuery, QueryResultLimits,
    ReasoningBundle, SemanticSnapshot, VerifiedQueryResultTransport, admit_query_result_candidate,
    bind_query_to_catalog, export_query_result_transport, verify_query_result_transport,
};

pub const AGENTIC_AI_CONTEXT_QUERY_SELECTION_SCHEMA: &str =
    "mrr.agentic-ai-context.query-selection-record.v1";

/// Decoding this record grants no admission. Result bytes reuse the existing
/// versioned Query transport rather than introduce another row representation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgenticAiContextQuerySelectionRecord {
    pub schema: String,
    pub context: AgenticAiContextManifestRecord,
    pub context_digest: [u8; 32],
    pub column: Binding,
    pub result_transport: Vec<u8>,
}

/// Trusted expected selection identity and caller-owned resource ceilings.
/// The expected digest must come from an admitted reference, not this record.
pub struct AgenticAiContextQuerySelectionRestoreRequest<'a> {
    pub record: &'a AgenticAiContextQuerySelectionRecord,
    pub expected_digest: [u8; 32],
    pub limits: AgenticAiContextLimits,
    pub result_limits: QueryResultLimits,
    pub max_result_bytes: NonZeroUsize,
}

impl AdmittedAgenticAiContextQuerySelection {
    /// Recheck the exact candidate before exporting provenance. No storage I/O.
    pub fn export_record(
        &self,
        query: &CatalogBoundQuery,
        candidate: &CandidateQueryResult,
        limits: QueryResultLimits,
        max_result_bytes: NonZeroUsize,
    ) -> Result<AgenticAiContextQuerySelectionRecord, AgenticAiContextAdmissionError> {
        let receipt = admit_query_result_candidate(query, candidate, limits)
            .map_err(AgenticAiContextAdmissionError::QueryResult)?;
        if &receipt != self.result_receipt() {
            return Err(AgenticAiContextAdmissionError::SelectionRecordMismatch);
        }
        let result_transport =
            export_query_result_transport(query, candidate, limits, max_result_bytes)
                .map_err(AgenticAiContextAdmissionError::SelectionTransport)?;
        Ok(AgenticAiContextQuerySelectionRecord {
            schema: AGENTIC_AI_CONTEXT_QUERY_SELECTION_SCHEMA.into(),
            context: self.context().manifest().record().clone(),
            context_digest: *self.context().manifest().digest(),
            column: self.column().clone(),
            result_transport,
        })
    }
}

/// Re-admit source, catalog/snapshot/generation, transported rows, selection and
/// closure; then compare the complete selection identity with the trusted caller.
pub(super) fn restore_selection_parts(
    bundle: &ReasoningBundle,
    snapshot: &SemanticSnapshot,
    request: AgenticAiContextQuerySelectionRestoreRequest<'_>,
) -> Result<
    (
        AdmittedAgenticAiContextQuerySelection,
        CatalogBoundQuery,
        VerifiedQueryResultTransport,
    ),
    AgenticAiContextAdmissionError,
> {
    let record = request.record;
    if record.schema != AGENTIC_AI_CONTEXT_QUERY_SELECTION_SCHEMA {
        return Err(AgenticAiContextAdmissionError::SelectionRecordSchema);
    }
    // Reject byte inflation before reconstructing or decoding either payload.
    if record.result_transport.len() > request.max_result_bytes.get() {
        return Err(AgenticAiContextAdmissionError::SelectionTransport(
            crate::QueryResultTransportError::TooLarge {
                limit: request.max_result_bytes.get(),
                observed: record.result_transport.len(),
            },
        ));
    }
    let context = restore_agentic_ai_context(
        bundle,
        snapshot,
        AgenticAiContextRestoreRequest {
            record: &record.context,
            expected_digest: record.context_digest,
            limits: request.limits,
        },
    )?;
    let query = bind_query_to_catalog(bundle, record.context.query, snapshot)
        .map_err(AgenticAiContextAdmissionError::QueryBinding)?;
    let result = verify_query_result_transport(
        &query,
        &record.result_transport,
        request.result_limits,
        request.max_result_bytes,
    )
    .map_err(AgenticAiContextAdmissionError::SelectionTransport)?;
    let admitted = select_agentic_ai_context_from_query(
        bundle,
        snapshot,
        &query,
        result.candidate(),
        AgenticAiContextQuerySelectionRequest {
            column: record.column.clone(),
            contract: record.context.contract.clone(),
            dependencies: record.context.dependencies.clone(),
            limits: record.context.limits,
            result_limits: request.result_limits,
        },
    )?;
    if admitted.context() != &context || admitted.digest() != &request.expected_digest {
        return Err(AgenticAiContextAdmissionError::SelectionRecordMismatch);
    }
    Ok((admitted, query, result))
}

/// Re-admit a digest-referenced selection. Cryptographic collision resistance
/// remains the identity assumption for this compact-reference profile.
pub fn restore_agentic_ai_context_query_selection(
    bundle: &ReasoningBundle,
    snapshot: &SemanticSnapshot,
    request: AgenticAiContextQuerySelectionRestoreRequest<'_>,
) -> Result<AdmittedAgenticAiContextQuerySelection, AgenticAiContextAdmissionError> {
    restore_selection_parts(bundle, snapshot, request).map(|(selection, _, _)| selection)
}
