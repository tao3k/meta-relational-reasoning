//! MRR-owned source-bound property query integration for physical executors.
#![forbid(unsafe_code)]

mod source_query;

pub use mrr_frontends::ParserOwnedCompilationReceipt;
pub use source_query::{
    BoundPropertySourceQuery, CompiledPropertySourceQuery, PropertySourceQueryError,
    compile_property_source_query,
};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
