//! Public compiler facade for parsing, lowering, and sema analysis.
#![forbid(unsafe_code)]

mod api;

pub use api::{Compilation, Compiler, ParserOutput};
pub use gql_sema::{analyze, Analysis};
pub use gql_syntax::{parse, Parse};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
