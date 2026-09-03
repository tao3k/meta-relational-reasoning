//! ISO GQL identifier spelling and escape normalization.
#![forbid(unsafe_code)]

use super::{Identifier, IdentifierForm};
use gql_syntax::{CharacterStringForm, Token, decode_character_string};

pub(super) fn identifier_from_token(token: &Token, _source: &str) -> Identifier {
    let (text, form) = match decode_character_string(token.text()) {
        Some(decoded)
            if matches!(
                decoded.form,
                CharacterStringForm::DoubleQuoted | CharacterStringForm::GraveQuoted
            ) =>
        {
            (decoded.value.into_owned(), IdentifierForm::Delimited)
        }
        _ => (token.text().to_owned(), IdentifierForm::Undelimited),
    };
    Identifier {
        text,
        span: token.span,
        form,
    }
}
