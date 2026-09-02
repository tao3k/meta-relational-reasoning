use crate::lexer::keyword;
use crate::syntax::{
    GrammarParserAction, Keyword, TokenKind, binary_operator_spec, grammar_projection_receipt,
    prefix_operator_precedence, top_level_parser_entrypoint,
};
use crate::{
    ISO_GQL_CHARACTER_STRING_FORMS, ISO_GQL_NON_RESERVED_WORDS, ISO_GQL_NUMERIC_LITERAL_FORMS,
    SyntaxKind, is_non_reserved_word, parse,
};
use gql_source::Span;

#[test]
fn gerbil_projection_owns_keywords_shapes_and_provenance() {
    let receipt = grammar_projection_receipt();

    assert_eq!(receipt.schema, "mrr.gerbil-grammar-projection.v1");
    assert_eq!(
        receipt.bridge_revision,
        "a83fb649ddbbeaabdb538a6eaf0ded10838f7fad"
    );
    assert_eq!(receipt.input_sha256.len(), 64);
    assert_eq!(receipt.body_sha256.len(), 64);
    assert!(
        receipt
            .input_sha256
            .bytes()
            .chain(receipt.body_sha256.bytes())
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    );

    assert_eq!(keyword("match"), Some(Keyword::Match));
    assert_eq!(keyword("CaSe"), Some(Keyword::Case));
    assert_eq!(keyword("not_a_keyword"), None);
    assert_eq!(
        ISO_GQL_NON_RESERVED_WORDS.split_ascii_whitespace().count(),
        47
    );
    assert!(is_non_reserved_word("graph"));
    assert!(is_non_reserved_word("ZoNe"));
    assert!(!is_non_reserved_word("MATCH"));
    assert_eq!(ISO_GQL_NUMERIC_LITERAL_FORMS.len(), 9);
    assert!(ISO_GQL_NUMERIC_LITERAL_FORMS.contains(&(
        "exact-scientific",
        "scientific",
        "M",
        "exact",
    )));
    assert_eq!(ISO_GQL_CHARACTER_STRING_FORMS.len(), 14);
    assert!(ISO_GQL_CHARACTER_STRING_FORMS.contains(&(
        "no-escape",
        "commercial-at",
        "preserve-representations",
        "raw",
    )));
    assert!(ISO_GQL_CHARACTER_STRING_FORMS.contains(&(
        "escaped-unicode6",
        "U",
        "decode",
        "six-hex-digits",
    )));
    assert!(ISO_GQL_NUMERIC_LITERAL_FORMS.contains(&(
        "approximate-scientific-unsuffixed",
        "scientific",
        "none",
        "approximate",
    )));

    assert!(receipt.syntax_shapes.contains(&(
        "BinaryExpression",
        "node",
        &["left", "operator", "right"],
    )));
    assert!(
        receipt
            .syntax_shapes
            .contains(&("PropertyEntry", "node", &["key", "value"],))
    );
    assert!(receipt.syntax_shapes.contains(&(
        "NodePattern",
        "node",
        &["binding", "labels", "properties", "predicate"],
    )));
    assert!(receipt.syntax_shapes.contains(&(
        "EdgePattern",
        "node",
        &[
            "direction",
            "binding",
            "labels",
            "properties",
            "predicate",
            "quantifier",
        ],
    )));
    assert!(
        receipt
            .syntax_shapes
            .contains(&("MatchClause", "node", &["mode", "patterns"],))
    );
    assert!(
        receipt
            .syntax_shapes
            .contains(&("GraphPatternList", "node", &["pattern"],))
    );
    assert!(
        receipt
            .syntax_shapes
            .contains(&("PathMode", "node", &["kind"],))
    );
    assert!(
        receipt
            .syntax_shapes
            .contains(&("InlineWhereClause", "node", &["expression"],))
    );
    assert!(receipt.recoveries.contains(&(
        "expression-syntax",
        "GQL-PARSE-EXPRESSION-SYNTAX",
        "preserve-source",
    )));
    assert!(receipt.syntax_shapes.contains(&(
        "CharacterStringLiteralExpression",
        "node",
        &["value", "form", "no-escape"],
    )));
    assert!(receipt.recoveries.contains(&(
        "character-string-literal",
        "GQL-SYNTAX-INVALID-CHARACTER-STRING-LITERAL",
        "preserve-source",
    )));
    assert!(receipt.recoveries.contains(&(
        "match-pattern-list",
        "GQL-PARSE-MATCH-PATTERN-LIST",
        "preserve-source",
    )));
    assert!(receipt.recoveries.contains(&(
        "where-clause",
        "GQL-PARSE-WHERE-SYNTAX",
        "preserve-source",
    )));
    assert!(receipt.recoveries.contains(&(
        "inline-node-where",
        "GQL-PARSE-INLINE-WHERE-SYNTAX",
        "preserve-source",
    )));
    assert!(receipt.recoveries.contains(&(
        "inline-edge-where",
        "GQL-PARSE-INLINE-WHERE-SYNTAX",
        "preserve-source",
    )));
    assert!(receipt.recoveries.contains(&(
        "path-mode",
        "GQL-PARSE-PATH-MODE-SYNTAX",
        "preserve-source",
    )));
    assert!(receipt.recoveries.contains(&(
        "path-quantifier",
        "GQL-PARSE-PATH-QUANTIFIER",
        "preserve-source",
    )));
    assert_eq!(keyword("walk"), Some(Keyword::Walk));
    assert_eq!(keyword("TRAIL"), Some(Keyword::Trail));
    assert_eq!(keyword("acyclic"), Some(Keyword::Acyclic));
    assert_eq!(keyword("Simple"), Some(Keyword::Simple));
}

#[test]
fn gerbil_projection_drives_lossless_event_parser_dispatch() {
    let match_entry = top_level_parser_entrypoint(Keyword::Match)
        .expect("MATCH must enter the generated event parser");
    assert_eq!(match_entry.action, GrammarParserAction::MatchClause);

    let return_entry = top_level_parser_entrypoint(Keyword::Return)
        .expect("RETURN must enter the generated event parser");
    assert_eq!(return_entry.action, GrammarParserAction::ReturnClause);

    let create_schema = top_level_parser_entrypoint(Keyword::Create)
        .expect("CREATE must enter generated catalog dispatch");
    assert_eq!(
        create_schema.action,
        GrammarParserAction::CreateSchemaStatement
    );
    assert_eq!(top_level_parser_entrypoint(Keyword::Case), None);
}

#[test]
fn gerbil_projection_defines_expression_precedence() {
    let or = binary_operator_spec(TokenKind::Keyword(Keyword::Or), None)
        .expect("OR must be generated as a binary operator");
    let xor = binary_operator_spec(TokenKind::Keyword(Keyword::Xor), None)
        .expect("XOR must be generated as a binary operator");
    let and = binary_operator_spec(TokenKind::Keyword(Keyword::And), None)
        .expect("AND must be generated as a binary operator");
    let comparison = binary_operator_spec(TokenKind::Punctuation('='), None)
        .expect("comparison must be generated as a binary operator");
    let concatenation = binary_operator_spec(
        TokenKind::Punctuation('|'),
        Some(TokenKind::Punctuation('|')),
    )
    .expect("concatenation must be generated as one binary operator");
    let additive = binary_operator_spec(TokenKind::Punctuation('+'), None)
        .expect("addition must be generated as a binary operator");
    let multiplicative = binary_operator_spec(TokenKind::Punctuation('*'), None)
        .expect("multiplication must be generated as a binary operator");

    assert!(or.left_binding_power < xor.left_binding_power);
    assert!(xor.left_binding_power < and.left_binding_power);
    assert!(and.left_binding_power < comparison.left_binding_power);
    assert!(comparison.left_binding_power < concatenation.left_binding_power);
    assert!(concatenation.left_binding_power < additive.left_binding_power);
    assert!(additive.left_binding_power < multiplicative.left_binding_power);
    for specification in [or, xor, and, comparison, additive, multiplicative] {
        assert!(!specification.is_right_associative);
        assert_eq!(
            specification.right_binding_power,
            specification.left_binding_power + 1,
        );
    }
    assert_eq!(or.width, 1);
    assert_eq!(xor.width, 1);
    assert_eq!(and.width, 1);
    assert_eq!(concatenation.width, 2);
    assert!(
        prefix_operator_precedence(TokenKind::Punctuation('+')).expect("unary plus is generated")
            > multiplicative.left_binding_power
    );
    assert!(
        prefix_operator_precedence(TokenKind::Punctuation('-')).expect("unary minus is generated")
            > multiplicative.left_binding_power
    );
    assert_eq!(
        binary_operator_spec(
            TokenKind::Punctuation('<'),
            Some(TokenKind::Punctuation('>')),
        )
        .expect("<> must be one generated operator")
        .width,
        2,
    );
}

#[test]
fn parser_consumes_generated_precedence_for_lossless_node_shape() {
    let source = "MATCH (n) RETURN 1 + 2 * 3";
    let parsed = parse("precedence.gql", source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);

    let binary_texts = parsed
        .tree
        .rowan_root()
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::BinaryExpression)
        .map(|node| node.text().to_string())
        .collect::<Vec<_>>();
    assert_eq!(binary_texts, ["1 + 2 * 3", "2 * 3"]);
    assert_eq!(parsed.tree.rowan_root().text().to_string(), source);
}

#[test]
fn generated_dialect_boundary_recovers_with_exact_span() {
    let unsupported = parse("dialect.gql", "CREATE (n)");
    assert!(unsupported.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "GQL-PARSE-UNSUPPORTED-STATEMENT" && diagnostic.span == Span::new(0, 6)
    }));

    let recovery = parse("recovery.gql", "MATCH (n) RETURN n.");
    assert_eq!(
        recovery.tree.rowan_root().text().to_string(),
        "MATCH (n) RETURN n."
    );
    assert!(recovery.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "GQL-PARSE-EXPRESSION-SYNTAX" && diagnostic.span == Span::new(19, 19)
    }));

    let non_iso = parse("non-iso.gql", "MATCH (n) RETURN 1 != 2");
    assert_eq!(
        non_iso
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-PARSE-NON-ISO-OPERATOR"]
    );
}
