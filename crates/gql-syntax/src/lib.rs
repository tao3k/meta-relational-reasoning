#![forbid(unsafe_code)]

use gql_source::{Diagnostic, SourceText, Span};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Keyword {
    Match,
    Where,
    Let,
    Return,
    Call,
    Create,
    Drop,
    Insert,
    Delete,
    Set,
    Remove,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    Keyword(Keyword),
    Identifier,
    Number,
    String,
    Whitespace,
    Punctuation(char),
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxTree {
    source: SourceText,
    tokens: Vec<Token>,
}

impl SyntaxTree {
    #[must_use]
    pub fn source(&self) -> &SourceText {
        &self.source
    }

    #[must_use]
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parse {
    pub tree: SyntaxTree,
    pub diagnostics: Vec<Diagnostic>,
}

#[must_use]
pub fn parse(name: impl Into<std::sync::Arc<str>>, input: impl Into<std::sync::Arc<str>>) -> Parse {
    let source = SourceText::new(name, input);
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    let text = source.text();
    let mut cursor = 0;
    while cursor < text.len() {
        let start = cursor;
        let ch = text[cursor..].chars().next().expect("cursor is in bounds");
        if ch.is_whitespace() {
            cursor += ch.len_utf8();
            while cursor < text.len() {
                let next = text[cursor..].chars().next().expect("cursor is in bounds");
                if !next.is_whitespace() {
                    break;
                }
                cursor += next.len_utf8();
            }
            tokens.push(token(TokenKind::Whitespace, start, cursor));
        } else if ch == '\'' {
            cursor += 1;
            while cursor < text.len() {
                let next = text[cursor..].chars().next().expect("cursor is in bounds");
                cursor += next.len_utf8();
                if next == '\'' {
                    break;
                }
            }
            if !text[start + 1..cursor].ends_with('\'') {
                diagnostics.push(Diagnostic::error(
                    "GQL-SYNTAX-UNTERMINATED-STRING",
                    "unterminated character string literal",
                    Span::new(start as u32, cursor as u32),
                ));
            }
            tokens.push(token(TokenKind::String, start, cursor));
        } else if ch.is_alphabetic() || ch == '_' {
            cursor += ch.len_utf8();
            while cursor < text.len() {
                let next = text[cursor..].chars().next().expect("cursor is in bounds");
                if !(next.is_alphanumeric() || next == '_') {
                    break;
                }
                cursor += next.len_utf8();
            }
            let word = &text[start..cursor];
            let kind = keyword(word).map_or(TokenKind::Identifier, TokenKind::Keyword);
            tokens.push(token(kind, start, cursor));
        } else if ch.is_ascii_digit() {
            cursor += ch.len_utf8();
            while cursor < text.len() && text.as_bytes()[cursor].is_ascii_digit() {
                cursor += 1;
            }
            tokens.push(token(TokenKind::Number, start, cursor));
        } else if "()[]{}.,:;*+-/=<>–".contains(ch) {
            cursor += ch.len_utf8();
            tokens.push(token(TokenKind::Punctuation(ch), start, cursor));
        } else {
            cursor += ch.len_utf8();
            diagnostics.push(Diagnostic::error(
                "GQL-SYNTAX-UNKNOWN-CHARACTER",
                format!("unrecognized character `{ch}`"),
                Span::new(start as u32, cursor as u32),
            ));
            tokens.push(token(TokenKind::Unknown, start, cursor));
        }
    }
    Parse {
        tree: SyntaxTree { source, tokens },
        diagnostics,
    }
}

fn token(kind: TokenKind, start: usize, end: usize) -> Token {
    Token {
        kind,
        span: Span::new(start as u32, end as u32),
    }
}

fn keyword(word: &str) -> Option<Keyword> {
    match word.to_ascii_uppercase().as_str() {
        "MATCH" => Some(Keyword::Match),
        "WHERE" => Some(Keyword::Where),
        "LET" => Some(Keyword::Let),
        "RETURN" => Some(Keyword::Return),
        "CALL" => Some(Keyword::Call),
        "CREATE" => Some(Keyword::Create),
        "DROP" => Some(Keyword::Drop),
        "INSERT" => Some(Keyword::Insert),
        "DELETE" => Some(Keyword::Delete),
        "SET" => Some(Keyword::Set),
        "REMOVE" => Some(Keyword::Remove),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_source_and_unicode_identifiers() {
        let input = "MATCH (节点)-[:CALLS]->(目标) RETURN 节点";
        let parsed = parse("test.gql", input);
        assert!(parsed.diagnostics.is_empty());
        assert_eq!(parsed.tree.source().text(), input);
        assert!(
            parsed
                .tree
                .tokens()
                .iter()
                .any(|token| token.kind == TokenKind::Identifier)
        );
    }
}
