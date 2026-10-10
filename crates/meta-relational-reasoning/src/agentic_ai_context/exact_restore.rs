//! Exact trusted-reference restoration without inferring equality from a hash.

use std::{num::NonZeroUsize, sync::Arc};

use super::{
    AdmittedAgenticAiContextQuerySelection, AgenticAiContextAdmissionError,
    AgenticAiContextQuerySelectionRecord, AgenticAiContextQuerySelectionRequest,
    AgenticAiContextQuerySelectionRestoreRequest, select_agentic_ai_context_from_query,
    selection_restore::restore_selection_parts,
};
use crate::{
    AgenticAiContextLimits, CandidateQueryResult, CatalogBoundQuery, QueryResultLimits,
    ReasoningBundle, SemanticSnapshot,
};

/// A sealed trusted reference retaining exact source, query and ordered result
/// contents. Hashes remain indexes; admission compares the retained contents.
/// Construction clones these bounded/admitted inputs once; cloning the sealed
/// reference itself shares them. It is not deserializable from untrusted storage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedExactAgenticAiContextQuerySelection {
    selection: AdmittedAgenticAiContextQuerySelection,
    source: Arc<ReasoningBundle>,
    query: Arc<CatalogBoundQuery>,
    candidate: Arc<CandidateQueryResult>,
}

/// The expected reference must originate from native admission and be retained
/// by the caller independently of the untrusted record being restored.
pub struct AgenticAiContextExactSelectionRestoreRequest<'a> {
    pub record: &'a AgenticAiContextQuerySelectionRecord,
    pub expected: &'a AdmittedExactAgenticAiContextQuerySelection,
    pub limits: AgenticAiContextLimits,
    pub result_limits: QueryResultLimits,
    pub max_result_bytes: NonZeroUsize,
}

pub fn select_agentic_ai_context_from_query_exact(
    bundle: &ReasoningBundle,
    snapshot: &SemanticSnapshot,
    query: &CatalogBoundQuery,
    candidate: &CandidateQueryResult,
    request: AgenticAiContextQuerySelectionRequest,
) -> Result<AdmittedExactAgenticAiContextQuerySelection, AgenticAiContextAdmissionError> {
    let selection =
        select_agentic_ai_context_from_query(bundle, snapshot, query, candidate, request)?;
    Ok(AdmittedExactAgenticAiContextQuerySelection {
        selection,
        source: Arc::new(bundle.clone()),
        query: Arc::new(query.clone()),
        candidate: Arc::new(candidate.clone()),
    })
}

impl AdmittedExactAgenticAiContextQuerySelection {
    #[must_use]
    pub const fn selection(&self) -> &AdmittedAgenticAiContextQuerySelection {
        &self.selection
    }

    /// Export only the exact candidate retained at admission.
    pub fn export_record(
        &self,
        limits: QueryResultLimits,
        max_result_bytes: NonZeroUsize,
    ) -> Result<AgenticAiContextQuerySelectionRecord, AgenticAiContextAdmissionError> {
        self.selection
            .export_record(&self.query, &self.candidate, limits, max_result_bytes)
    }
}

/// Current source/snapshot, decoded result, rebound query and reconstructed
/// Context must match exact trusted contents. Hash checks remain rejection
/// filters and never provide the final equality argument in this profile.
pub fn restore_agentic_ai_context_query_selection_exact(
    bundle: &ReasoningBundle,
    snapshot: &SemanticSnapshot,
    request: AgenticAiContextExactSelectionRestoreRequest<'_>,
) -> Result<AdmittedExactAgenticAiContextQuerySelection, AgenticAiContextAdmissionError> {
    let expected = request.expected;
    if bundle != expected.source.as_ref() {
        return Err(AgenticAiContextAdmissionError::SourceBundleMismatch);
    }
    if snapshot != expected.selection.context().state().snapshot() {
        return Err(AgenticAiContextAdmissionError::QueryBindingMismatch);
    }
    let (selection, query, result) = restore_selection_parts(
        bundle,
        snapshot,
        AgenticAiContextQuerySelectionRestoreRequest {
            record: request.record,
            expected_digest: *expected.selection.digest(),
            limits: request.limits,
            result_limits: request.result_limits,
            max_result_bytes: request.max_result_bytes,
        },
    )?;
    if selection != expected.selection
        || query != *expected.query
        || result.candidate() != expected.candidate.as_ref()
    {
        return Err(AgenticAiContextAdmissionError::SelectionRecordMismatch);
    }
    Ok(AdmittedExactAgenticAiContextQuerySelection {
        selection,
        source: Arc::clone(&expected.source),
        query: Arc::clone(&expected.query),
        candidate: Arc::clone(&expected.candidate),
    })
}
