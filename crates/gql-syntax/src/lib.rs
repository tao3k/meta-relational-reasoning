//! Frontend syntax crate facade for ISO GQL parse trees.
//!
//! This crate owns the parse entrypoint and its CST-facing public AST/syntax
//! types while delegating implementation details to internal modules.

#![forbid(unsafe_code)]

mod generated;
mod lexer;
mod parser;
mod syntax;

pub use parser::parse;
pub use syntax::{
    GqlSyntax, GrammarProjectionReceipt, Keyword, Parse, SyntaxElement, SyntaxElementKind,
    SyntaxKind, SyntaxNode, SyntaxTree, Token, TokenKind, grammar_projection_receipt,
};

pub use syntax::{RowanSyntaxNode, RowanSyntaxToken};

#[cfg(test)]
#[path = "../tests/unit/contracts.rs"]
mod tests;
