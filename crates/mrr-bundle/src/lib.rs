// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Validated portable composition boundary for MRR contracts.
#![forbid(unsafe_code)]
mod api;
mod catalog;
mod entity_catalog;
pub use api::{
    BundleError, InverseGoal, LineagePolicy, ProjectionPolicy, QueryTemplate, ReasoningBundle,
    ReasoningBundleDeclaration, RulePack, TransitionSystem, ValidationProfile,
};
pub use catalog::{RelationCatalog, RelationCatalogDigest, RelationCatalogError};
pub use entity_catalog::{EntityCatalog, EntityCatalogDigest, EntityCatalogError};
pub use mrr_logic::Rule;
pub use mrr_relation::{EntitySchema, Fact, RelationError, RelationSchema};
pub use mrr_transition::Transition;
#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
