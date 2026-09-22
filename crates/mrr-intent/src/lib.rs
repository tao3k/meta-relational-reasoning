//! Strict projection of one bounded Org intent vocabulary into MRR bundle selection.
#![forbid(unsafe_code)]
mod api;
pub use api::{
    IntentBindingStatus, IntentBundleBinding, IntentProjectionError, IntentSemanticModel,
};
#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
