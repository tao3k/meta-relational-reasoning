//! Public semantic facade for query analysis.
#![forbid(unsafe_code)]

mod api;

pub use api::{Analysis, analyze};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
