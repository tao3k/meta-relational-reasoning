//! Versioned selection manifests and untrusted storage records.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use mrr_identity::{FactId, GenerationId, QueryId, ReasoningBundleId};
use mrr_relation::EvidenceCompleteness;

use super::error::AgenticAiContextAdmissionError;
use crate::{
    AgenticAiContextContract, AgenticAiContextLimits, AgenticAiContextState, CatalogBoundQuery,
};

/// Domain and encoding version for deterministic CBOR manifest hashing.
pub const AGENTIC_AI_CONTEXT_MANIFEST_SCHEMA: &str = "mrr.agentic-ai-context.manifest.v1";

/// Candidate metadata supplied by storage. Decoding grants no source admission.
/// Fact payloads and schemas are bound by the canonical source bundle identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgenticAiContextManifestRecord {
    pub schema: String,
    pub source_bundle: ReasoningBundleId,
    pub generation: GenerationId,
    pub snapshot_digest: [u8; 32],
    pub query: QueryId,
    pub query_binding_digest: [u8; 32],
    pub roots: Vec<FactId>,
    pub contract: AgenticAiContextContract,
    pub dependencies: Vec<(FactId, Vec<FactId>)>,
    pub selected: Vec<FactId>,
    pub completeness: EvidenceCompleteness,
    pub limits: AgenticAiContextLimits,
}

/// Facade-created manifest over one exact admitted semantic selection.
/// This is a content identity, not a signature, policy grant or cache receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgenticAiContextManifest {
    digest: [u8; 32],
    record: AgenticAiContextManifestRecord,
}

impl AgenticAiContextManifest {
    pub(super) fn from_admitted_state(
        bundle: ReasoningBundleId,
        query: &CatalogBoundQuery,
        state: &AgenticAiContextState,
    ) -> Result<Self, AgenticAiContextAdmissionError> {
        let record = AgenticAiContextManifestRecord {
            schema: AGENTIC_AI_CONTEXT_MANIFEST_SCHEMA.into(),
            source_bundle: bundle,
            generation: state.snapshot().generation(),
            snapshot_digest: *state.snapshot().digest(),
            query: state.query().id(),
            query_binding_digest: *query.digest(),
            roots: state.query().roots().to_vec(),
            contract: state.contract().clone(),
            dependencies: state
                .elements()
                .iter()
                .map(|(id, element)| (*id, element.dependencies().to_vec()))
                .collect(),
            selected: state.required_closure().elements().to_vec(),
            completeness: state.required_closure().coverage(),
            limits: state.limits(),
        };
        let mut canonical = Vec::new();
        ciborium::into_writer(&record, &mut canonical)
            .map_err(|error| AgenticAiContextAdmissionError::ManifestEncoding(error.to_string()))?;
        Ok(Self {
            digest: Sha256::digest(&canonical).into(),
            record,
        })
    }

    #[must_use]
    pub const fn digest(&self) -> &[u8; 32] {
        &self.digest
    }

    /// Metadata may be persisted generically and must be re-admitted on restore.
    #[must_use]
    pub const fn record(&self) -> &AgenticAiContextManifestRecord {
        &self.record
    }
}
