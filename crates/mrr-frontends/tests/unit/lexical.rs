use crate::lexical::{
    ParameterNameForm, decode_character_string, decode_parameter_reference,
};

#[test]
fn character_decoder_preserves_iso_escape_and_no_escape_semantics() {
    assert_eq!(
        decode_character_string(r"'A\nB'")
            .expect("escaped character string")
            .value,
        "A\nB"
    );
    assert_eq!(
        decode_character_string(r"@'A\nB'")
            .expect("no-escape character string")
            .value,
        r"A\nB"
    );
}

#[test]
fn parameter_decoder_uses_unicode_identifier_continue() {
    let extended = decode_parameter_reference("$limit").expect("extended parameter");
    assert_eq!(extended.name, "limit");
    assert_eq!(extended.form, ParameterNameForm::Extended);

    let delimited = decode_parameter_reference("$\"limit\"").expect("delimited parameter");
    assert_eq!(delimited.name, "limit");
    assert_eq!(delimited.form, ParameterNameForm::Delimited);

    assert!(decode_parameter_reference("$-").is_none());
}
