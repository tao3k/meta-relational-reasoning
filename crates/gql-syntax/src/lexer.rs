//! Lexer for the minimal GQL lexical surface in the current milestone.

use gql_source::{Diagnostic, Span};

pub(crate) use crate::syntax::keyword;
use crate::syntax::{Token, TokenKind};

/// Lexes `text` into tokens and diagnostics.
pub(crate) fn lex(text: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    let mut cursor = 0usize;

    while cursor < text.len() {
        let start = cursor;
        let ch = text[cursor..]
            .chars()
            .next()
            .expect("cursor must stay in bounds");

        if ch.is_whitespace() {
            consume_while(&mut cursor, text, |next| next.is_whitespace());
            tokens.push(Token::new(
                TokenKind::Whitespace,
                Span::new(start as u32, cursor as u32),
                &text[start..cursor],
            ));
            continue;
        }

        if ch == '#' {
            cursor += ch.len_utf8();
            consume_while(&mut cursor, text, |next| next != '\n');

            tokens.push(Token::new(
                TokenKind::Comment,
                Span::new(start as u32, cursor as u32),
                &text[start..cursor],
            ));
            continue;
        }

        if ch == '\'' {
            cursor += 1;
            consume_while(&mut cursor, text, |next| next != '\'');
            if cursor < text.len() {
                cursor += 1;
            }

            if start + 1 == cursor
                || text
                    .as_bytes()
                    .get(cursor.saturating_sub(1))
                    .is_none_or(|byte| *byte != b'\'')
            {
                diagnostics.push(Diagnostic::error(
                    "GQL-SYNTAX-UNTERMINATED-STRING",
                    "unterminated string literal",
                    Span::new(start as u32, cursor as u32),
                ));
            }

            tokens.push(Token::new(
                TokenKind::String,
                Span::new(start as u32, cursor as u32),
                &text[start..cursor],
            ));
            continue;
        }

        if is_identifier_start(ch) {
            cursor += ch.len_utf8();
            consume_while(&mut cursor, text, is_identifier_continue);

            let word = &text[start..cursor];
            let kind = keyword(word)
                .map(TokenKind::Keyword)
                .unwrap_or(TokenKind::Identifier);

            tokens.push(Token::new(
                kind,
                Span::new(start as u32, cursor as u32),
                &text[start..cursor],
            ));
            continue;
        }

        if ch.is_ascii_digit() {
            consume_while(&mut cursor, text, |next| next.is_ascii_digit());
            if text.as_bytes().get(cursor) == Some(&b'.')
                && text
                    .as_bytes()
                    .get(cursor + 1)
                    .is_some_and(|next| next.is_ascii_digit())
            {
                cursor += 1;
                consume_while(&mut cursor, text, |next| next.is_ascii_digit());
            }

            tokens.push(Token::new(
                TokenKind::Number,
                Span::new(start as u32, cursor as u32),
                &text[start..cursor],
            ));
            continue;
        }

        let punctuation = "[]{}():,.-><-+*/%=;!";
        if punctuation.contains(ch) {
            cursor += ch.len_utf8();
            tokens.push(Token::new(
                TokenKind::Punctuation(ch),
                Span::new(start as u32, cursor as u32),
                &text[start..cursor],
            ));
            continue;
        }

        cursor += ch.len_utf8();
        diagnostics.push(Diagnostic::error(
            "GQL-SYNTAX-UNKNOWN-CHARACTER",
            format!("unrecognized character `{ch}`"),
            Span::new(start as u32, cursor as u32),
        ));
        tokens.push(Token::new(
            TokenKind::Unknown,
            Span::new(start as u32, cursor as u32),
            &text[start..cursor],
        ));
    }

    (tokens, diagnostics)
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_'
}

fn is_identifier_continue(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn consume_while<F>(cursor: &mut usize, text: &str, mut keep: F)
where
    F: FnMut(char) -> bool,
{
    while *cursor < text.len() {
        let next = text[*cursor..]
            .chars()
            .next()
            .expect("cursor must stay in bounds");
        if !keep(next) {
            break;
        }
        *cursor += next.len_utf8();
    }
}
