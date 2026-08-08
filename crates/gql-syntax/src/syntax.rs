//! Syntax kinds, lexical tokens, and typed views over the Rowan CST.

use gql_source::{Diagnostic, SourceText, Span};
use rowan::NodeOrToken;

/// Rowan syntax node for the GQL language.
pub type RowanSyntaxNode = rowan::SyntaxNode<GqlSyntax>;
/// Rowan syntax token for the GQL language.
pub type RowanSyntaxToken = rowan::SyntaxToken<GqlSyntax>;

/// Syntax kinds for CST nodes and tokens.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
#[repr(u16)]
pub enum SyntaxKind {
    /// Top-level source container.
    SourceFile,
    /// Query root node.
    Query,
    /// `MATCH` clause node.
    MatchClause,
    /// `WHERE` clause node.
    WhereClause,
    /// `LET` clause node.
    LetClause,
    /// `RETURN` clause node.
    ReturnClause,
    /// Graph pattern node.
    GraphPattern,
    /// Node pattern node.
    NodePattern,
    /// Edge pattern node.
    EdgePattern,
    /// Label list node.
    LabelList,
    /// Generic recovery expression node.
    Expression,
    /// Name/reference expression node.
    NameExpression,
    /// String or numeric literal expression node.
    LiteralExpression,
    /// Unary expression node.
    UnaryExpression,
    /// Binary expression node.
    BinaryExpression,
    /// Parenthesized expression node.
    ParenthesizedExpression,
    /// Keyword token kind.
    Keyword,
    /// Identifier token kind.
    Identifier,
    /// Number token kind.
    Number,
    /// String token kind.
    String,
    /// Whitespace token kind.
    Whitespace,
    /// Punctuation token kind.
    Punctuation,
    /// Comment token kind.
    Comment,
    /// Unknown token kind.
    Unknown,
}

impl SyntaxKind {
    /// Converts this kind into Rowan's raw kind.
    pub(crate) fn to_rowan(self) -> rowan::SyntaxKind {
        rowan::SyntaxKind(self as u16)
    }

    /// Decodes Rowan's raw kind into the language kind.
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
            8 => Self::EdgePattern,
            9 => Self::LabelList,
            10 => Self::Expression,
            11 => Self::NameExpression,
            12 => Self::LiteralExpression,
            13 => Self::UnaryExpression,
            14 => Self::BinaryExpression,
            15 => Self::ParenthesizedExpression,
            16 => Self::Keyword,
            17 => Self::Identifier,
            18 => Self::Number,
            19 => Self::String,
            20 => Self::Whitespace,
            21 => Self::Punctuation,
            22 => Self::Comment,
            _ => Self::Unknown,
        }
    }
}

/// Parsed keyword space for this project slice.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Keyword {
    Match,
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
}

/// Token kind for lexical output and typed CST views.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    /// Keyword token.
    Keyword(Keyword),
    /// Identifier token.
    Identifier,
    /// Number token.
    Number,
    /// String token.
    String,
    /// Whitespace token.
    Whitespace,
    /// Punctuation token with exact character.
    Punctuation(char),
    /// Comment token.
    Comment,
    /// Fallback unknown token.
    Unknown,
}

/// A lexical token or a token projected from the Rowan CST.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    /// Token kind.
    pub kind: TokenKind,
    /// Byte span in source text.
    pub span: Span,
    text: String,
}

impl Token {
    /// Creates a lexical token with its exact source text.
    pub(crate) fn new(kind: TokenKind, span: Span, text: impl Into<String>) -> Self {
        Self {
            kind,
            span,
            text: text.into(),
        }
    }

    fn from_rowan(token: RowanSyntaxToken) -> Self {
        let text = token.text().to_string();
        let kind = match token.kind() {
            SyntaxKind::Keyword => crate::lexer::keyword(&text)
                .map(TokenKind::Keyword)
                .unwrap_or(TokenKind::Unknown),
            SyntaxKind::Identifier => TokenKind::Identifier,
            SyntaxKind::Number => TokenKind::Number,
            SyntaxKind::String => TokenKind::String,
            SyntaxKind::Whitespace => TokenKind::Whitespace,
            SyntaxKind::Punctuation => text
                .chars()
                .next()
                .map(TokenKind::Punctuation)
                .unwrap_or(TokenKind::Unknown),
            SyntaxKind::Comment => TokenKind::Comment,
            _ => TokenKind::Unknown,
        };
        Self {
            kind,
            span: span_from_range(token.text_range()),
            text,
        }
    }

    /// Exact token text as represented in the CST.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Converts the token kind into its CST kind.
    pub(crate) fn syntax_kind(&self) -> SyntaxKind {
        match self.kind {
            TokenKind::Keyword(_) => SyntaxKind::Keyword,
            TokenKind::Identifier => SyntaxKind::Identifier,
            TokenKind::Number => SyntaxKind::Number,
            TokenKind::String => SyntaxKind::String,
            TokenKind::Whitespace => SyntaxKind::Whitespace,
            TokenKind::Punctuation(_) => SyntaxKind::Punctuation,
            TokenKind::Comment => SyntaxKind::Comment,
            TokenKind::Unknown => SyntaxKind::Unknown,
        }
    }
}

/// One typed view over a Rowan syntax element.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxElement {
    /// Child view.
    pub kind: SyntaxElementKind,
}

/// Child view in the Rowan concrete syntax tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SyntaxElementKind {
    /// Rowan child node view.
    Node(SyntaxNode),
    /// Rowan child token view.
    Token(Token),
}

/// Typed view over one Rowan node. It owns no independent children or spans.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxNode {
    rowan: RowanSyntaxNode,
}

impl SyntaxNode {
    pub(crate) fn from_rowan(rowan: RowanSyntaxNode) -> Self {
        Self { rowan }
    }

    /// Node kind supplied by the Rowan language marker.
    #[must_use]
    pub fn kind(&self) -> SyntaxKind {
        self.rowan.kind()
    }

    /// Node span derived from the Rowan text range.
    #[must_use]
    pub fn span(&self) -> Span {
        span_from_range(self.rowan.text_range())
    }

    /// Typed child views derived from the Rowan tree.
    #[must_use]
    pub fn children(&self) -> Vec<SyntaxElement> {
        self.rowan
            .children_with_tokens()
            .map(|element| SyntaxElement {
                kind: match element {
                    NodeOrToken::Node(node) => SyntaxElementKind::Node(Self::from_rowan(node)),
                    NodeOrToken::Token(token) => SyntaxElementKind::Token(Token::from_rowan(token)),
                },
            })
            .collect()
    }

    /// Underlying Rowan node for typed syntax adapters.
    #[must_use]
    pub fn rowan(&self) -> &RowanSyntaxNode {
        &self.rowan
    }
}

/// Full syntax tree with source attachment and one Rowan green tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxTree {
    source: SourceText,
    tokens: Vec<Token>,
    rowan: rowan::GreenNode,
}

impl SyntaxTree {
    /// Creates a syntax tree from the source, lexical stream, and Rowan root.
    #[must_use]
    pub(crate) fn new(source: SourceText, tokens: Vec<Token>, rowan: rowan::GreenNode) -> Self {
        Self {
            source,
            tokens,
            rowan,
        }
    }

    /// Borrow source text.
    #[must_use]
    pub fn source(&self) -> &SourceText {
        &self.source
    }

    /// Borrow the lexical stream used to build the Rowan tree.
    #[must_use]
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    /// Typed Rowan root view.
    #[must_use]
    pub fn root(&self) -> SyntaxNode {
        SyntaxNode::from_rowan(self.rowan_root())
    }

    /// Raw Rowan root node.
    #[must_use]
    pub fn rowan_root(&self) -> RowanSyntaxNode {
        RowanSyntaxNode::new_root(self.rowan.clone())
    }
}

/// Parse result returned from [`crate::parse`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parse {
    /// Parsed Rowan tree.
    pub tree: SyntaxTree,
    /// Diagnostics produced while parsing.
    pub diagnostics: Vec<Diagnostic>,
}

/// Rowan language marker for the crate syntax kinds.
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct GqlSyntax;

impl rowan::Language for GqlSyntax {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        SyntaxKind::from_rowan(raw)
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.to_rowan()
    }
}

fn span_from_range(range: rowan::TextRange) -> Span {
    Span::new(u32::from(range.start()), u32::from(range.end()))
}
