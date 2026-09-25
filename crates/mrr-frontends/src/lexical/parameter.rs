// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

//! ISO GQL parameter semantic-name decoding.

use std::borrow::Cow;

use icu_properties::CodePointSetData;
use icu_properties::props::IdContinue;

use super::character_string::{CharacterStringForm, decode_character_string};

const PARAMETER_NAME_CONTINUE: icu_properties::CodePointSetDataBorrowed<'static> =
    CodePointSetData::new::<IdContinue>();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ParameterNameForm {
    Extended,
    Delimited,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DecodedParameterReference<'a> {
    pub(crate) name: Cow<'a, str>,
    pub(crate) form: ParameterNameForm,
}

#[must_use]
pub(crate) fn decode_parameter_reference(text: &str) -> Option<DecodedParameterReference<'_>> {
    let name = if let Some(name) = text.strip_prefix("$$") {
        name
    } else {
        text.strip_prefix('$')?
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
            CharacterStringForm::Double | CharacterStringForm::Grave
        ) {
            return None;
        }
        return Some(DecodedParameterReference {
            name: decoded.value,
            form: ParameterNameForm::Delimited,
        });
    }
    if !name.chars().all(|ch| PARAMETER_NAME_CONTINUE.contains(ch)) {
        return None;
    }
    Some(DecodedParameterReference {
        name: Cow::Borrowed(name),
        form: ParameterNameForm::Extended,
    })
}
