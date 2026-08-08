//! Public compiler facade for parsing, lowering, and sema analysis.
#![forbid(unsafe_code)]

mod api;

pub use api::{Compilation, Compiler, ParserOutput};
pub use gql_sema::{Analysis, analyze};
pub use gql_syntax::{Parse, parse};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
