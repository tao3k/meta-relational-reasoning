// mrr.gerbil-grammar-projection.v1 input-sha256=6ea6b2df8eb55b77191e0b6bf110b75af1f1d399a12735c0858a0c66624d7dfc body-sha256=bbd18ac3e80e7dc3061f15736f8dfcca247d9c8b536228967daaf28920f2d42f gerbil-scheme-rust-rev=a83fb649ddbbeaabdb538a6eaf0ded10838f7fad
// Generated through the Gerbil native AOT bindings; do not edit.
//! Parser grammar forms projected from the Gerbil grammar authority.
use super::projection::Keyword;
use crate::syntax::TokenKind;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GrammarParserAction {
    MatchClause,
    OptionalMatchClause,
    ReturnClause,
    FinishStatement,
    WhereClause,
    LetClause,
    FilterStatement,
    ForStatement,
    UnionClause,
    LimitClause,
    OrderByClause,
    OffsetClause,
    GroupByClause,
    CallStatement,
    CreateSchemaStatement,
    DropSchemaStatement,
    InsertStatement,
    DeleteStatement,
    SetStatement,
    RemoveStatement,
    StartTransactionStatement,
    CommitStatement,
    RollbackStatement,
    SessionSetStatement,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct GrammarParserEntrypoint {
    pub(crate) action: GrammarParserAction,
}
pub(crate) fn top_level_parser_entrypoint(keyword: Keyword) -> Option<GrammarParserEntrypoint> {
    match keyword {
        Keyword::Match => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::MatchClause,
        }),
        Keyword::Optional => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::OptionalMatchClause,
        }),
        Keyword::Return => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::ReturnClause,
        }),
        Keyword::Finish => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::FinishStatement,
        }),
        Keyword::Where => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::WhereClause,
        }),
        Keyword::Let => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::LetClause,
        }),
        Keyword::Filter => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::FilterStatement,
        }),
        Keyword::For => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::ForStatement,
        }),
        Keyword::Union => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::UnionClause,
        }),
        Keyword::Limit => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::LimitClause,
        }),
        Keyword::Order => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::OrderByClause,
        }),
        Keyword::Offset => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::OffsetClause,
        }),
        Keyword::Skip => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::OffsetClause,
        }),
        Keyword::Group => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::GroupByClause,
        }),
        Keyword::Call => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::CallStatement,
        }),
        Keyword::Create => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::CreateSchemaStatement,
        }),
        Keyword::Drop => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::DropSchemaStatement,
        }),
        Keyword::Insert => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::InsertStatement,
        }),
        Keyword::Delete => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::DeleteStatement,
        }),
        Keyword::Set => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::SetStatement,
        }),
        Keyword::Remove => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::RemoveStatement,
        }),
        Keyword::Detach => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::DeleteStatement,
        }),
        Keyword::Nodetach => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::DeleteStatement,
        }),
        Keyword::Start => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::StartTransactionStatement,
        }),
        Keyword::Commit => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::CommitStatement,
        }),
        Keyword::Rollback => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::RollbackStatement,
        }),
        Keyword::Session => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::SessionSetStatement,
        }),
        _ => None,
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BinaryOperatorSpec {
    pub(crate) left_binding_power: u8,
    pub(crate) right_binding_power: u8,
    pub(crate) is_right_associative: bool,
    pub(crate) width: u8,
}
pub(crate) fn binary_operator_spec(
    first: TokenKind,
    second: Option<TokenKind>,
) -> Option<BinaryOperatorSpec> {
    match (first, second) {
        (TokenKind::Punctuation('<'), Some(TokenKind::Punctuation('>'))) => {
            Some(BinaryOperatorSpec {
                left_binding_power: 30,
                right_binding_power: 31,
                is_right_associative: false,
                width: 2,
            })
        }
        (TokenKind::Punctuation('<'), Some(TokenKind::Punctuation('='))) => {
            Some(BinaryOperatorSpec {
                left_binding_power: 30,
                right_binding_power: 31,
                is_right_associative: false,
                width: 2,
            })
        }
        (TokenKind::Punctuation('>'), Some(TokenKind::Punctuation('='))) => {
            Some(BinaryOperatorSpec {
                left_binding_power: 30,
                right_binding_power: 31,
                is_right_associative: false,
                width: 2,
            })
        }
        (TokenKind::Punctuation('|'), Some(TokenKind::Punctuation('|'))) => {
            Some(BinaryOperatorSpec {
                left_binding_power: 35,
                right_binding_power: 36,
                is_right_associative: false,
                width: 2,
            })
        }
        (TokenKind::Keyword(Keyword::Or), _) => Some(BinaryOperatorSpec {
            left_binding_power: 10,
            right_binding_power: 11,
            is_right_associative: false,
            width: 1,
        }),
        (TokenKind::Keyword(Keyword::Xor), _) => Some(BinaryOperatorSpec {
            left_binding_power: 15,
            right_binding_power: 16,
            is_right_associative: false,
            width: 1,
        }),
        (TokenKind::Keyword(Keyword::And), _) => Some(BinaryOperatorSpec {
            left_binding_power: 20,
            right_binding_power: 21,
            is_right_associative: false,
            width: 1,
        }),
        (TokenKind::Keyword(Keyword::In), _) => Some(BinaryOperatorSpec {
            left_binding_power: 30,
            right_binding_power: 31,
            is_right_associative: false,
            width: 1,
        }),
        (TokenKind::Punctuation('='), _) => Some(BinaryOperatorSpec {
            left_binding_power: 30,
            right_binding_power: 31,
            is_right_associative: false,
            width: 1,
        }),
        (TokenKind::Punctuation('<'), _) => Some(BinaryOperatorSpec {
            left_binding_power: 30,
            right_binding_power: 31,
            is_right_associative: false,
            width: 1,
        }),
        (TokenKind::Punctuation('>'), _) => Some(BinaryOperatorSpec {
            left_binding_power: 30,
            right_binding_power: 31,
            is_right_associative: false,
            width: 1,
        }),
        (TokenKind::Punctuation('+'), _) => Some(BinaryOperatorSpec {
            left_binding_power: 40,
            right_binding_power: 41,
            is_right_associative: false,
            width: 1,
        }),
        (TokenKind::Punctuation('-'), _) => Some(BinaryOperatorSpec {
            left_binding_power: 40,
            right_binding_power: 41,
            is_right_associative: false,
            width: 1,
        }),
        (TokenKind::Punctuation('*'), _) => Some(BinaryOperatorSpec {
            left_binding_power: 50,
            right_binding_power: 51,
            is_right_associative: false,
            width: 1,
        }),
        (TokenKind::Punctuation('/'), _) => Some(BinaryOperatorSpec {
            left_binding_power: 50,
            right_binding_power: 51,
            is_right_associative: false,
            width: 1,
        }),
        (TokenKind::Punctuation('%'), _) => Some(BinaryOperatorSpec {
            left_binding_power: 50,
            right_binding_power: 51,
            is_right_associative: false,
            width: 1,
        }),
        _ => None,
    }
}
pub(crate) fn prefix_operator_precedence(kind: TokenKind) -> Option<u8> {
    match kind {
        TokenKind::Keyword(Keyword::Not) => Some(25),
        TokenKind::Punctuation('+') => Some(60),
        TokenKind::Punctuation('-') => Some(60),
        _ => None,
    }
}
#[rustfmt::skip]
pub(crate) const GRAMMAR_RECOVERIES: &[(&str, &str, &str)] = &[
    ("block-comment", "GQL-SYNTAX-UNTERMINATED-BLOCK-COMMENT", "preserve-source"),
    ("numeric-literal", "GQL-SYNTAX-INVALID-NUMERIC-LITERAL", "preserve-source"),
    ("integer-literal-range", "GQL-SYNTAX-NUMERIC-LITERAL-OUT-OF-RANGE", "preserve-source"),
    ("edge-label-separator", "GQL-PARSE-EDGE-LABEL-SEPARATOR", "preserve-source"),
    ("create-schema", "GQL-PARSE-CREATE-SCHEMA-SYNTAX", "preserve-source"),
    ("drop-schema", "GQL-PARSE-DROP-SCHEMA-SYNTAX", "preserve-source"),
    ("create-graph", "GQL-PARSE-CREATE-GRAPH-SYNTAX", "preserve-source"),
    ("drop-graph", "GQL-PARSE-DROP-GRAPH-SYNTAX", "preserve-source"),
    ("create-graph-type", "GQL-PARSE-CREATE-GRAPH-TYPE-SYNTAX", "preserve-source"),
    ("nested-graph-type", "GQL-PARSE-NESTED-GRAPH-TYPE-SYNTAX", "preserve-source"),
    ("drop-graph-type", "GQL-PARSE-DROP-GRAPH-TYPE-SYNTAX", "preserve-source"),
    ("insert-statement", "GQL-PARSE-INSERT-SYNTAX", "preserve-source"),
    ("set-statement", "GQL-PARSE-SET-SYNTAX", "preserve-source"),
    ("remove-statement", "GQL-PARSE-REMOVE-SYNTAX", "preserve-source"),
    ("delete-statement", "GQL-PARSE-DELETE-SYNTAX", "preserve-source"),
    ("call-statement", "GQL-PARSE-CALL-SYNTAX", "preserve-source"),
    ("transaction-command", "GQL-PARSE-TRANSACTION-SYNTAX", "preserve-source"),
    ("session-command", "GQL-PARSE-SESSION-COMMAND-SYNTAX", "preserve-source"),
    ("inline-node-where", "GQL-PARSE-INLINE-WHERE-SYNTAX", "preserve-source"),
    ("inline-edge-where", "GQL-PARSE-INLINE-WHERE-SYNTAX", "preserve-source"),
    ("path-mode", "GQL-PARSE-PATH-MODE-SYNTAX", "preserve-source"),
    ("graph-match-mode", "GQL-PARSE-GRAPH-MATCH-MODE-SYNTAX", "preserve-source"),
    ("path-search-prefix", "GQL-PARSE-PATH-SEARCH-PREFIX-SYNTAX", "preserve-source"),
    ("keep-clause", "GQL-PARSE-KEEP-CLAUSE-SYNTAX", "preserve-source"),
    ("order-by-clause", "GQL-PARSE-ORDER-BY-SYNTAX", "preserve-source"),
    ("limit-clause", "GQL-PARSE-LIMIT-SYNTAX", "preserve-source"),
    ("offset-clause", "GQL-PARSE-OFFSET-SYNTAX", "preserve-source"),
    ("path-quantifier", "GQL-PARSE-PATH-QUANTIFIER", "preserve-source"),
    ("string-literal", "GQL-SYNTAX-UNTERMINATED-STRING", "preserve-source"),
    ("character-string-literal", "GQL-SYNTAX-INVALID-CHARACTER-STRING-LITERAL", "preserve-source"),
    ("byte-string-literal", "GQL-SYNTAX-INVALID-BYTE-STRING", "preserve-source"),
    ("temporal-literal", "GQL-SYNTAX-INVALID-TEMPORAL-LITERAL", "preserve-source"),
    ("duration-literal", "GQL-SYNTAX-INVALID-DURATION-LITERAL", "preserve-source"),
    ("list-literal", "GQL-PARSE-LIST-SYNTAX", "preserve-source"),
    ("record-literal", "GQL-PARSE-RECORD-SYNTAX", "preserve-source"),
    ("delimited-identifier", "GQL-SYNTAX-UNTERMINATED-DELIMITED-IDENTIFIER", "preserve-source"),
    ("identifier-escape", "GQL-SYNTAX-INVALID-IDENTIFIER-ESCAPE", "preserve-source"),
    ("dynamic-parameter", "GQL-SYNTAX-INVALID-DYNAMIC-PARAMETER", "preserve-source"),
    ("substituted-parameter-context", "GQL-PARSE-SUBSTITUTED-PARAMETER-CONTEXT", "preserve-source"),
    ("predicate-test", "GQL-PARSE-PREDICATE-TEST-SYNTAX", "preserve-source"),
    ("null-predicate-operand", "GQL-PARSE-NULL-PREDICATE-OPERAND", "preserve-source"),
    ("value-type-predicate", "GQL-PARSE-VALUE-TYPE-PREDICATE-SYNTAX", "preserve-source"),
    ("value-type-predicate-operand", "GQL-PARSE-VALUE-TYPE-PREDICATE-OPERAND", "preserve-source"),
    ("graph-element-predicate", "GQL-PARSE-GRAPH-ELEMENT-PREDICATE-SYNTAX", "preserve-source"),
    ("aggregate-function", "GQL-PARSE-AGGREGATE-FUNCTION-SYNTAX", "preserve-source"),
    ("binding-variable", "GQL-PARSE-BINDING-VARIABLE-SYNTAX", "preserve-source"),
    ("unsupported-statement", "GQL-PARSE-UNSUPPORTED-STATEMENT", "preserve-source"),
    ("unsupported-keyword-expression", "GQL-PARSE-UNSUPPORTED-KEYWORD-EXPRESSION", "preserve-source"),
    ("non-iso-operator", "GQL-PARSE-NON-ISO-OPERATOR", "preserve-source"),
    ("label-expression", "GQL-PARSE-LABEL-EXPRESSION", "preserve-source"),
    ("match-pattern-list", "GQL-PARSE-MATCH-PATTERN-LIST", "preserve-source"),
    ("optional-match", "GQL-PARSE-OPTIONAL-MATCH-SYNTAX", "preserve-source"),
    ("where-clause", "GQL-PARSE-WHERE-SYNTAX", "preserve-source"),
    ("filter-statement", "GQL-PARSE-FILTER-SYNTAX", "preserve-source"),
    ("for-statement", "GQL-PARSE-FOR-SYNTAX", "preserve-source"),
    ("finish-statement", "GQL-PARSE-FINISH-SYNTAX", "preserve-source"),
    ("union-clause", "GQL-PARSE-UNION-SYNTAX", "preserve-source"),
    ("expression-syntax", "GQL-PARSE-EXPRESSION-SYNTAX", "preserve-source"),
];
pub(crate) fn recovery_diagnostic(site: &str) -> Option<&'static str> {
    GRAMMAR_RECOVERIES
        .iter()
        .find_map(|(candidate, code, _)| (*candidate == site).then_some(*code))
}
