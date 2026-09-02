//! Public semantic facade for query analysis.
#![forbid(unsafe_code)]

mod aggregate_analysis;
mod api;
mod data_management;
mod record_lowering;
mod type_inference;

pub use api::{Analysis, analyze};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
