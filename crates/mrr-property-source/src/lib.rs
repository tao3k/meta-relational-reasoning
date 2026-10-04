//! MRR-owned source compilation, catalog binding, and result admission.
#![forbid(unsafe_code)]

mod execution;
mod source_query;

pub use meta_relational_reasoning::{
    AdmittedPropertyExecution, PropertyExecutionCandidate, PropertyExecutionError,
    PropertyQueryBackend,
};

pub use mrr_frontends::ParserOwnedCompilationReceipt;
pub use source_query::{
    BoundPropertySourceQuery, CompiledPropertySourceQuery, PropertySourceQueryError,
    compile_property_source_query,
};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
