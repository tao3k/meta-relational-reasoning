//! ISO GQL facade with optional reasoning backends.
#![forbid(unsafe_code)]

mod api;
pub use api::{
    Compiler, DerivationResult, ast, catalog, compiler, ir, sema, source, syntax, types,
};

#[cfg(feature = "ascent")]
pub use api::AscentTransitiveClosure;

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
