//! Frontend syntax crate facade for ISO GQL parse trees.
//!
//! This crate owns the parse entrypoint and its CST-facing public AST/syntax
//! types while delegating implementation details to internal modules.

#![forbid(unsafe_code)]

mod character_string;
mod generated;
mod lexer;
mod parameter;
mod parser;
mod syntax;

pub use character_string::{CharacterStringForm, DecodedCharacterString, decode_character_string};
pub use parameter::{DecodedParameterReference, ParameterNameForm, decode_parameter_reference};
pub use parser::parse;
pub use syntax::{
    GqlSyntax, GrammarProjectionReceipt, ISO_GQL_AGGREGATE_FUNCTION_FORMS,
    ISO_GQL_CHARACTER_STRING_FORMS, ISO_GQL_NON_RESERVED_WORDS, ISO_GQL_NUMERIC_LITERAL_FORMS,
    ISO_GQL_PARAMETER_REFERENCE_FORMS, ISO_GQL_PREDICATE_TEST_FORMS, Keyword, Parse, SyntaxElement,
    SyntaxElementKind, SyntaxKind, SyntaxNode, SyntaxTree, Token, TokenKind,
    grammar_projection_receipt, is_non_reserved_word,
};

pub use syntax::{RowanSyntaxNode, RowanSyntaxToken};

#[cfg(test)]
#[path = "../tests/unit/contracts.rs"]
mod tests;
