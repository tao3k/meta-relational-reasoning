//! Result-driven selection of existing source facts. Candidate admission checks
//! identity and types; it does not prove physical query execution correctness.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

use super::{
    AdmittedAgenticAiContext, AgenticAiContextAdmissionError, AgenticAiContextAdmissionRequest,
    admit_agentic_ai_context,
};
use crate::{
    AgenticAiContextContract, AgenticAiContextLimits, Binding, CandidateQueryResult,
    CatalogBoundQuery, FactId, QueryResultAdmissionReceipt, QueryResultLimits, QueryResultValue,
    ReasoningBundle, SemanticSnapshot, admit_query_result_candidate, bind_query_to_catalog,
};

/// Select the relation identities in one named result column. Dependencies and
/// mandatory contract facts retain the existing finite closure semantics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgenticAiContextQuerySelectionRequest {
    pub column: Binding,
    pub contract: AgenticAiContextContract,
    pub dependencies: Vec<(FactId, Vec<FactId>)>,
    pub limits: AgenticAiContextLimits,
    pub result_limits: QueryResultLimits,
}

/// Exact candidate provenance paired with a source-admitted semantic Context.
/// The private fields prevent callers from attaching an unrelated result receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedAgenticAiContextQuerySelection {
    context: AdmittedAgenticAiContext,
    result: QueryResultAdmissionReceipt,
    column: Binding,
    digest: [u8; 32],
}

pub fn select_agentic_ai_context_from_query(
    bundle: &ReasoningBundle,
    snapshot: &SemanticSnapshot,
    query: &CatalogBoundQuery,
    candidate: &CandidateQueryResult,
    request: AgenticAiContextQuerySelectionRequest,
) -> Result<AdmittedAgenticAiContextQuerySelection, AgenticAiContextAdmissionError> {
    let rebound = bind_query_to_catalog(bundle, query.query().id(), snapshot)
        .map_err(AgenticAiContextAdmissionError::QueryBinding)?;
    if &rebound != query {
        return Err(AgenticAiContextAdmissionError::QueryBindingMismatch);
    }
    let result = admit_query_result_candidate(query, candidate, request.result_limits)
        .map_err(AgenticAiContextAdmissionError::QueryResult)?;
    let column = candidate
        .columns()
        .iter()
        .position(|name| name == &request.column)
        .ok_or_else(|| AgenticAiContextAdmissionError::SelectionColumn(request.column.clone()))?;
    if !matches!(
        query.static_typing().result_fields()[column]
            .expression_type()
            .query_type(),
        crate::QueryType::Relation(_)
    ) {
        return Err(AgenticAiContextAdmissionError::SelectionColumn(
            request.column,
        ));
    }
    if bundle.facts().len() > request.limits.max_elements.get() {
        return Err(AgenticAiContextAdmissionError::Context(
            crate::AgenticAiContextError::ElementBudget,
        ));
    }
    let facts: BTreeMap<_, _> = bundle
        .facts()
        .iter()
        .map(|fact| (fact.id(), fact))
        .collect();
    let mut roots = BTreeSet::new();
    for (row, values) in candidate.rows().iter().enumerate() {
        let QueryResultValue::Relation { id, relation_type } = &values[column] else {
            return Err(AgenticAiContextAdmissionError::SelectionValue { row });
        };
        if facts
            .get(id)
            .is_none_or(|fact| fact.relation() != *relation_type)
        {
            return Err(AgenticAiContextAdmissionError::SelectionFactMismatch(*id));
        }
        roots.insert(*id);
    }
    let context = admit_agentic_ai_context(
        bundle,
        snapshot,
        query,
        AgenticAiContextAdmissionRequest {
            contract: request.contract,
            roots: roots.into_iter().collect(),
            dependencies: request.dependencies,
            limits: request.limits,
        },
    )?;
    let mut encoded = Vec::new();
    ciborium::into_writer(
        &(
            "mrr.agentic-ai-context.query-selection.v1",
            context.manifest().digest(),
            result.digest(),
            &request.column,
        ),
        &mut encoded,
    )
    .map_err(|error| AgenticAiContextAdmissionError::ManifestEncoding(error.to_string()))?;
    Ok(AdmittedAgenticAiContextQuerySelection {
        context,
        result,
        column: request.column,
        digest: Sha256::digest(encoded).into(),
    })
}

impl AdmittedAgenticAiContextQuerySelection {
    #[must_use]
    pub const fn context(&self) -> &AdmittedAgenticAiContext {
        &self.context
    }
    #[must_use]
    pub const fn result_receipt(&self) -> &QueryResultAdmissionReceipt {
        &self.result
    }
    #[must_use]
    pub const fn column(&self) -> &Binding {
        &self.column
    }
    #[must_use]
    pub const fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
    /// Source replacement, snapshot/catalog or generation drift must re-enter admission.
    pub fn check_source(
        &self,
        bundle: &ReasoningBundle,
        snapshot: &SemanticSnapshot,
    ) -> Result<(), AgenticAiContextAdmissionError> {
        self.context.check_source(bundle, snapshot)
    }
}
