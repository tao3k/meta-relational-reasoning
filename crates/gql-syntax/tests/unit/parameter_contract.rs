use crate::{ParameterNameForm, decode_parameter_reference};

#[test]
fn parameter_decoder_preserves_prefix_form_and_semantic_name() {
    let cases = [
        ("$limit", "limit", ParameterNameForm::Extended, false),
        ("$42", "42", ParameterNameForm::Extended, false),
        ("$\"MATCH\"", "MATCH", ParameterNameForm::Delimited, false),
        ("$`say``hi`", "say`hi", ParameterNameForm::Delimited, false),
        (
            "$@\"raw\\n\"",
            "raw\\n",
            ParameterNameForm::Delimited,
            false,
        ),
        ("$$catalog", "catalog", ParameterNameForm::Extended, true),
    ];

    for (source, expected_name, expected_form, expected_substituted) in cases {
        let decoded = decode_parameter_reference(source).expect("valid parameter reference");
        assert_eq!(decoded.name, expected_name, "source: {source}");
        assert_eq!(decoded.form, expected_form, "source: {source}");
        assert_eq!(
            decoded.substituted, expected_substituted,
            "source: {source}"
        );
    }
}

#[test]
fn parameter_decoder_rejects_incomplete_or_wrong_delimiter_tokens() {
    for source in ["", "$", "$$", "$'name'", "$$'name'"] {
        assert!(
            decode_parameter_reference(source).is_none(),
            "source must be rejected: {source}"
        );
    }
}
