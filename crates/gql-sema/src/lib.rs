//! Public semantic facade for query analysis.
#![forbid(unsafe_code)]

mod aggregate_analysis;
mod api;
mod binding_analysis;
mod data_management;
mod order_page_semantics;
mod path_semantics;
mod predicate_lowering;
mod primitive_query_semantics;
mod query_clause;
mod record_lowering;
mod result_semantics;
mod type_inference;

pub use api::{Analysis, analyze};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
