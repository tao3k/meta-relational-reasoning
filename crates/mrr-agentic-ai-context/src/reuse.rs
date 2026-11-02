//! Compatible computational declarations and actual-token prefix eligibility.

use std::num::NonZeroUsize;

use mrr_identity::QueryId;

use crate::{AgenticAiContextContract, AgenticAiContextError, AgenticAiContextMaterialization};

/// External owner that encodes one complete presentation under a fixed identity.
/// The implementation owns the model-specific algorithm, vocabulary and assets.
/// It must encode the full prompt; concatenated per-segment tokens are insufficient.
pub trait AgenticAiContextTokenizer {
    type Error: std::error::Error + 'static;

    /// Versioned tokenizer identity matching the computational declaration.
    fn identity(&self) -> &str;

    /// Encode the exact full-presentation bytes without inserting hidden inputs.
    fn tokenize(&self, presentation: &[u8]) -> Result<Vec<u32>, Self::Error>;
}

/// Exact runtime configuration declarations required before token-prefix reuse
/// may be considered. The serving owner supplies and authenticates these values.
/// Equal labels are a contract, not a proof about model weights or GPU tensors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgenticAiContextComputationalIdentity {
    pub model_weights: String,
    pub tokenizer: String,
    pub renderer: String,
    pub chat_template: String,
    pub position_scheme: String,
    pub attention_semantics: String,
    pub cache_format: String,
    pub adapter_digest: [u8; 32],
    pub multimodal_digest: [u8; 32],
    pub sharing_scope: [u8; 32],
    pub block_tokens: NonZeroUsize,
}

impl AgenticAiContextComputationalIdentity {
    /// Check declarations before invoking a tokenizer or accepting token output.
    pub fn check_materialization(
        &self,
        materialization: &AgenticAiContextMaterialization,
    ) -> Result<(), AgenticAiContextError> {
        if [
            &self.model_weights,
            &self.tokenizer,
            &self.renderer,
            &self.chat_template,
            &self.position_scheme,
            &self.attention_semantics,
            &self.cache_format,
        ]
        .iter()
        .any(|value| value.trim().is_empty())
        {
            return Err(AgenticAiContextError::EmptyComputationalIdentity);
        }
        if self.renderer != materialization.renderer_identity() {
            return Err(AgenticAiContextError::ComputationalIdentityMismatch);
        }
        Ok(())
    }
}

/// Actual full-prompt token IDs supplied by the tokenizer owner.
/// There is no assumption that byte-prefix extension preserves these IDs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgenticAiContextTokenLayout {
    identity: AgenticAiContextComputationalIdentity,
    query: QueryId,
    contract: AgenticAiContextContract,
    tokens: Vec<u32>,
}

impl AgenticAiContextTokenLayout {
    pub fn new(
        materialization: &AgenticAiContextMaterialization,
        identity: AgenticAiContextComputationalIdentity,
        tokens: Vec<u32>,
    ) -> Result<Self, AgenticAiContextError> {
        identity.check_materialization(materialization)?;
        Ok(Self {
            identity,
            query: materialization.query(),
            contract: materialization.contract().clone(),
            tokens,
        })
    }

    #[must_use]
    pub fn tokens(&self) -> &[u32] {
        &self.tokens
    }
}

/// Maximum common-prefix/full-block eligibility under compatible declarations.
/// Actual cache hits, eviction and recomputation are serving-runtime receipts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgenticAiContextReuseEligibility {
    stable_token_prefix: usize,
    eligible_full_block_tokens: usize,
}

impl AgenticAiContextReuseEligibility {
    pub fn between(
        old: &AgenticAiContextTokenLayout,
        new: &AgenticAiContextTokenLayout,
    ) -> Result<Self, AgenticAiContextError> {
        if old.identity != new.identity || old.query != new.query || old.contract != new.contract {
            return Err(AgenticAiContextError::ComputationalIdentityMismatch);
        }
        let stable_token_prefix = old
            .tokens
            .iter()
            .zip(&new.tokens)
            .take_while(|(old, new)| old == new)
            .count();
        let block = old.identity.block_tokens.get();
        Ok(Self {
            stable_token_prefix,
            eligible_full_block_tokens: stable_token_prefix / block * block,
        })
    }

    #[must_use]
    pub const fn stable_token_prefix(&self) -> usize {
        self.stable_token_prefix
    }
    #[must_use]
    pub const fn eligible_full_block_tokens(&self) -> usize {
        self.eligible_full_block_tokens
    }
}
