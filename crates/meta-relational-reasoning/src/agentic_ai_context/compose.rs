//! Source-checked invocation of external composition owners.
use super::{
    admission::{
        AdmittedAgenticAiContext, AdmittedAgenticAiContextMaterialization,
        AgenticAiContextMaterializationRequest,
    },
    composition_receipt::AgenticAiContextCompositionReceipt,
    error::AgenticAiContextAdmissionError,
};
use crate::{ReasoningBundle, SemanticSnapshot};
use mrr_agentic_ai_context::{
    AgenticAiContextComposer, AgenticAiContextCompositionGraph,
    AgenticAiContextCompositionGraphInput, AgenticAiContextCompositionProducer,
    AgenticAiContextRenderedElement,
};
use std::{fmt, sync::Arc};

pub struct AgenticAiContextCompositionRequest<'a, T: AgenticAiContextComposer> {
    pub graph: AgenticAiContextCompositionGraphInput,
    pub expected_producer: AgenticAiContextCompositionProducer,
    pub composer: &'a T,
}
#[derive(Debug)]
pub enum AgenticAiContextCompositionError<E> {
    Admission(AgenticAiContextAdmissionError),
    Composer(E),
}
impl<E: fmt::Display> fmt::Display for AgenticAiContextCompositionError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Admission(e) => write!(f, "{e}"),
            Self::Composer(e) => write!(f, "composer: {e}"),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for AgenticAiContextCompositionError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Admission(e) => Some(e),
            Self::Composer(e) => Some(e),
        }
    }
}
/// Rendering segments supplied in the receipt's parent-first order.
pub struct AgenticAiContextComposedMaterializationRequest<'a> {
    pub composition: &'a AgenticAiContextCompositionReceipt,
    pub renderer_identity: String,
    pub segments: Vec<AgenticAiContextRenderedElement>,
}
impl AdmittedAgenticAiContext {
    pub fn compose<T: AgenticAiContextComposer>(
        &self,
        bundle: &ReasoningBundle,
        snapshot: &SemanticSnapshot,
        request: AgenticAiContextCompositionRequest<'_, T>,
    ) -> Result<AgenticAiContextCompositionReceipt, AgenticAiContextCompositionError<T::Error>>
    {
        self.check_source(bundle, snapshot)
            .map_err(AgenticAiContextCompositionError::Admission)?;
        let map = |e| {
            AgenticAiContextCompositionError::Admission(AgenticAiContextAdmissionError::Context(e))
        };
        request.expected_producer.validate().map_err(map)?;
        if request.composer.identity() != &request.expected_producer {
            return Err(AgenticAiContextCompositionError::Admission(
                AgenticAiContextAdmissionError::CompositionProducerMismatch,
            ));
        }
        let graph =
            AgenticAiContextCompositionGraph::validate(self.state(), request.graph).map_err(map)?;
        let output = request
            .composer
            .compose(&graph)
            .map_err(AgenticAiContextCompositionError::Composer)?;
        let composition = graph.check_result(self.state(), output).map_err(map)?;
        AgenticAiContextCompositionReceipt::new(
            *self.manifest().digest(),
            graph,
            request.expected_producer,
            composition,
        )
        .map_err(AgenticAiContextCompositionError::Admission)
    }
    pub fn materialize_composed(
        &self,
        bundle: &ReasoningBundle,
        snapshot: &SemanticSnapshot,
        request: AgenticAiContextComposedMaterializationRequest<'_>,
    ) -> Result<AdmittedAgenticAiContextMaterialization, AgenticAiContextAdmissionError> {
        let materialization = AgenticAiContextMaterializationRequest {
            precedence: request.composition.composition().precedence().to_vec(),
            renderer_identity: request.renderer_identity,
            segments: request.segments,
        };
        self.materialize_with_receipt(
            bundle,
            snapshot,
            materialization,
            Some(Arc::new(request.composition.clone())),
        )
    }
}
