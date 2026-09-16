//! Typed relations shared by every MRR domain.
#![forbid(unsafe_code)]

mod api;
mod validation;
pub use api::{
    EvidenceCompleteness, Fact, FactProvenance, FactValidity, FloatWidth, RelationAuthority,
    RelationConstraint, RelationContext, RelationContextError, RelationError, RelationField,
    RelationSchema, TemporalUnit, TimezonePolicy, Value, ValueKind, ValueSchema,
};
pub use mrr_identity::{
    DerivationId, EntityId, FactId, GenerationId, RelationId, RuleId, RulePackId,
};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
