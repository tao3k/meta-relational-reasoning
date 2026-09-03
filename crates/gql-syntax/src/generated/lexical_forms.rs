// mrr.gerbil-grammar-projection.v1 input-sha256=6ea6b2df8eb55b77191e0b6bf110b75af1f1d399a12735c0858a0c66624d7dfc body-sha256=8874dca34c15e386320720e30b1a142e257096eaeebccf4500074d3a2baf7bc0 gerbil-scheme-rust-rev=a83fb649ddbbeaabdb538a6eaf0ded10838f7fad
// Generated through the Gerbil native AOT bindings; do not edit.
//! Lexical form tables projected from the Gerbil grammar authority.
/// ISO GQL non-reserved words admitted as regular identifiers.
pub const ISO_GQL_NON_RESERVED_WORDS: &str = "ACYCLIC BINDING BINDINGS CONNECTING DESTINATION DIFFERENT DIRECTED EDGE EDGES ELEMENT ELEMENTS FIRST GRAPH GROUPS KEEP LABEL LABELED LABELS LAST NFC NFD NFKC NFKD NO NODE NORMALIZED ONLY ORDINALITY PROPERTY READ RELATIONSHIP RELATIONSHIPS REPEATABLE SHORTEST SIMPLE SOURCE TABLE TO TRAIL TRANSACTION TYPE UNDIRECTED VERTEX WALK WITHOUT WRITE ZONE";
/// Returns whether `word` is an ISO GQL non-reserved word.
pub fn is_non_reserved_word(word: &str) -> bool {
    ISO_GQL_NON_RESERVED_WORDS
        .split_ascii_whitespace()
        .any(|candidate| word.eq_ignore_ascii_case(candidate))
}
/// Gerbil-owned ISO GQL numeric literal forms: form, notation, suffix, semantic class.
pub const ISO_GQL_NUMERIC_LITERAL_FORMS: &[(&str, &str, &str, &str)] = &[
    ("exact-scientific", "scientific", "M", "exact"),
    ("exact-common", "common", "M", "exact"),
    ("exact-common-unsuffixed", "common", "none", "exact"),
    ("exact-integer", "integer", "M", "exact"),
    ("unsigned-integer", "integer", "none", "integer"),
    ("approximate-scientific", "scientific", "FD", "approximate"),
    (
        "approximate-scientific-unsuffixed",
        "scientific",
        "none",
        "approximate",
    ),
    ("approximate-common", "common", "FD", "approximate"),
    ("approximate-integer", "integer", "FD", "approximate"),
];
/// Gerbil-owned character-string forms and escape actions.
pub const ISO_GQL_CHARACTER_STRING_FORMS: &[(&str, &str, &str, &str)] = &[
    (
        "single-quoted",
        "quote",
        "escaped-or-doubled",
        "character-string",
    ),
    (
        "double-quoted",
        "double-quote",
        "escaped-or-doubled",
        "character-string",
    ),
    (
        "no-escape",
        "commercial-at",
        "preserve-representations",
        "raw",
    ),
    (
        "escaped-reverse-solidus",
        "reverse-solidus",
        "decode",
        "scalar",
    ),
    ("escaped-quote", "quote", "decode", "scalar"),
    ("escaped-double-quote", "double-quote", "decode", "scalar"),
    ("escaped-grave-accent", "grave-accent", "decode", "scalar"),
    ("escaped-tab", "t", "decode", "control"),
    ("escaped-backspace", "b", "decode", "control"),
    ("escaped-new-line", "n", "decode", "control"),
    ("escaped-carriage-return", "r", "decode", "control"),
    ("escaped-form-feed", "f", "decode", "control"),
    ("escaped-unicode4", "u", "decode", "four-hex-digits"),
    ("escaped-unicode6", "U", "decode", "six-hex-digits"),
];
/// Gerbil-owned parameter reference forms: form, prefix, name grammar, semantic context.
pub const ISO_GQL_PARAMETER_REFERENCE_FORMS: &[(&str, &str, &str, &str)] = &[
    ("general", "dollar", "separated-identifier", "dynamic-value"),
    (
        "substituted",
        "double-dollar",
        "separated-identifier",
        "catalog-reference",
    ),
];
/// Gerbil-owned postfix predicate tests: kind, negation, value, operand domain.
pub const ISO_GQL_PREDICATE_TEST_FORMS: &[(&str, &str, &str, &str)] = &[
    ("null", "optional-not", "Null", "any-value"),
    ("truth", "optional-not", "True", "boolean-or-null"),
    ("truth", "optional-not", "False", "boolean-or-null"),
    ("truth", "optional-not", "UnknownTruth", "boolean-or-null"),
    (
        "value-type",
        "optional-not",
        "declared-value-type",
        "value-primary",
    ),
    ("directed", "optional-not", "Directed", "edge-element"),
    ("source", "optional-not", "Source", "node-edge"),
    ("destination", "optional-not", "Destination", "node-edge"),
    (
        "all-different",
        "forbidden",
        "AllDifferent",
        "element-list-min-two",
    ),
    ("same", "forbidden", "Same", "element-list-min-two"),
    (
        "property-exists",
        "forbidden",
        "PropertyExists",
        "element-property",
    ),
];
/// Gerbil-owned aggregate forms: semantic name, keyword, family, quantifier policy, arity.
pub const ISO_GQL_AGGREGATE_FUNCTION_FORMS: &[(&str, &str, &str, &str, u8)] = &[
    ("count-star", "Count", "star", "forbidden", 0),
    ("average", "Avg", "general", "optional", 1),
    ("count", "Count", "general", "optional", 1),
    ("maximum", "Max", "general", "optional", 1),
    ("minimum", "Min", "general", "optional", 1),
    ("sum", "Sum", "general", "optional", 1),
    ("collect-list", "CollectList", "general", "optional", 1),
    (
        "standard-deviation-sample",
        "StddevSamp",
        "general",
        "optional",
        1,
    ),
    (
        "standard-deviation-population",
        "StddevPop",
        "general",
        "optional",
        1,
    ),
    (
        "percentile-continuous",
        "PercentileCont",
        "binary",
        "dependent",
        2,
    ),
    (
        "percentile-discrete",
        "PercentileDisc",
        "binary",
        "dependent",
        2,
    ),
];
