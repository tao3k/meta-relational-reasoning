//! Syntax kinds, lexical tokens, and typed views over the Rowan CST.

use gql_source::{Diagnostic, SourceText, Span};
use rowan::NodeOrToken;

use crate::generated::{
    GERBIL_SCHEME_RUST_REVISION, GRAMMAR_PROJECTION_SCHEMA, GRAMMAR_RECOVERIES,
    GRAMMAR_SYNTAX_SHAPES,
};
pub(crate) use crate::generated::{
    GrammarParserAction, aggregate_function_spec, binary_operator_spec, keyword,
    prefix_operator_precedence, recovery_diagnostic, top_level_parser_entrypoint,
};
pub use crate::generated::{
    ISO_GQL_AGGREGATE_FUNCTION_FORMS, ISO_GQL_CHARACTER_STRING_FORMS, ISO_GQL_NON_RESERVED_WORDS,
    ISO_GQL_NUMERIC_LITERAL_FORMS, ISO_GQL_PARAMETER_REFERENCE_FORMS, ISO_GQL_PREDICATE_TEST_FORMS,
    Keyword, SyntaxKind, is_non_reserved_word,
};

/// Rowan syntax node for the GQL language.
pub type RowanSyntaxNode = rowan::SyntaxNode<GqlSyntax>;
/// Rowan syntax token for the GQL language.
pub type RowanSyntaxToken = rowan::SyntaxToken<GqlSyntax>;

/// Provenance and contracts embedded in the generated grammar.
pub struct GrammarProjectionReceipt<'a> {
    /// Projection schema identifier.
    pub schema: &'a str,
    /// SHA-256 over the canonical Scheme inputs.
    pub input_sha256: &'a str,
    /// SHA-256 over the generated Rust body.
    pub body_sha256: &'a str,
    /// Exact admitted `gerbil-scheme-rust` PR16 revision.
    pub bridge_revision: &'a str,
    /// Generated node/token category and field-shape contracts.
    pub syntax_shapes: &'static [(&'static str, &'static str, &'static [&'static str])],
    /// Generated recovery site, diagnostic, and strategy contracts.
    pub recoveries: &'static [(&'static str, &'static str, &'static str)],
}

/// Reads the fail-closed provenance receipt from the tracked projection.
#[must_use]
pub fn grammar_projection_receipt() -> GrammarProjectionReceipt<'static> {
    const TRACKED_PROJECTION: &str = include_str!("generated/projection.rs");
    let mut fields = TRACKED_PROJECTION
        .lines()
        .next()
        .expect("generated grammar must have a provenance header")
        .split_ascii_whitespace();
    assert_eq!(fields.next(), Some("//"));
    let schema = fields.next().expect("generated grammar schema");
    let input_sha256 = fields
        .next()
        .and_then(|field| field.strip_prefix("input-sha256="))
        .expect("generated grammar input fingerprint");
    let body_sha256 = fields
        .next()
        .and_then(|field| field.strip_prefix("body-sha256="))
        .expect("generated grammar body fingerprint");
    let bridge_revision = fields
        .next()
        .and_then(|field| field.strip_prefix("gerbil-scheme-rust-rev="))
        .expect("generated grammar bridge revision");
    assert!(fields.next().is_none());
    assert_eq!(schema, GRAMMAR_PROJECTION_SCHEMA);
    assert_eq!(bridge_revision, GERBIL_SCHEME_RUST_REVISION);
    GrammarProjectionReceipt {
        schema,
        input_sha256,
        body_sha256,
        bridge_revision,
        syntax_shapes: GRAMMAR_SYNTAX_SHAPES,
        recoveries: GRAMMAR_RECOVERIES,
    }
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
    /// Byte-string token.
    ByteString,
    /// General parameter reference used as a dynamic value.
    DynamicParameter,
    /// Substituted parameter reference reserved for catalog references.
    SubstitutedParameter,
    /// Whitespace token.
    Whitespace,
    /// Punctuation token with exact character.
    Punctuation(char),
    /// Comment token.
    Comment,
    /// Unrecognized source token retained for lossless diagnostics.
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
            SyntaxKind::ByteString => TokenKind::ByteString,
            SyntaxKind::Parameter if text.starts_with("$$") => TokenKind::SubstitutedParameter,
            SyntaxKind::Parameter => TokenKind::DynamicParameter,
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
            TokenKind::ByteString => SyntaxKind::ByteString,
            TokenKind::DynamicParameter | TokenKind::SubstitutedParameter => SyntaxKind::Parameter,
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
