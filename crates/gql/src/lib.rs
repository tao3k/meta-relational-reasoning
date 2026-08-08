//! ISO GQL facade with optional reasoning backends.
#![forbid(unsafe_code)]

mod api;
pub use api::{ast, catalog, compiler, DerivationResult, ir, sema, source, syntax, types, Compiler};

#[cfg(feature = "ascent")]
pub use api::AscentTransitiveClosure;

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
