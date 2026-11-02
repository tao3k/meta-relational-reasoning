//! Exact closure precedence, parent-first byte layout and immutable segment spans.

use std::collections::BTreeSet;

use mrr_identity::{FactId, QueryId};

use crate::{AgenticAiContextContract, AgenticAiContextError, AgenticAiContextState};

/// Caller-supplied semantic precedence over exactly the required closure.
/// A C4 producer owns the order: this type validates membership and uniqueness,
/// and never claims that a second Rust algorithm has checked C4 constraints.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgenticAiContextComposition {
    precedence: Vec<FactId>,
}

impl AgenticAiContextComposition {
    pub fn from_precedence(
        state: &AgenticAiContextState,
        precedence: Vec<FactId>,
    ) -> Result<Self, AgenticAiContextError> {
        let closure = state.required_closure();
        if precedence.len() != closure.elements().len() {
            return Err(AgenticAiContextError::InvalidPrecedence);
        }
        let actual: BTreeSet<_> = precedence.iter().copied().collect();
        if actual.len() != precedence.len()
            || actual.into_iter().collect::<Vec<_>>() != closure.elements()
        {
            return Err(AgenticAiContextError::InvalidPrecedence);
        }
        Ok(Self { precedence })
    }

    #[must_use]
    pub fn precedence(&self) -> &[FactId] {
        &self.precedence
    }

    pub fn parent_first(&self) -> impl Iterator<Item = FactId> + '_ {
        self.precedence.iter().rev().copied()
    }

    /// Only the exact one-child order relation; old payload stability is a
    /// separate obligation handled by materialization comparison.
    #[must_use]
    pub fn is_leaf_extension_of(&self, old: &Self, child: FactId) -> bool {
        self.precedence.first() == Some(&child)
            && self.precedence.get(1..) == Some(old.precedence.as_slice())
            && !old.precedence.contains(&child)
    }
}

/// Bytes produced by the external, versioned presentation owner for one element.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgenticAiContextRenderedElement {
    pub element: FactId,
    pub bytes: Vec<u8>,
}

/// Half-open byte span of one rendered element in the complete presentation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgenticAiContextSpan {
    pub element: FactId,
    pub start: usize,
    pub end: usize,
}

/// Concatenation of separately supplied immutable segments, with exact spans.
/// This checks physical layout, not whether an LLM obeys override semantics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgenticAiContextMaterialization {
    snapshot_digest: [u8; 32],
    query: QueryId,
    contract: AgenticAiContextContract,
    renderer_identity: String,
    composition: AgenticAiContextComposition,
    segments: Vec<AgenticAiContextRenderedElement>,
    spans: Vec<AgenticAiContextSpan>,
    bytes: Vec<u8>,
}

impl AgenticAiContextMaterialization {
    pub fn new(
        state: &AgenticAiContextState,
        composition: AgenticAiContextComposition,
        renderer_identity: String,
        segments: Vec<AgenticAiContextRenderedElement>,
    ) -> Result<Self, AgenticAiContextError> {
        if renderer_identity.trim().is_empty() {
            return Err(AgenticAiContextError::EmptyComputationalIdentity);
        }
        // Recheck against this exact state, even if the order was created for another.
        let composition =
            AgenticAiContextComposition::from_precedence(state, composition.precedence)?;
        if !composition
            .parent_first()
            .eq(segments.iter().map(|segment| segment.element))
        {
            return Err(AgenticAiContextError::RenderedElementMismatch);
        }
        let total = segments.iter().try_fold(0_usize, |total, segment| {
            total
                .checked_add(segment.bytes.len())
                .ok_or(AgenticAiContextError::RenderedByteBudget)
        })?;
        if total > state.limits().max_rendered_bytes.get() {
            return Err(AgenticAiContextError::RenderedByteBudget);
        }
        let mut bytes = Vec::with_capacity(total);
        let mut spans = Vec::with_capacity(segments.len());
        for segment in &segments {
            let start = bytes.len();
            bytes.extend_from_slice(&segment.bytes);
            spans.push(AgenticAiContextSpan {
                element: segment.element,
                start,
                end: bytes.len(),
            });
        }
        Ok(Self {
            snapshot_digest: *state.snapshot().digest(),
            query: state.query().id(),
            contract: state.contract().clone(),
            renderer_identity,
            composition,
            segments,
            spans,
            bytes,
        })
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    #[must_use]
    pub const fn snapshot_digest(&self) -> &[u8; 32] {
        &self.snapshot_digest
    }
    #[must_use]
    pub fn spans(&self) -> &[AgenticAiContextSpan] {
        &self.spans
    }
    #[must_use]
    pub fn renderer_identity(&self) -> &str {
        &self.renderer_identity
    }
    #[must_use]
    pub const fn query(&self) -> QueryId {
        self.query
    }
    #[must_use]
    pub const fn contract(&self) -> &AgenticAiContextContract {
        &self.contract
    }

    /// Exact byte append under unchanged scope and old rendered segments.
    /// It intentionally makes no claim about full-prompt tokenization.
    #[must_use]
    pub fn is_leaf_append_of(&self, old: &Self, child: FactId) -> bool {
        self.snapshot_digest == old.snapshot_digest
            && self.query == old.query
            && self.contract == old.contract
            && self.renderer_identity == old.renderer_identity
            && self
                .composition
                .is_leaf_extension_of(&old.composition, child)
            && self.segments.get(..old.segments.len()) == Some(old.segments.as_slice())
            && self.bytes.starts_with(&old.bytes)
    }
}
