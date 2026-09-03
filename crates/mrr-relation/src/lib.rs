//! Typed relations shared by every MRR domain.
#![forbid(unsafe_code)]

mod api;
pub use api::{
    EvidenceCompleteness, Fact, FactProvenance, FactValidity, RelationAuthority,
    RelationCardinality, RelationContext, RelationError, RelationField, RelationSchema, Value,
    ValueType,
};
pub use mrr_identity::{
    DerivationId, EntityId, FactId, GenerationId, RelationId, RuleId, RulePackId,
};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
