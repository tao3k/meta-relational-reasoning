// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Typed relations shared by every MRR domain.
#![forbid(unsafe_code)]

mod api;
mod validation;
pub use api::{
    EntitySchema, EvidenceCompleteness, Fact, FactProvenance, FactValidity, FloatWidth,
    RelationAuthority, RelationConstraint, RelationContext, RelationContextError, RelationError,
    RelationField, RelationSchema, TemporalUnit, TimezonePolicy, Value, ValueKind, ValueSchema,
};
pub use mrr_identity::{
    DerivationId, EntityId, FactId, GenerationId, RelationId, RuleId, RulePackId,
};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
