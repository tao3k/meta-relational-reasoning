//! Abstract syntax model and lowering entrypoint for GQL source trees.
#![forbid(unsafe_code)]

mod lowering;
mod types;

pub use lowering::lower_from_syntax;
pub use types::{
    BinaryOperator, CatalogStatement, DataStatement, EdgeDirection, EdgePattern, Expression,
    GraphPattern, Identifier, MatchClause, NodePattern, PathPattern, PatternElement, Query,
    QueryClause, Statement, SyntaxParseOutput, UnaryOperator,
};
