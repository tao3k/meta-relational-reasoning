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
