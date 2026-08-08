//! Syntax-surface types for tokens, syntax kinds, and CST nodes.

use gql_source::{Diagnostic, SourceText, Span};

/// Owned alias for a `rowan` syntax node.
pub type RowanSyntaxNode = rowan::SyntaxNode<GqlSyntax>;
/// Owned alias for a `rowan` syntax token.
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
    /// Generic expression node.
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
    /// Keyword token wrapper.
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
    /// Converts this kind into a `rowan` kind id.
    pub(crate) fn to_rowan(self) -> rowan::SyntaxKind {
        rowan::SyntaxKind(self as u16)
    }

    /// Decodes a `rowan` syntax kind back to this enum.
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

/// Token kind for lexical output.
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

/// A parsed token.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    /// Token kind.
    pub kind: TokenKind,
    /// Byte span in source text.
    pub span: Span,
}

impl Token {
    /// Converts token kind into syntax kind.
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

/// Union for owned syntax node/token children.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxElement {
    /// Child kind.
    pub kind: SyntaxElementKind,
}

/// Child edge in a concrete syntax tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SyntaxElementKind {
    /// Owned node child.
    Node(SyntaxNode),
    /// Owned token child.
    Token(Token),
}

/// One CST node with kind, span, and children.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxNode {
    /// Syntax kind.
    kind: SyntaxKind,
    /// Byte span for the node.
    span: Span,
    /// Child elements.
    children: Vec<SyntaxElement>,
}

impl SyntaxNode {
    /// Creates a node from kind, span, and children.
    pub(crate) fn new(kind: SyntaxKind, span: Span, children: Vec<SyntaxElement>) -> Self {
        Self {
            kind,
            span,
            children,
        }
    }

    /// Node kind.
    #[must_use]
    pub fn kind(&self) -> SyntaxKind {
        self.kind
    }

    /// Node span.
    #[must_use]
    pub fn span(&self) -> Span {
        self.span
    }

    /// Child slice.
    #[must_use]
    pub fn children(&self) -> &[SyntaxElement] {
        &self.children
    }
}

/// A full syntax tree with rowan backing and source attachment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxTree {
    /// Owned source text.
    source: SourceText,
    /// Root node.
    root: SyntaxNode,
    /// Token stream.
    tokens: Vec<Token>,
    /// Owned `rowan` root.
    rowan: rowan::GreenNode,
}

impl SyntaxTree {
    /// Creates a syntax tree from all required fields.
    #[must_use]
    pub fn new(
        source: SourceText,
        root: SyntaxNode,
        tokens: Vec<Token>,
        rowan: rowan::GreenNode,
    ) -> Self {
        Self {
            source,
            root,
            tokens,
            rowan,
        }
    }

    /// Borrow source text.
    #[must_use]
    pub fn source(&self) -> &SourceText {
        &self.source
    }

    /// Borrow token stream.
    #[must_use]
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    /// Borrow root node.
    #[must_use]
    pub fn root(&self) -> &SyntaxNode {
        &self.root
    }

    /// Converts to a `rowan` root.
    #[must_use]
    pub fn rowan_root(&self) -> RowanSyntaxNode {
        RowanSyntaxNode::new_root(self.rowan.clone())
    }
}

/// Parse result returned from [`parse`](crate::parse).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parse {
    /// Parsed tree.
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
