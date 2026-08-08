//! ISO GQL facade that re-exports core modules and optional reasoning backends.
pub use gql_core::{ast, catalog, compiler, ir, sema, source, syntax, types, Compiler};
pub use gql_reasoning::DerivationResult;

#[cfg(feature = "ascent")]
pub use gql_ascent::AscentTransitiveClosure;
