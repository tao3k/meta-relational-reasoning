// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Typed stable identities shared by every MRR layer.
#![forbid(unsafe_code)]
mod api;
pub use api::{
    ActionId, DerivationId, EntityId, FactId, GenerationId, IDENTITY_SCHEMA, IdentityDomain,
    IdentityError, LineageEdgeId, LineageNodeId, QueryId, QueryOperatorId, ReasoningBundleId,
    RelationId, RevisionId, RuleId, RulePackId, StateId, TransitionId,
};
#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
