//! Public abstract syntax model and lowering entrypoint for GQL.
#![forbid(unsafe_code)]

mod api;

pub use api::{
    BinaryOperator, CatalogStatement, DataStatement, EdgeDirection, EdgePattern, Expression,
    GraphPattern, Identifier, MatchClause, NodePattern, PathPattern, PatternElement, Query,
    QueryClause, Statement, SyntaxParseOutput, UnaryOperator, lower_from_syntax,
};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
