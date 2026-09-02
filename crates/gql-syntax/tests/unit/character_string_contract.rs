use crate::{CharacterStringForm, decode_character_string};
use std::borrow::Cow;

#[test]
fn decoding_borrows_plain_sequences_and_allocates_only_for_folding() {
    let plain = decode_character_string("'plain'").expect("plain sequence");
    assert!(matches!(plain.value, Cow::Borrowed("plain")));

    let escaped = decode_character_string(r"'A\n\u0042\U01F642'").expect("escaped sequence");
    assert_eq!(escaped.value, "A\nB\u{1f642}");
    assert!(matches!(escaped.value, Cow::Owned(_)));

    let raw = decode_character_string(r"@'A\nB'").expect("no-escape sequence");
    assert_eq!(raw.value, r"A\nB");
    assert_eq!(raw.form, CharacterStringForm::SingleQuoted);
    assert!(raw.no_escape);
}

#[test]
fn decoding_rejects_unknown_short_and_non_scalar_escapes() {
    for source in [r"'\q'", r"'\u12'", r"'\u12G4'", r"'\uD800'", r"'\U110000'"] {
        assert!(
            decode_character_string(source).is_none(),
            "invalid representation admitted: {source:?}"
        );
    }
}

#[test]
fn no_escape_preserves_reverse_solidus_but_folds_the_delimiter() {
    let decoded = decode_character_string(r"@'Ada''s\n'").expect("NO_ESCAPE sequence");
    assert_eq!(decoded.value, r"Ada's\n");
    assert!(decoded.no_escape);
}
