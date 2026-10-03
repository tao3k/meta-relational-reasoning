//! Source-checked semantic revision receipts for an exact pair of manifests.
use super::{admission::AdmittedAgenticAiContext, error::AgenticAiContextAdmissionError};
use crate::{ReasoningBundle, SemanticSnapshot};
use mrr_agentic_ai_context::AgenticAiContextRevision;
use sha2::{Digest, Sha256};

pub struct AgenticAiContextRevisionRequest<'a> {
    pub old: &'a AdmittedAgenticAiContext,
    pub new: &'a AdmittedAgenticAiContext,
    pub old_bundle: &'a ReasoningBundle,
    pub old_snapshot: &'a SemanticSnapshot,
    pub new_bundle: &'a ReasoningBundle,
    pub new_snapshot: &'a SemanticSnapshot,
}
/// Semantic impact and reuse only; no byte, token, policy or cache grant.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgenticAiContextRevisionReceipt {
    from_manifest: [u8; 32],
    to_manifest: [u8; 32],
    query_binding_changed: bool,
    revision: AgenticAiContextRevision,
    digest: [u8; 32],
}
pub fn compare_agentic_ai_context_revision(
    request: AgenticAiContextRevisionRequest<'_>,
) -> Result<AgenticAiContextRevisionReceipt, AgenticAiContextAdmissionError> {
    request
        .old
        .check_source(request.old_bundle, request.old_snapshot)?;
    request
        .new
        .check_source(request.new_bundle, request.new_snapshot)?;
    let from_manifest = *request.old.manifest().digest();
    let to_manifest = *request.new.manifest().digest();
    let query_binding_changed = request.old.manifest().record().query_binding_digest
        != request.new.manifest().record().query_binding_digest;
    let revision = if query_binding_changed {
        AgenticAiContextRevision::invalidating_all(request.old.state(), request.new.state())
    } else {
        AgenticAiContextRevision::between(request.old.state(), request.new.state())
    };
    let mut encoded = Vec::new();
    ciborium::into_writer(
        &(
            "mrr.agentic-ai-context.revision.v1",
            from_manifest,
            to_manifest,
            query_binding_changed,
            revision.changed(),
            revision.invalidated(),
            revision.reusable().elements(),
        ),
        &mut encoded,
    )
    .map_err(|e| AgenticAiContextAdmissionError::RevisionEncoding(e.to_string()))?;
    Ok(AgenticAiContextRevisionReceipt {
        from_manifest,
        to_manifest,
        query_binding_changed,
        revision,
        digest: Sha256::digest(encoded).into(),
    })
}
impl AgenticAiContextRevisionReceipt {
    #[must_use]
    pub const fn from_manifest(&self) -> &[u8; 32] {
        &self.from_manifest
    }
    #[must_use]
    pub const fn to_manifest(&self) -> &[u8; 32] {
        &self.to_manifest
    }
    #[must_use]
    pub const fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
    #[must_use]
    pub const fn query_binding_changed(&self) -> bool {
        self.query_binding_changed
    }
    #[must_use]
    pub const fn revision(&self) -> &AgenticAiContextRevision {
        &self.revision
    }
    pub fn check_contexts(
        &self,
        old: &AdmittedAgenticAiContext,
        new: &AdmittedAgenticAiContext,
    ) -> Result<(), AgenticAiContextAdmissionError> {
        if old.manifest().digest() != &self.from_manifest
            || new.manifest().digest() != &self.to_manifest
        {
            return Err(AgenticAiContextAdmissionError::RevisionMismatch);
        }
        Ok(())
    }
}
