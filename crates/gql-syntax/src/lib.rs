//! Frontend syntax crate façade for ISO GQL parse trees.
//!
//! This crate owns the parse entrypoint and its CST-facing public AST/syntax
//! types while delegating implementation details to internal modules.

#![forbid(unsafe_code)]

mod lexer;
mod parser;
mod rowan_build;
mod syntax;

pub use parser::parse;
pub use syntax::{
    GqlSyntax, Keyword, Parse, SyntaxElement, SyntaxElementKind, SyntaxKind, SyntaxNode,
    SyntaxTree, Token, TokenKind,
};

pub use syntax::{RowanSyntaxNode, RowanSyntaxToken};

#[cfg(test)]
#[path = "../tests/unit/unit.rs"]
mod tests;
