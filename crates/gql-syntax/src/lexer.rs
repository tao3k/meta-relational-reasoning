//! Lossless lexer for the ISO GQL lexical surface admitted by the active profile.

use crate::character_string::{CharacterStringForm, scan_character_string};
pub(crate) use crate::syntax::keyword;
use crate::syntax::recovery_diagnostic;
use crate::syntax::{Token, TokenKind};
use gql_source::{Diagnostic, Span};
use icu_properties::props::{GeneralCategory, IdContinue, IdStart};
use icu_properties::{
    CodePointMapData, CodePointMapDataBorrowed, CodePointSetData, CodePointSetDataBorrowed,
};

const ID_START: CodePointSetDataBorrowed<'static> = CodePointSetData::new::<IdStart>();
const ID_CONTINUE: CodePointSetDataBorrowed<'static> = CodePointSetData::new::<IdContinue>();
const GENERAL_CATEGORY: CodePointMapDataBorrowed<'static, GeneralCategory> =
    CodePointMapData::<GeneralCategory>::new();

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

        if text[start..].starts_with("//") || text[start..].starts_with("--") {
            cursor += 2;
            consume_while(&mut cursor, text, |next| next != '\r' && next != '\n');
            tokens.push(Token::new(
                TokenKind::Comment,
                Span::new(start as u32, cursor as u32),
                &text[start..cursor],
            ));
            continue;
        }

        if text[start..].starts_with("/*") {
            cursor += 2;
            let mut terminated = false;
            while cursor < text.len() {
                if text[cursor..].starts_with("*/") {
                    cursor += 2;
                    terminated = true;
                    break;
                }
                let next = text[cursor..]
                    .chars()
                    .next()
                    .expect("cursor must stay in bounds");
                cursor += next.len_utf8();
            }
            if !terminated {
                diagnostics.push(Diagnostic::error(
                    recovery_diagnostic("block-comment")
                        .expect("Gerbil grammar declares block comment recovery"),
                    "unterminated block comment",
                    Span::new(start as u32, cursor as u32),
                ));
            }
            tokens.push(Token::new(
                TokenKind::Comment,
                Span::new(start as u32, cursor as u32),
                &text[start..cursor],
            ));
            continue;
        }

        if matches!(ch, 'X' | 'x') && text.as_bytes().get(start + 1) == Some(&b'\'') {
            let scan = scan_byte_string_literal(text.as_bytes(), start);
            cursor = scan.end;
            if !scan.valid {
                diagnostics.push(Diagnostic::error(
                    recovery_diagnostic("byte-string-literal")
                        .expect("Gerbil grammar declares byte string recovery"),
                    "byte string requires terminated hexadecimal byte pairs",
                    Span::new(start as u32, cursor as u32),
                ));
            }
            tokens.push(Token::new(
                TokenKind::ByteString,
                Span::new(start as u32, cursor as u32),
                &text[start..cursor],
            ));
            continue;
        }

        let delimiter = match ch {
            '\'' | '"' | '`' => Some(ch),
            '@' => text[start + ch.len_utf8()..]
                .chars()
                .next()
                .filter(|next| matches!(next, '\'' | '"' | '`')),
            _ => None,
        };
        if delimiter.is_some() {
            let scan = scan_character_string(text, start)
                .expect("quoted dispatch guarantees a character-string scan");
            cursor = scan.end;
            if !scan.terminated {
                let (recovery, message) = if scan.form == CharacterStringForm::SingleQuoted {
                    ("string-literal", "unterminated string literal")
                } else {
                    ("delimited-identifier", "unterminated delimited identifier")
                };
                diagnostics.push(Diagnostic::error(
                    recovery_diagnostic(recovery).expect("Gerbil grammar declares quoted recovery"),
                    message,
                    Span::new(start as u32, cursor as u32),
                ));
            } else if !scan.valid {
                let (recovery, message) = if scan.form == CharacterStringForm::SingleQuoted {
                    (
                        "character-string-literal",
                        "invalid character-string literal representation",
                    )
                } else {
                    (
                        "identifier-escape",
                        "invalid escape in delimited identifier",
                    )
                };
                diagnostics.push(Diagnostic::error(
                    recovery_diagnostic(recovery)
                        .expect("Gerbil grammar declares quoted representation recovery"),
                    message,
                    Span::new(start as u32, cursor as u32),
                ));
            }
            let kind = if scan.form == CharacterStringForm::SingleQuoted {
                TokenKind::String
            } else {
                TokenKind::Identifier
            };
            tokens.push(Token::new(
                kind,
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

        if ch.is_ascii_digit()
            || (ch == '.'
                && text
                    .as_bytes()
                    .get(start + 1)
                    .is_some_and(u8::is_ascii_digit))
        {
            let scan = scan_numeric_literal(text, start);
            cursor = scan.end;
            if !scan.valid {
                diagnostics.push(Diagnostic::error(
                    recovery_diagnostic("numeric-literal")
                        .expect("Gerbil grammar declares numeric literal recovery"),
                    "invalid numeric literal",
                    Span::new(start as u32, cursor as u32),
                ));
            } else if scan.requires_integer_range_check
                && parse_unsigned_integer_i64(&text[start..cursor], scan.radix).is_none()
            {
                diagnostics.push(Diagnostic::error(
                    recovery_diagnostic("integer-literal-range")
                        .expect("Gerbil grammar declares integer literal range recovery"),
                    "integer literal is outside the canonical integer range",
                    Span::new(start as u32, cursor as u32),
                ));
            }
            tokens.push(Token::new(
                TokenKind::Number,
                Span::new(start as u32, cursor as u32),
                &text[start..cursor],
            ));
            continue;
        }

        let punctuation = "[]{}():,.-><-+*/%=;!?|&~";
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

#[derive(Clone, Copy, Debug)]
struct ByteStringScan {
    end: usize,
    valid: bool,
}

fn scan_byte_string_literal(bytes: &[u8], start: usize) -> ByteStringScan {
    let mut cursor = start + 2;
    let mut hex_digits = 0usize;
    let mut valid = true;
    let mut terminated = false;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'\'' => {
                cursor += 1;
                terminated = true;
                break;
            }
            byte if byte.is_ascii_hexdigit() => {
                hex_digits += 1;
                cursor += 1;
            }
            byte if byte.is_ascii_whitespace() => cursor += 1,
            _ => {
                valid = false;
                cursor += 1;
            }
        }
    }
    valid &= terminated && hex_digits > 0 && hex_digits.is_multiple_of(2);
    ByteStringScan { end: cursor, valid }
}

#[derive(Clone, Copy, Debug)]
struct NumericScan {
    end: usize,
    radix: u32,
    valid: bool,
    requires_integer_range_check: bool,
}

fn scan_numeric_literal(text: &str, start: usize) -> NumericScan {
    let bytes = text.as_bytes();
    if bytes.get(start) == Some(&b'0')
        && let Some(prefix) = bytes.get(start + 1).copied()
        && matches!(prefix, b'x' | b'o' | b'b' | b'X' | b'O' | b'B')
    {
        return scan_radix_integer(bytes, start, prefix);
    }

    let mut cursor = start;
    let mut valid = true;
    let mut common_notation = false;
    if bytes.get(cursor) == Some(&b'.') {
        common_notation = true;
        cursor += 1;
        let digits = scan_decimal_digits(bytes, cursor);
        cursor = digits.end;
        valid &= digits.valid && digits.count > 0;
    } else {
        let digits = scan_decimal_digits(bytes, cursor);
        cursor = digits.end;
        valid &= digits.valid;
        if bytes.get(cursor) == Some(&b'.') {
            common_notation = true;
            cursor += 1;
            let fraction = scan_decimal_digits(bytes, cursor);
            cursor = fraction.end;
            valid &= fraction.valid;
        }
    }

    let mut scientific_notation = false;
    if matches!(bytes.get(cursor), Some(b'e' | b'E')) {
        scientific_notation = true;
        cursor += 1;
        if matches!(bytes.get(cursor), Some(b'+' | b'-')) {
            cursor += 1;
        }
        let exponent = scan_decimal_digits(bytes, cursor);
        cursor = exponent.end;
        valid &= exponent.valid && exponent.count > 0;
    }

    let mut has_suffix = false;
    if matches!(
        bytes.get(cursor),
        Some(b'm' | b'M' | b'f' | b'F' | b'd' | b'D')
    ) {
        has_suffix = true;
        cursor += 1;
    }

    if bytes
        .get(cursor)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
    {
        valid = false;
        cursor = consume_ascii_numeric_candidate(bytes, cursor);
    }

    NumericScan {
        end: cursor,
        radix: 10,
        valid,
        requires_integer_range_check: !common_notation && !scientific_notation && !has_suffix,
    }
}

fn scan_radix_integer(bytes: &[u8], start: usize, prefix: u8) -> NumericScan {
    let radix = match prefix {
        b'x' | b'X' => 16,
        b'o' | b'O' => 8,
        b'b' | b'B' => 2,
        _ => unreachable!("numeric prefix already classified"),
    };
    let mut cursor = start + 2;
    let mut valid = prefix.is_ascii_lowercase();
    let mut digit_count = 0usize;
    while let Some(byte) = bytes.get(cursor).copied() {
        if byte == b'_' {
            let Some(next) = bytes.get(cursor + 1).copied() else {
                cursor += 1;
                valid = false;
                break;
            };
            if !is_radix_digit(next, radix) {
                cursor += 1;
                valid = false;
                continue;
            }
            cursor += 1;
            continue;
        }
        if is_radix_digit(byte, radix) {
            digit_count += 1;
            cursor += 1;
            continue;
        }
        if byte.is_ascii_alphanumeric() {
            valid = false;
            cursor = consume_ascii_numeric_candidate(bytes, cursor);
        }
        break;
    }
    valid &= digit_count > 0;
    NumericScan {
        end: cursor,
        radix,
        valid,
        requires_integer_range_check: true,
    }
}

#[derive(Clone, Copy, Debug)]
struct DigitScan {
    end: usize,
    count: usize,
    valid: bool,
}

fn scan_decimal_digits(bytes: &[u8], start: usize) -> DigitScan {
    let mut cursor = start;
    let mut count = 0usize;
    let mut valid = true;
    let mut previous_was_digit = false;
    while let Some(byte) = bytes.get(cursor).copied() {
        if byte.is_ascii_digit() {
            count += 1;
            previous_was_digit = true;
            cursor += 1;
        } else if byte == b'_' {
            let next_is_digit = bytes.get(cursor + 1).is_some_and(u8::is_ascii_digit);
            valid &= previous_was_digit && next_is_digit;
            previous_was_digit = false;
            cursor += 1;
        } else {
            break;
        }
    }
    DigitScan {
        end: cursor,
        count,
        valid,
    }
}

fn consume_ascii_numeric_candidate(bytes: &[u8], mut cursor: usize) -> usize {
    while bytes
        .get(cursor)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
    {
        cursor += 1;
    }
    cursor
}

fn is_radix_digit(byte: u8, radix: u32) -> bool {
    match radix {
        2 => matches!(byte, b'0' | b'1'),
        8 => matches!(byte, b'0'..=b'7'),
        16 => byte.is_ascii_hexdigit(),
        _ => false,
    }
}

fn parse_unsigned_integer_i64(text: &str, radix: u32) -> Option<i64> {
    let digits = if radix == 10 { text } else { &text[2..] };
    digits
        .bytes()
        .filter(|byte| *byte != b'_')
        .try_fold(0i64, |value, digit| {
            value
                .checked_mul(i64::from(radix))?
                .checked_add(i64::from(char::from(digit).to_digit(radix)?))
        })
}

fn is_identifier_start(ch: char) -> bool {
    ID_START.contains(ch) || GENERAL_CATEGORY.get(ch) == GeneralCategory::ConnectorPunctuation
}

fn is_identifier_continue(ch: char) -> bool {
    ID_CONTINUE.contains(ch)
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
