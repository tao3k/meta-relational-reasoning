// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

//! External revision and MRR generation binding contracts.
#![forbid(unsafe_code)]
mod api;
mod snapshot;
pub use api::{ExternalRevisionIdentity, RevisionBinding, RevisionBindingError};
pub use mrr_identity::{GenerationId, RevisionId, StateId};
pub use snapshot::{SemanticSnapshot, SemanticSnapshotError};
#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
