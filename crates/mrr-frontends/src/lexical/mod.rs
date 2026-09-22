//! Private lexical value decoding for parser-owned CST projection.

mod character_string;
mod numeric;
mod parameter;

pub(crate) use character_string::{CharacterStringForm, decode_character_string};
pub(crate) use numeric::{NumericLiteral, decode_numeric_literal};
#[cfg(test)]
pub(crate) use parameter::ParameterNameForm;
pub(crate) use parameter::decode_parameter_reference;
