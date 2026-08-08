//! Public semantic facade for query analysis.
#![forbid(unsafe_code)]

mod api;

pub use api::{analyze, Analysis};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
