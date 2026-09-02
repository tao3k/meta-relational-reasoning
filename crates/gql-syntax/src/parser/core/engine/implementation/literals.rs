//! Structural parser and lexical validators for general literal values.
#![forbid(unsafe_code)]

use super::{Event, Parser, node};
use crate::decode_character_string;
use crate::syntax::{Keyword, SyntaxKind, TokenKind, recovery_diagnostic};

fn is_character_string_token(kind: TokenKind, text: &str) -> bool {
    kind == TokenKind::String
        || kind == TokenKind::Identifier && (text.starts_with('"') || text.starts_with("@\""))
}

fn valid_temporal_literal(qualifier: Keyword, text: &str) -> bool {
    let Some(decoded) = decode_character_string(text) else {
        return false;
    };
    let value = decoded.value.as_ref();
    match qualifier {
        Keyword::Date => valid_date(value),
        Keyword::Time => valid_time(value),
        Keyword::Timestamp | Keyword::Datetime => value
            .split_once(['T', 't', ' '])
            .is_some_and(|(date, time)| valid_date(date) && valid_time(time)),
        _ => false,
    }
}

fn decimal_component(bytes: &[u8]) -> Option<u32> {
    bytes.iter().try_fold(0u32, |value, byte| {
        byte.is_ascii_digit()
            .then(|| value * 10 + u32::from(byte - b'0'))
    })
}

fn valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let (Some(year), Some(month), Some(day)) = (
        decimal_component(&bytes[..4]),
        decimal_component(&bytes[5..7]),
        decimal_component(&bytes[8..]),
    ) else {
        return false;
    };
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let maximum = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=maximum).contains(&day)
}

fn valid_time(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 8 || bytes[2] != b':' || bytes[5] != b':' {
        return false;
    }
    let (Some(hour), Some(minute), Some(second)) = (
        decimal_component(&bytes[..2]),
        decimal_component(&bytes[3..5]),
        decimal_component(&bytes[6..8]),
    ) else {
        return false;
    };
    if hour > 23 || minute > 59 || second > 59 {
        return false;
    }
    let mut suffix = &bytes[8..];
    if suffix.first() == Some(&b'.') {
        let digits = suffix[1..]
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if digits == 0 {
            return false;
        }
        suffix = &suffix[digits + 1..];
    }
    match suffix {
        [] | [b'Z' | b'z'] => true,
        [b'+' | b'-', h1, h2, b':', m1, m2] => {
            decimal_component(&[*h1, *h2]).is_some_and(|hours| hours <= 23)
                && decimal_component(&[*m1, *m2]).is_some_and(|minutes| minutes <= 59)
        }
        _ => false,
    }
}

fn valid_duration_literal(text: &str) -> bool {
    let Some(decoded) = decode_character_string(text) else {
        return false;
    };
    let value = decoded.value.as_ref();
    let bytes = value.as_bytes();
    let mut cursor = usize::from(bytes.first() == Some(&b'-'));
    if bytes.get(cursor) != Some(&b'P') {
        return false;
    }
    cursor += 1;
    let mut in_time = false;
    let mut component_count = 0usize;
    while cursor < bytes.len() {
        if bytes[cursor] == b'T' && !in_time {
            in_time = true;
            cursor += 1;
            continue;
        }
        let number_start = cursor;
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
        if bytes.get(cursor) == Some(&b'.') {
            cursor += 1;
            let fraction_start = cursor;
            while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
                cursor += 1;
            }
            if cursor == fraction_start {
                return false;
            }
        }
        if cursor == number_start || cursor >= bytes.len() {
            return false;
        }
        let designator = bytes[cursor];
        let valid_designator = if in_time {
            matches!(designator, b'H' | b'M' | b'S')
        } else {
            matches!(designator, b'Y' | b'M' | b'W' | b'D')
        };
        if !valid_designator {
            return false;
        }
        component_count += 1;
        cursor += 1;
    }
    component_count > 0
}

impl Parser<'_> {
    pub(in crate::parser) fn parse_list_expression(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        let mut after_comma = false;
        loop {
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation(']')) {
                if after_comma {
                    self.emit_match_syntax(
                        recovery_diagnostic("list-literal")
                            .expect("Gerbil grammar owns list literal recovery"),
                        "list value cannot end with a trailing comma",
                        self.next_span_or(start),
                    );
                }
                children.push(self.bump_event());
                break;
            }
            if self.at_eof() {
                self.emit_match_syntax(
                    recovery_diagnostic("list-literal")
                        .expect("Gerbil grammar owns list literal recovery"),
                    "list value is missing `]`",
                    self.next_span_or(start),
                );
                break;
            }
            children.extend(self.parse_expression());
            after_comma = false;
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation(',')) {
                children.push(self.bump_event());
                after_comma = true;
                continue;
            }
            if self.matches_kind(TokenKind::Punctuation(']')) {
                children.push(self.bump_event());
                break;
            }
            self.emit_match_syntax(
                recovery_diagnostic("list-literal")
                    .expect("Gerbil grammar owns list literal recovery"),
                "list value requires `,` or `]`",
                self.next_span_or(start),
            );
            if !self.at_eof() {
                children.push(self.bump_event());
            }
        }
        node(SyntaxKind::ListExpression, children)
    }

    pub(in crate::parser) fn parse_temporal_literal(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let qualifier = match self.current_kind() {
            Some(TokenKind::Keyword(
                keyword @ (Keyword::Date | Keyword::Time | Keyword::Timestamp | Keyword::Datetime),
            )) => keyword,
            _ => unreachable!("temporal parser requires a temporal qualifier"),
        };
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        let valid = self.current().is_some_and(|token| {
            is_character_string_token(token.kind, token.text())
                && valid_temporal_literal(qualifier, token.text())
        });
        if self
            .current()
            .is_some_and(|token| is_character_string_token(token.kind, token.text()))
        {
            children.push(self.bump_event());
        }
        if !valid {
            self.emit_match_syntax(
                recovery_diagnostic("temporal-literal")
                    .expect("Gerbil grammar owns temporal literal recovery"),
                "temporal literal does not match its qualifier",
                self.next_span_or(start),
            );
        }
        node(SyntaxKind::TemporalLiteralExpression, children)
    }

    pub(in crate::parser) fn parse_duration_literal(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        let valid = self.current().is_some_and(|token| {
            is_character_string_token(token.kind, token.text())
                && valid_duration_literal(token.text())
        });
        if self
            .current()
            .is_some_and(|token| is_character_string_token(token.kind, token.text()))
        {
            children.push(self.bump_event());
        }
        if !valid {
            self.emit_match_syntax(
                recovery_diagnostic("duration-literal")
                    .expect("Gerbil grammar owns duration literal recovery"),
                "duration literal requires an ISO 8601 duration character sequence",
                self.next_span_or(start),
            );
        }
        node(SyntaxKind::DurationLiteralExpression, children)
    }

    pub(in crate::parser) fn parse_record_expression(&mut self, prefixed: bool) -> Vec<Event> {
        let start = self.span_start();
        let mut children = Vec::new();
        if prefixed {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
        }
        if !self.matches_kind(TokenKind::Punctuation('{')) {
            self.emit_match_syntax(
                recovery_diagnostic("record-literal")
                    .expect("Gerbil grammar owns record literal recovery"),
                "record value requires `{`",
                self.next_span_or(start),
            );
            return node(SyntaxKind::RecordExpression, children);
        }
        children.push(self.bump_event());
        let mut after_comma = false;
        loop {
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation('}')) {
                if after_comma {
                    self.emit_match_syntax(
                        recovery_diagnostic("record-literal")
                            .expect("Gerbil grammar owns record literal recovery"),
                        "record value does not permit a trailing comma",
                        self.next_span_or(start),
                    );
                }
                children.push(self.bump_event());
                break;
            }
            if self.at_eof() {
                self.emit_match_syntax(
                    recovery_diagnostic("record-literal")
                        .expect("Gerbil grammar owns record literal recovery"),
                    "record value is missing `}`",
                    self.next_span_or(start),
                );
                break;
            }
            after_comma = false;
            let mut entry = Vec::new();
            if self.matches_identifier() {
                entry.push(self.bump_event());
            } else {
                self.emit_match_syntax(
                    recovery_diagnostic("record-literal")
                        .expect("Gerbil grammar owns record literal recovery"),
                    "record field requires a field name",
                    self.next_span_or(start),
                );
                entry.push(self.bump_event());
            }
            entry.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation(':')) {
                entry.push(self.bump_event());
            } else {
                self.emit_match_syntax(
                    recovery_diagnostic("record-literal")
                        .expect("Gerbil grammar owns record literal recovery"),
                    "record field requires `:`",
                    self.next_span_or(start),
                );
            }
            entry.extend(self.skip_trivia());
            if self
                .current_kind()
                .is_some_and(|kind| self.is_expression_start(kind))
            {
                entry.extend(self.parse_expression());
            } else {
                self.emit_match_syntax(
                    recovery_diagnostic("record-literal")
                        .expect("Gerbil grammar owns record literal recovery"),
                    "record field requires a value expression",
                    self.next_span_or(start),
                );
            }
            children.extend(node(SyntaxKind::RecordEntry, entry));
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation(',')) {
                children.push(self.bump_event());
                after_comma = true;
                continue;
            }
            if self.matches_kind(TokenKind::Punctuation('}')) {
                children.push(self.bump_event());
                break;
            }
            self.emit_match_syntax(
                recovery_diagnostic("record-literal")
                    .expect("Gerbil grammar owns record literal recovery"),
                "record value requires `,` or `}`",
                self.next_span_or(start),
            );
            if !self.at_eof() {
                children.push(self.bump_event());
            }
        }
        node(SyntaxKind::RecordExpression, children)
    }
}
