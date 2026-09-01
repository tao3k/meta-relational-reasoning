//! Build-time Gerbil native projection and provenance admission.
//!
//! This crate is intentionally absent from the query runtime dependency graph.
//! It consumes a Gerbil AOT artifact produced by `build.ss`; no Scheme runtime
//! is started from a query hot path.

mod cli;
mod native;
mod projection;
mod projection_renderer;

pub use cli::run_cli;
pub use native::{ReasoningBundleLoadError, load_reasoning_bundle};
pub use projection::{
    GrammarProjectionError, stamp_projection, validate_projection, workspace_input_fingerprint,
};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
