//! ISO GQL quoted character-sequence scanning and decoding.

#![forbid(unsafe_code)]

use std::borrow::Cow;

/// The syntactic delimiter used by a quoted character sequence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CharacterStringForm {
    SingleQuoted,
    DoubleQuoted,
    GraveQuoted,
}

impl CharacterStringForm {
    const fn delimiter(self) -> char {
        match self {
            Self::SingleQuoted => '\'',
            Self::DoubleQuoted => '"',
            Self::GraveQuoted => '`',
        }
    }
}

/// A validated and decoded quoted character sequence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedCharacterString<'a> {
    pub value: Cow<'a, str>,
    pub form: CharacterStringForm,
    pub no_escape: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CharacterStringScan {
    pub end: usize,
    pub terminated: bool,
    pub valid: bool,
    pub form: CharacterStringForm,
    pub no_escape: bool,
}

pub(crate) fn scan_character_string(text: &str, start: usize) -> Option<CharacterStringScan> {
    let mut cursor = start;
    let no_escape = text.as_bytes().get(cursor) == Some(&b'@');
    if no_escape {
        cursor += 1;
    }
    let delimiter = text.get(cursor..)?.chars().next()?;
    let form = match delimiter {
        '\'' => CharacterStringForm::SingleQuoted,
        '"' => CharacterStringForm::DoubleQuoted,
        '`' => CharacterStringForm::GraveQuoted,
        _ => return None,
    };
    cursor += delimiter.len_utf8();
    let mut valid = true;
    let mut terminated = false;

    while cursor < text.len() {
        let next = text[cursor..]
            .chars()
            .next()
            .expect("character-string cursor must stay in bounds");
        if next == delimiter {
            let after = cursor + delimiter.len_utf8();
            if text[after..].starts_with(delimiter) {
                cursor = after + delimiter.len_utf8();
                continue;
            }
            cursor = after;
            terminated = true;
            break;
        }
        if matches!(next, '\r' | '\n') {
            valid = false;
            cursor += next.len_utf8();
            continue;
        }
        if next == '\\' && !no_escape {
            let escape = scan_escape(text, cursor);
            valid &= escape.valid;
            cursor = escape.end;
            continue;
        }
        cursor += next.len_utf8();
    }

    Some(CharacterStringScan {
        end: cursor,
        terminated,
        valid,
        form,
        no_escape,
    })
}

#[derive(Clone, Copy)]
struct EscapeScan {
    end: usize,
    valid: bool,
}

fn scan_escape(text: &str, start: usize) -> EscapeScan {
    let mut cursor = start + 1;
    let Some(escaped) = text.get(cursor..).and_then(|tail| tail.chars().next()) else {
        return EscapeScan {
            end: cursor,
            valid: false,
        };
    };
    cursor += escaped.len_utf8();
    if matches!(
        escaped,
        '\\' | '\'' | '"' | '`' | 't' | 'b' | 'n' | 'r' | 'f'
    ) {
        return EscapeScan {
            end: cursor,
            valid: true,
        };
    }
    if !matches!(escaped, 'u' | 'U') {
        return EscapeScan {
            end: cursor,
            valid: false,
        };
    }

    let width = if escaped == 'u' { 4 } else { 6 };
    let digits_start = cursor;
    for _ in 0..width {
        let Some(byte) = text.as_bytes().get(cursor).copied() else {
            return EscapeScan {
                end: cursor,
                valid: false,
            };
        };
        if !byte.is_ascii_hexdigit() {
            return EscapeScan {
                end: cursor,
                valid: false,
            };
        }
        cursor += 1;
    }
    let scalar = u32::from_str_radix(&text[digits_start..cursor], 16)
        .ok()
        .and_then(char::from_u32);
    EscapeScan {
        end: cursor,
        valid: scalar.is_some(),
    }
}

/// Validates and decodes one complete ISO GQL quoted character sequence.
///
/// Unescaped inputs borrow their source slice. An allocation is performed only
/// when an escape or a doubled delimiter must be folded.
pub fn decode_character_string(text: &str) -> Option<DecodedCharacterString<'_>> {
    let scan = scan_character_string(text, 0)?;
    if !scan.terminated || !scan.valid || scan.end != text.len() {
        return None;
    }
    let delimiter = scan.form.delimiter();
    let prefix = usize::from(scan.no_escape) + delimiter.len_utf8();
    let inner = &text[prefix..text.len() - delimiter.len_utf8()];
    let doubled = match delimiter {
        '\'' => "''",
        '"' => "\"\"",
        '`' => "``",
        _ => unreachable!("character-string delimiter is closed"),
    };
    if !inner.contains(doubled) && (scan.no_escape || !inner.contains('\\')) {
        return Some(DecodedCharacterString {
            value: Cow::Borrowed(inner),
            form: scan.form,
            no_escape: scan.no_escape,
        });
    }

    let mut decoded = String::with_capacity(inner.len());
    let mut cursor = 0usize;
    while cursor < inner.len() {
        if inner[cursor..].starts_with(doubled) {
            decoded.push(delimiter);
            cursor += doubled.len();
            continue;
        }
        let next = inner[cursor..]
            .chars()
            .next()
            .expect("decoded cursor must stay in bounds");
        if next != '\\' || scan.no_escape {
            decoded.push(next);
            cursor += next.len_utf8();
            continue;
        }
        let escaped = inner[cursor + 1..]
            .chars()
            .next()
            .expect("validated escape must have a payload");
        cursor += 1 + escaped.len_utf8();
        match escaped {
            '\\' => decoded.push('\\'),
            '\'' => decoded.push('\''),
            '"' => decoded.push('"'),
            '`' => decoded.push('`'),
            't' => decoded.push('\t'),
            'b' => decoded.push('\u{0008}'),
            'n' => decoded.push('\n'),
            'r' => decoded.push('\r'),
            'f' => decoded.push('\u{000c}'),
            'u' | 'U' => {
                let width = if escaped == 'u' { 4 } else { 6 };
                let scalar = u32::from_str_radix(&inner[cursor..cursor + width], 16).ok()?;
                decoded.push(char::from_u32(scalar)?);
                cursor += width;
            }
            _ => return None,
        }
    }
    Some(DecodedCharacterString {
        value: Cow::Owned(decoded),
        form: scan.form,
        no_escape: scan.no_escape,
    })
}
