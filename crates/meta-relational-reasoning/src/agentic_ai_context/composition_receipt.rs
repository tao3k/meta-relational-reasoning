//! Sealed composition receipts binding one manifest, graph and producer result.
use super::error::AgenticAiContextAdmissionError;
use mrr_agentic_ai_context::{
    AgenticAiContextComposition, AgenticAiContextCompositionGraph,
    AgenticAiContextCompositionProducer,
};
use sha2::{Digest, Sha256};

/// Binding and bounded order checks; not a proof of the producer's C4 implementation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgenticAiContextCompositionReceipt {
    manifest_digest: [u8; 32],
    graph: AgenticAiContextCompositionGraph,
    producer: AgenticAiContextCompositionProducer,
    composition: AgenticAiContextComposition,
    digest: [u8; 32],
}
impl AgenticAiContextCompositionReceipt {
    pub(super) fn new(
        manifest_digest: [u8; 32],
        graph: AgenticAiContextCompositionGraph,
        producer: AgenticAiContextCompositionProducer,
        composition: AgenticAiContextComposition,
    ) -> Result<Self, AgenticAiContextAdmissionError> {
        let mut encoded = Vec::new();
        ciborium::into_writer(
            &(
                "mrr.agentic-ai-context.composition.v1",
                manifest_digest,
                graph.input(),
                &producer,
                composition.precedence(),
            ),
            &mut encoded,
        )
        .map_err(|e| AgenticAiContextAdmissionError::CompositionEncoding(e.to_string()))?;
        Ok(Self {
            manifest_digest,
            graph,
            producer,
            composition,
            digest: Sha256::digest(encoded).into(),
        })
    }
    #[must_use]
    pub const fn manifest_digest(&self) -> &[u8; 32] {
        &self.manifest_digest
    }
    #[must_use]
    pub const fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
    #[must_use]
    pub const fn graph(&self) -> &AgenticAiContextCompositionGraph {
        &self.graph
    }
    #[must_use]
    pub const fn producer(&self) -> &AgenticAiContextCompositionProducer {
        &self.producer
    }
    #[must_use]
    pub const fn composition(&self) -> &AgenticAiContextComposition {
        &self.composition
    }
}
