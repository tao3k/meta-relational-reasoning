//! Optional external-token binding and serving-prefix eligibility.

use super::{
    admission::AdmittedAgenticAiContextMaterialization, error::AgenticAiContextAdmissionError,
};
use crate::{AgenticAiContextError, ReasoningBundle, SemanticSnapshot};
use mrr_agentic_ai_context::{
    AgenticAiContextComputationalIdentity, AgenticAiContextReuseEligibility,
    AgenticAiContextTokenLayout, AgenticAiContextTokenizer,
};

/// Computational declaration and external adapter for one full-prompt encoding.
pub struct AgenticAiContextTokenizationRequest<'a, T: AgenticAiContextTokenizer> {
    pub identity: AgenticAiContextComputationalIdentity,
    pub tokenizer: &'a T,
}

/// Source/layout admission and adapter failures retain distinct typed causes.
#[derive(Debug)]
pub enum AgenticAiContextTokenizationError<E> {
    Admission(AgenticAiContextAdmissionError),
    Tokenizer(E),
}

impl<E: std::fmt::Display> std::fmt::Display for AgenticAiContextTokenizationError<E> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admission(error) => write!(formatter, "{error}"),
            Self::Tokenizer(error) => write!(formatter, "tokenizer: {error}"),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for AgenticAiContextTokenizationError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Admission(error) => Some(error),
            Self::Tokenizer(error) => Some(error),
        }
    }
}

/// Actual token IDs and configuration declared by the tokenizer/serving owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgenticAiContextTokenBindingRequest {
    pub identity: AgenticAiContextComputationalIdentity,
    pub tokens: Vec<u32>,
}

/// External token declaration tied to one exact source-bound presentation.
/// The external owner must establish that these tokens encode these bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceBoundAgenticAiContextTokens {
    presentation: AdmittedAgenticAiContextMaterialization,
    layout: AgenticAiContextTokenLayout,
}

impl AdmittedAgenticAiContextMaterialization {
    /// Consume this exact presentation and bind the owner's actual token IDs.
    pub fn bind_tokens(
        self,
        bundle: &ReasoningBundle,
        snapshot: &SemanticSnapshot,
        request: AgenticAiContextTokenBindingRequest,
    ) -> Result<SourceBoundAgenticAiContextTokens, AgenticAiContextAdmissionError> {
        self.check_source(bundle, snapshot)?;
        self.bind_checked_tokens(request)
    }

    /// Recheck source and adapter identity before encoding the exact full bytes.
    /// The tokenizer implementation remains external; no assets are loaded here.
    pub fn tokenize<T: AgenticAiContextTokenizer>(
        self,
        bundle: &ReasoningBundle,
        snapshot: &SemanticSnapshot,
        request: AgenticAiContextTokenizationRequest<'_, T>,
    ) -> Result<SourceBoundAgenticAiContextTokens, AgenticAiContextTokenizationError<T::Error>>
    {
        self.check_source(bundle, snapshot)
            .map_err(AgenticAiContextTokenizationError::Admission)?;
        request
            .identity
            .check_materialization(&self.materialization)
            .map_err(|error| {
                AgenticAiContextTokenizationError::Admission(
                    AgenticAiContextAdmissionError::Context(error),
                )
            })?;
        if request.tokenizer.identity() != request.identity.tokenizer {
            return Err(AgenticAiContextTokenizationError::Admission(
                AgenticAiContextAdmissionError::Context(
                    AgenticAiContextError::ComputationalIdentityMismatch,
                ),
            ));
        }
        let tokens = request
            .tokenizer
            .tokenize(self.materialization.bytes())
            .map_err(AgenticAiContextTokenizationError::Tokenizer)?;
        self.bind_checked_tokens(AgenticAiContextTokenBindingRequest {
            identity: request.identity,
            tokens,
        })
        .map_err(AgenticAiContextTokenizationError::Admission)
    }

    fn bind_checked_tokens(
        self,
        request: AgenticAiContextTokenBindingRequest,
    ) -> Result<SourceBoundAgenticAiContextTokens, AgenticAiContextAdmissionError> {
        let layout = AgenticAiContextTokenLayout::new(
            &self.materialization,
            request.identity,
            request.tokens,
        )
        .map_err(AgenticAiContextAdmissionError::Context)?;
        Ok(SourceBoundAgenticAiContextTokens {
            presentation: self,
            layout,
        })
    }
}

impl SourceBoundAgenticAiContextTokens {
    #[must_use]
    pub const fn presentation(&self) -> &AdmittedAgenticAiContextMaterialization {
        &self.presentation
    }

    #[must_use]
    pub const fn layout(&self) -> &AgenticAiContextTokenLayout {
        &self.layout
    }

    /// Recheck source binding before handing this token declaration to serving.
    pub fn check_source(
        &self,
        bundle: &ReasoningBundle,
        snapshot: &SemanticSnapshot,
    ) -> Result<(), AgenticAiContextAdmissionError> {
        self.presentation.check_source(bundle, snapshot)
    }

    /// Recheck the new source and compare actual declared tokens/configuration.
    /// The old layout is a historical computational basis, not current evidence.
    /// Returning eligibility does not certify a cache hit or authorize disclosure.
    pub fn reuse_eligibility_from(
        &self,
        old: &Self,
        bundle: &ReasoningBundle,
        snapshot: &SemanticSnapshot,
    ) -> Result<AgenticAiContextReuseEligibility, AgenticAiContextAdmissionError> {
        self.check_source(bundle, snapshot)?;
        AgenticAiContextReuseEligibility::between(&old.layout, &self.layout)
            .map_err(AgenticAiContextAdmissionError::Context)
    }
}
