//! ISO GQL parameter-reference scanning and semantic-name decoding.
#![forbid(unsafe_code)]

use std::borrow::Cow;

use crate::character_string::{
    CharacterStringForm, decode_character_string, scan_character_string,
};
use icu_properties::CodePointSetData;
use icu_properties::props::IdContinue;

const PARAMETER_NAME_CONTINUE: icu_properties::CodePointSetDataBorrowed<'static> =
    CodePointSetData::new::<IdContinue>();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ParameterReferenceKind {
    Dynamic,
    Substituted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ParameterScan {
    pub(crate) end: usize,
    pub(crate) kind: ParameterReferenceKind,
    pub(crate) valid: bool,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
/// Lexical form of the separated identifier following a parameter prefix.
pub enum ParameterNameForm {
    /// One or more Unicode identifier-continue code points.
    Extended,
    /// A decoded double-quote or grave-accent delimited name.
    Delimited,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Decoded parameter identity plus the source spelling class retained by the AST.
pub struct DecodedParameterReference<'a> {
    /// Semantic parameter name without `$` prefixes or delimiters.
    pub name: Cow<'a, str>,
    /// Source-level parameter-name spelling class.
    pub form: ParameterNameForm,
    /// Whether the source used the catalog-reference `$$` prefix.
    pub substituted: bool,
}

pub(crate) fn scan_parameter_reference(
    text: &str,
    start: usize,
    is_identifier_continue: impl Fn(char) -> bool,
) -> ParameterScan {
    debug_assert_eq!(text.as_bytes().get(start), Some(&b'$'));
    let substituted = text.as_bytes().get(start + 1) == Some(&b'$');
    let prefix_len = if substituted { 2 } else { 1 };
    let name_start = start + prefix_len;
    let kind = if substituted {
        ParameterReferenceKind::Substituted
    } else {
        ParameterReferenceKind::Dynamic
    };

    let quoted_start = match text[name_start..].chars().next() {
        Some('"' | '`') => Some(name_start),
        Some('@')
            if text[name_start + 1..]
                .chars()
                .next()
                .is_some_and(|delimiter| matches!(delimiter, '"' | '`')) =>
        {
            Some(name_start)
        }
        _ => None,
    };
    if let Some(quoted_start) = quoted_start {
        let scan = scan_character_string(text, quoted_start)
            .expect("parameter quote dispatch guarantees a character-sequence scan");
        return ParameterScan {
            end: scan.end,
            kind,
            valid: scan.terminated
                && scan.valid
                && matches!(
                    scan.form,
                    CharacterStringForm::DoubleQuoted | CharacterStringForm::GraveQuoted
                ),
        };
    }

    let mut end = name_start;
    while let Some(ch) = text[end..]
        .chars()
        .next()
        .filter(|ch| is_identifier_continue(*ch))
    {
        end += ch.len_utf8();
    }
    ParameterScan {
        end,
        kind,
        valid: end > name_start,
    }
}

#[must_use]
/// Decodes a complete general or substituted parameter token.
pub fn decode_parameter_reference(text: &str) -> Option<DecodedParameterReference<'_>> {
    let (substituted, name) = if let Some(name) = text.strip_prefix("$$") {
        (true, name)
    } else {
        (false, text.strip_prefix('$')?)
    };
    if name.is_empty() {
        return None;
    }
    if name.starts_with('"')
        || name.starts_with('`')
        || name.starts_with("@\"")
        || name.starts_with("@`")
    {
        let decoded = decode_character_string(name)?;
        if !matches!(
            decoded.form,
            CharacterStringForm::DoubleQuoted | CharacterStringForm::GraveQuoted
        ) {
            return None;
        }
        return Some(DecodedParameterReference {
            name: decoded.value,
            form: ParameterNameForm::Delimited,
            substituted,
        });
    }
    if !name.chars().all(|ch| PARAMETER_NAME_CONTINUE.contains(ch)) {
        return None;
    }
    Some(DecodedParameterReference {
        name: Cow::Borrowed(name),
        form: ParameterNameForm::Extended,
        substituted,
    })
}
