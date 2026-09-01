// mrr.gerbil-grammar-projection.v1 input-sha256=349b110601e6ca7af5933998d319da4406ce5a0d55914e360f50a758d5da6195 body-sha256=3801fd6378b92c4a7c7adab04ae59c0dabd384430270f3a74b7a894e2b935eda gerbil-scheme-rust-rev=a83fb649ddbbeaabdb538a6eaf0ded10838f7fad
// Generated through the Gerbil native AOT bindings; do not edit.
//! Property-graph grammar projection consumed by the Rowan CST frontend.
use crate::syntax::TokenKind;
pub(crate) const GRAMMAR_PROJECTION_SCHEMA: &str = "mrr.gerbil-grammar-projection.v1";
pub(crate) const GERBIL_SCHEME_RUST_REVISION: &str = "a83fb649ddbbeaabdb538a6eaf0ded10838f7fad";
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
#[repr(u16)]
pub enum SyntaxKind {
    SourceFile,
    Query,
    MatchClause,
    WhereClause,
    LetClause,
    ReturnClause,
    GraphPattern,
    NodePattern,
    PropertyMap,
    PropertyEntry,
    EdgePattern,
    LabelList,
    Expression,
    NameExpression,
    LiteralExpression,
    UnaryExpression,
    BinaryExpression,
    ParenthesizedExpression,
    Keyword,
    Identifier,
    Number,
    String,
    Whitespace,
    Punctuation,
    Comment,
    Unknown,
    PropertyAccessExpression,
    PathPattern,
    PathQuantifier,
    OptionalMatchClause,
    ListExpression,
    SubscriptExpression,
    ProjectionAlias,
    UnionClause,
    LimitClause,
    OrderByClause,
    OffsetClause,
    CaseExpression,
    CaseWhenClause,
    CaseElseClause,
}
impl SyntaxKind {
    pub(crate) fn to_rowan(self) -> rowan::SyntaxKind {
        rowan::SyntaxKind(self as u16)
    }
    pub(crate) fn from_rowan(kind: rowan::SyntaxKind) -> Self {
        match kind.0 {
            0 => Self::SourceFile,
            1 => Self::Query,
            2 => Self::MatchClause,
            3 => Self::WhereClause,
            4 => Self::LetClause,
            5 => Self::ReturnClause,
            6 => Self::GraphPattern,
            7 => Self::NodePattern,
            8 => Self::PropertyMap,
            9 => Self::PropertyEntry,
            10 => Self::EdgePattern,
            11 => Self::LabelList,
            12 => Self::Expression,
            13 => Self::NameExpression,
            14 => Self::LiteralExpression,
            15 => Self::UnaryExpression,
            16 => Self::BinaryExpression,
            17 => Self::ParenthesizedExpression,
            18 => Self::Keyword,
            19 => Self::Identifier,
            20 => Self::Number,
            21 => Self::String,
            22 => Self::Whitespace,
            23 => Self::Punctuation,
            24 => Self::Comment,
            25 => Self::Unknown,
            26 => Self::PropertyAccessExpression,
            27 => Self::PathPattern,
            28 => Self::PathQuantifier,
            29 => Self::OptionalMatchClause,
            30 => Self::ListExpression,
            31 => Self::SubscriptExpression,
            32 => Self::ProjectionAlias,
            33 => Self::UnionClause,
            34 => Self::LimitClause,
            35 => Self::OrderByClause,
            36 => Self::OffsetClause,
            37 => Self::CaseExpression,
            38 => Self::CaseWhenClause,
            39 => Self::CaseElseClause,
            _ => Self::Unknown,
        }
    }
}
pub(crate) const GRAMMAR_SYNTAX_SHAPES: &[(&str, &str, &[&str])] = &[
    ("SourceFile", "node", &["query"]),
    ("Query", "node", &["clause"]),
    ("MatchClause", "node", &["pattern"]),
    ("WhereClause", "node", &["expression"]),
    ("LetClause", "node", &["binding", "expression"]),
    ("ReturnClause", "node", &["projection"]),
    ("GraphPattern", "node", &["element"]),
    ("NodePattern", "node", &["binding", "labels", "properties"]),
    ("PropertyMap", "node", &["entry"]),
    ("PropertyEntry", "node", &["key", "value"]),
    (
        "EdgePattern",
        "node",
        &["direction", "binding", "labels", "properties", "quantifier"],
    ),
    ("LabelList", "node", &["label"]),
    ("Expression", "node", &["token"]),
    ("NameExpression", "node", &["name"]),
    ("LiteralExpression", "node", &["literal"]),
    ("UnaryExpression", "node", &["operator", "operand"]),
    ("BinaryExpression", "node", &["left", "operator", "right"]),
    ("ParenthesizedExpression", "node", &["expression"]),
    ("Keyword", "token", &["text"]),
    ("Identifier", "token", &["text"]),
    ("Number", "token", &["text"]),
    ("String", "token", &["text"]),
    ("Whitespace", "token", &["text"]),
    ("Punctuation", "token", &["text"]),
    ("Comment", "token", &["text"]),
    ("Unknown", "token", &["text"]),
    ("PropertyAccessExpression", "node", &["base", "property"]),
    ("PathPattern", "node", &["binding", "pattern"]),
    ("PathQuantifier", "node", &["minimum", "maximum"]),
    ("OptionalMatchClause", "node", &["match"]),
    ("ListExpression", "node", &["element"]),
    ("SubscriptExpression", "node", &["base", "index"]),
    ("ProjectionAlias", "node", &["expression", "alias"]),
    ("UnionClause", "node", &["query"]),
    ("LimitClause", "node", &["limit"]),
    ("OrderByClause", "node", &["key", "direction"]),
    ("OffsetClause", "node", &["offset"]),
    (
        "CaseExpression",
        "node",
        &["operand", "branch", "else-result"],
    ),
    ("CaseWhenClause", "node", &["condition", "result"]),
    ("CaseElseClause", "node", &["result"]),
];
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Keyword {
    Match,
    Optional,
    Where,
    Let,
    Return,
    Or,
    And,
    Not,
    Call,
    Create,
    Drop,
    Insert,
    Delete,
    Set,
    Remove,
    True,
    False,
    Null,
    In,
    As,
    Union,
    Limit,
    Order,
    By,
    Asc,
    Desc,
    Offset,
    Case,
    When,
    Then,
    Else,
    End,
}
pub(crate) fn keyword(word: &str) -> Option<Keyword> {
    match word.to_ascii_uppercase().as_str() {
        "MATCH" => Some(Keyword::Match),
        "OPTIONAL" => Some(Keyword::Optional),
        "WHERE" => Some(Keyword::Where),
        "LET" => Some(Keyword::Let),
        "RETURN" => Some(Keyword::Return),
        "OR" => Some(Keyword::Or),
        "AND" => Some(Keyword::And),
        "NOT" => Some(Keyword::Not),
        "CALL" => Some(Keyword::Call),
        "CREATE" => Some(Keyword::Create),
        "DROP" => Some(Keyword::Drop),
        "INSERT" => Some(Keyword::Insert),
        "DELETE" => Some(Keyword::Delete),
        "SET" => Some(Keyword::Set),
        "REMOVE" => Some(Keyword::Remove),
        "TRUE" => Some(Keyword::True),
        "FALSE" => Some(Keyword::False),
        "NULL" => Some(Keyword::Null),
        "IN" => Some(Keyword::In),
        "AS" => Some(Keyword::As),
        "UNION" => Some(Keyword::Union),
        "LIMIT" => Some(Keyword::Limit),
        "ORDER" => Some(Keyword::Order),
        "BY" => Some(Keyword::By),
        "ASC" => Some(Keyword::Asc),
        "DESC" => Some(Keyword::Desc),
        "OFFSET" => Some(Keyword::Offset),
        "CASE" => Some(Keyword::Case),
        "WHEN" => Some(Keyword::When),
        "THEN" => Some(Keyword::Then),
        "ELSE" => Some(Keyword::Else),
        "END" => Some(Keyword::End),
        _ => None,
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GrammarParserAction {
    MatchClause,
    OptionalMatchClause,
    ReturnClause,
    WhereClause,
    LetClause,
    UnionClause,
    LimitClause,
    OrderByClause,
    OffsetClause,
    UnsupportedStatement,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct GrammarParserEntrypoint {
    pub(crate) action: GrammarParserAction,
    pub(crate) marks_match: bool,
    pub(crate) marks_return: bool,
}
pub(crate) fn top_level_parser_entrypoint(keyword: Keyword) -> Option<GrammarParserEntrypoint> {
    match keyword {
        Keyword::Match => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::MatchClause,
            marks_match: true,
            marks_return: false,
        }),
        Keyword::Optional => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::OptionalMatchClause,
            marks_match: true,
            marks_return: false,
        }),
        Keyword::Return => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::ReturnClause,
            marks_match: false,
            marks_return: true,
        }),
        Keyword::Where => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::WhereClause,
            marks_match: false,
            marks_return: false,
        }),
        Keyword::Let => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::LetClause,
            marks_match: false,
            marks_return: false,
        }),
        Keyword::Union => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::UnionClause,
            marks_match: false,
            marks_return: false,
        }),
        Keyword::Limit => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::LimitClause,
            marks_match: false,
            marks_return: false,
        }),
        Keyword::Order => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::OrderByClause,
            marks_match: false,
            marks_return: false,
        }),
        Keyword::Offset => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::OffsetClause,
            marks_match: false,
            marks_return: false,
        }),
        Keyword::Call => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::UnsupportedStatement,
            marks_match: false,
            marks_return: false,
        }),
        Keyword::Create => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::UnsupportedStatement,
            marks_match: false,
            marks_return: false,
        }),
        Keyword::Drop => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::UnsupportedStatement,
            marks_match: false,
            marks_return: false,
        }),
        Keyword::Insert => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::UnsupportedStatement,
            marks_match: false,
            marks_return: false,
        }),
        Keyword::Delete => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::UnsupportedStatement,
            marks_match: false,
            marks_return: false,
        }),
        Keyword::Set => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::UnsupportedStatement,
            marks_match: false,
            marks_return: false,
        }),
        Keyword::Remove => Some(GrammarParserEntrypoint {
            action: GrammarParserAction::UnsupportedStatement,
            marks_match: false,
            marks_return: false,
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
        (TokenKind::Punctuation('!'), Some(TokenKind::Punctuation('='))) => {
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
        (TokenKind::Keyword(Keyword::Or), _) => Some(BinaryOperatorSpec {
            left_binding_power: 10,
            right_binding_power: 11,
            is_right_associative: false,
            width: 2,
        }),
        (TokenKind::Keyword(Keyword::And), _) => Some(BinaryOperatorSpec {
            left_binding_power: 20,
            right_binding_power: 21,
            is_right_associative: false,
            width: 3,
        }),
        (TokenKind::Keyword(Keyword::In), _) => Some(BinaryOperatorSpec {
            left_binding_power: 30,
            right_binding_power: 31,
            is_right_associative: false,
            width: 2,
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
pub(crate) fn prefix_operator_precedence(keyword: Keyword) -> Option<u8> {
    match keyword {
        Keyword::Not => Some(25),
        _ => None,
    }
}
pub(crate) const GRAMMAR_RECOVERIES: &[(&str, &str, &str)] = &[
    (
        "unsupported-statement",
        "GQL-PARSE-UNSUPPORTED-STATEMENT",
        "preserve-source",
    ),
    (
        "unsupported-keyword-expression",
        "GQL-PARSE-UNSUPPORTED-KEYWORD-EXPRESSION",
        "preserve-source",
    ),
    (
        "expression-syntax",
        "GQL-PARSE-EXPRESSION-SYNTAX",
        "preserve-source",
    ),
];
pub(crate) fn recovery_diagnostic(site: &str) -> Option<&'static str> {
    GRAMMAR_RECOVERIES
        .iter()
        .find_map(|(candidate, code, _)| (*candidate == site).then_some(*code))
}
