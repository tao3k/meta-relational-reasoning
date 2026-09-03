//! Generated grammar projection interface.
mod aggregate_forms;
mod lexical_forms;
mod parser_forms;
mod projection;

pub(crate) use aggregate_forms::aggregate_function_spec;
pub use lexical_forms::{
    ISO_GQL_AGGREGATE_FUNCTION_FORMS, ISO_GQL_CHARACTER_STRING_FORMS, ISO_GQL_NON_RESERVED_WORDS,
    ISO_GQL_NUMERIC_LITERAL_FORMS, ISO_GQL_PARAMETER_REFERENCE_FORMS, ISO_GQL_PREDICATE_TEST_FORMS,
    is_non_reserved_word,
};
pub(crate) use parser_forms::{
    GRAMMAR_RECOVERIES, GrammarParserAction, binary_operator_spec, prefix_operator_precedence,
    recovery_diagnostic, top_level_parser_entrypoint,
};
pub(crate) use projection::{
    GERBIL_SCHEME_RUST_REVISION, GRAMMAR_PROJECTION_SCHEMA, GRAMMAR_SYNTAX_SHAPES, keyword,
};
pub use projection::{Keyword, SyntaxKind};
