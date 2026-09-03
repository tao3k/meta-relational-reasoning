//! External revision and MRR generation binding contracts.
#![forbid(unsafe_code)]
mod api;
pub use api::{ExternalRevisionIdentity, RevisionBinding, RevisionBindingError};
pub use mrr_identity::{GenerationId, RevisionId, StateId};
#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
