//! Private lexical value decoding for parser-owned CST projection.

mod character_string;
mod parameter;

pub(crate) use character_string::{CharacterStringForm, decode_character_string};
pub(crate) use parameter::{ParameterNameForm, decode_parameter_reference};
