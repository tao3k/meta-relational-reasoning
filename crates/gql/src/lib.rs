//! Pure ISO GQL facade. MRR evaluation is admitted through its own facade.
#![forbid(unsafe_code)]

mod api;
pub use api::{Compiler, ast, catalog, compiler, ir, sema, source, syntax, types};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
