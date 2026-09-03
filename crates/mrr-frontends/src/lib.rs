//! GQL and Cypher frontend adapters into the shared `MetaQueryIr` contract.
#![forbid(unsafe_code)]

mod lowering;
mod value_type_identity;

pub use lowering::{FrontendError, QueryFrontend, QueryLanguage};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
