//! Source and query admission for the explicit fact-selection Context slice.

use std::{collections::BTreeMap, sync::Arc};

use super::composition_receipt::AgenticAiContextCompositionReceipt;
use super::{error::AgenticAiContextAdmissionError, manifest::AgenticAiContextManifest};

use mrr_agentic_ai_context::{
    AgenticAiContextClosure, AgenticAiContextComposition, AgenticAiContextContract,
    AgenticAiContextElement, AgenticAiContextError, AgenticAiContextLimits,
    AgenticAiContextMaterialization, AgenticAiContextQuery, AgenticAiContextRenderedElement,
    AgenticAiContextState, AgenticAiContextStateInput,
};
use mrr_identity::{FactId, ReasoningBundleId};

use crate::{CatalogBoundQuery, ReasoningBundle, SemanticSnapshot, bind_query_to_catalog};

/// The facade has rechecked the exact source bundle and catalog-bound query.
/// The selected view contains existing bundle facts, not newly inferred truth.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedAgenticAiContext {
    source: ContextSourceBinding,
    state: AgenticAiContextState,
    manifest: Arc<AgenticAiContextManifest>,
}

/// Facade-owned source binding shared by every admitted presentation stage.
#[derive(Clone, Debug, Eq, PartialEq)]
struct ContextSourceBinding {
    bundle: ReasoningBundleId,
    query: crate::QueryId,
    query_binding_digest: [u8; 32],
}

impl ContextSourceBinding {
    fn check(
        &self,
        bundle: &ReasoningBundle,
        snapshot: &SemanticSnapshot,
    ) -> Result<(), AgenticAiContextAdmissionError> {
        if bundle.id() != self.bundle {
            return Err(AgenticAiContextAdmissionError::SourceBundleMismatch);
        }
        let bound = bind_query_to_catalog(bundle, self.query, snapshot)
            .map_err(AgenticAiContextAdmissionError::QueryBinding)?;
        if bound.digest() != &self.query_binding_digest {
            return Err(AgenticAiContextAdmissionError::QueryBindingMismatch);
        }
        Ok(())
    }
}

/// External composition and rendering declarations checked against an admitted view.
/// Precedence is most-specific-first; rendered segments must be parent-first.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgenticAiContextMaterializationRequest {
    pub precedence: Vec<FactId>,
    pub renderer_identity: String,
    pub segments: Vec<AgenticAiContextRenderedElement>,
}

/// Ordered presentation that retains facade source admission.
/// This certifies binding/layout, not the external renderer's semantic correctness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedAgenticAiContextMaterialization {
    source: ContextSourceBinding,
    pub(super) materialization: AgenticAiContextMaterialization,
    composition_receipt: Option<Arc<AgenticAiContextCompositionReceipt>>,
    manifest: Arc<AgenticAiContextManifest>,
}

/// Consumer-owned selection and contract declarations for one source-bound query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgenticAiContextAdmissionRequest {
    pub contract: AgenticAiContextContract,
    pub roots: Vec<FactId>,
    pub dependencies: Vec<(FactId, Vec<FactId>)>,
    pub limits: AgenticAiContextLimits,
}

/// Admit explicit fact roots and their required closure for one existing query.
/// This neither executes GQL nor admits an Ascent/physical result candidate.
/// Dependencies are declared by the consumer and cannot name facts outside the
/// admitted bundle. The complete temporal/contract meanings remain upstream.
pub fn admit_agentic_ai_context(
    bundle: &ReasoningBundle,
    snapshot: &SemanticSnapshot,
    query: &CatalogBoundQuery,
    request: AgenticAiContextAdmissionRequest,
) -> Result<AdmittedAgenticAiContext, AgenticAiContextAdmissionError> {
    let AgenticAiContextAdmissionRequest {
        contract,
        roots,
        dependencies,
        limits,
    } = request;
    let rebound = bind_query_to_catalog(bundle, query.query().id(), snapshot)
        .map_err(AgenticAiContextAdmissionError::QueryBinding)?;
    if &rebound != query {
        return Err(AgenticAiContextAdmissionError::QueryBindingMismatch);
    }
    if dependencies.len() > limits.max_elements.get()
        || bundle.facts().len() > limits.max_elements.get()
    {
        return Err(AgenticAiContextAdmissionError::Context(
            AgenticAiContextError::ElementBudget,
        ));
    }
    let mut declared = BTreeMap::new();
    for (id, edges) in dependencies {
        if declared.insert(id, edges).is_some() {
            return Err(AgenticAiContextAdmissionError::Context(
                AgenticAiContextError::DuplicateElement(id),
            ));
        }
    }
    let elements = bundle
        .facts()
        .iter()
        .map(|fact| {
            AgenticAiContextElement::new(
                fact.clone(),
                declared.remove(&fact.id()).unwrap_or_default(),
            )
        })
        .collect();
    if let Some(id) = declared.keys().next() {
        return Err(AgenticAiContextAdmissionError::Context(
            AgenticAiContextError::UnknownElement(*id),
        ));
    }
    let state = AgenticAiContextState::validate(AgenticAiContextStateInput {
        snapshot: snapshot.clone(),
        query: AgenticAiContextQuery::new(query.query().id(), roots),
        contract,
        elements,
        limits,
    })
    .map_err(AgenticAiContextAdmissionError::Context)?;
    let manifest = AgenticAiContextManifest::from_admitted_state(bundle.id(), query, &state)?;
    Ok(AdmittedAgenticAiContext {
        source: ContextSourceBinding {
            bundle: bundle.id(),
            query: query.query().id(),
            query_binding_digest: *query.digest(),
        },
        state,
        manifest: Arc::new(manifest),
    })
}

impl AdmittedAgenticAiContext {
    #[must_use]
    pub fn manifest(&self) -> &AgenticAiContextManifest {
        &self.manifest
    }
    #[must_use]
    pub const fn source_bundle(&self) -> ReasoningBundleId {
        self.source.bundle
    }
    #[must_use]
    pub const fn state(&self) -> &AgenticAiContextState {
        &self.state
    }
    #[must_use]
    pub const fn closure(&self) -> &AgenticAiContextClosure {
        self.state.required_closure()
    }

    /// Storage restore/source replacement must re-enter facade admission.
    pub fn check_source(
        &self,
        bundle: &ReasoningBundle,
        snapshot: &SemanticSnapshot,
    ) -> Result<(), AgenticAiContextAdmissionError> {
        self.source.check(bundle, snapshot)
    }

    /// Recheck the source before admitting external precedence and rendered bytes.
    pub fn materialize(
        &self,
        bundle: &ReasoningBundle,
        snapshot: &SemanticSnapshot,
        request: AgenticAiContextMaterializationRequest,
    ) -> Result<AdmittedAgenticAiContextMaterialization, AgenticAiContextAdmissionError> {
        self.materialize_with_receipt(bundle, snapshot, request, None)
    }

    pub(super) fn materialize_with_receipt(
        &self,
        bundle: &ReasoningBundle,
        snapshot: &SemanticSnapshot,
        request: AgenticAiContextMaterializationRequest,
        receipt: Option<Arc<AgenticAiContextCompositionReceipt>>,
    ) -> Result<AdmittedAgenticAiContextMaterialization, AgenticAiContextAdmissionError> {
        self.check_source(bundle, snapshot)?;
        if let Some(receipt) = &receipt
            && (receipt.manifest_digest() != self.manifest.digest()
                || receipt.composition().precedence() != request.precedence)
        {
            return Err(AgenticAiContextAdmissionError::CompositionMismatch);
        }
        let composition =
            AgenticAiContextComposition::from_precedence(&self.state, request.precedence)
                .map_err(AgenticAiContextAdmissionError::Context)?;
        let materialization = AgenticAiContextMaterialization::new(
            &self.state,
            composition,
            request.renderer_identity,
            request.segments,
        )
        .map_err(AgenticAiContextAdmissionError::Context)?;
        Ok(AdmittedAgenticAiContextMaterialization {
            source: self.source.clone(),
            materialization,
            composition_receipt: receipt,
            manifest: Arc::clone(&self.manifest),
        })
    }
}

impl AdmittedAgenticAiContextMaterialization {
    #[must_use]
    pub fn composition_receipt(&self) -> Option<&AgenticAiContextCompositionReceipt> {
        self.composition_receipt.as_deref()
    }
    #[must_use]
    pub fn manifest(&self) -> &AgenticAiContextManifest {
        &self.manifest
    }
    #[must_use]
    pub const fn source_bundle(&self) -> ReasoningBundleId {
        self.source.bundle
    }

    #[must_use]
    pub const fn materialization(&self) -> &AgenticAiContextMaterialization {
        &self.materialization
    }

    /// Check the current source before using stored presentation bytes.
    /// This does not perform a policy/authorization observation.
    pub fn check_source(
        &self,
        bundle: &ReasoningBundle,
        snapshot: &SemanticSnapshot,
    ) -> Result<(), AgenticAiContextAdmissionError> {
        self.source.check(bundle, snapshot)
    }
}
