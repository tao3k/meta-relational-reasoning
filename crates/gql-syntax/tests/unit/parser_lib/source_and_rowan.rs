#[test]
fn preserves_source_and_ascii_identifiers() {
    let input = "MATCH (node)-[:CALLS]->(target) RETURN node";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert!(
        parsed.diagnostics.is_empty(),
        "diagnostics: {:?}",
        parsed.diagnostics
    );
    assert_eq!(parsed.tree.root().kind(), SyntaxKind::SourceFile);
    assert_eq!(parsed.tree.rowan_root().text().to_string(), input);
}

#[test]
fn generated_inputs_never_panic_or_lose_source_bytes() {
    const FRAGMENTS: &[&str] = &[
        "MATCH",
        "RETURN",
        "WHERE",
        "UNION",
        "ORDER",
        "BY",
        "LIMIT",
        "OFFSET",
        "(",
        ")",
        "[",
        "]",
        "{",
        "}",
        "-",
        "->",
        "<-",
        ":",
        ",",
        ".",
        "'",
        "\"",
        "# comment\n",
        " ",
        "\t",
        "\r\n",
        "\0",
        "@",
        "/",
        "identifier_1",
        "Z9",
        "_",
        "~",
        "123",
        "1.2.3",
    ];

    for seed in 0_u64..4_096 {
        let mut state = seed.wrapping_add(1);
        let mut input = String::new();
        for _ in 0..96 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            input.push_str(FRAGMENTS[(state as usize) % FRAGMENTS.len()]);
        }

        let parsed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            parse("generated-input.gql", &input)
        }))
        .unwrap_or_else(|_| panic!("parser panicked for generated seed {seed}"));
        let repeated = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            parse("generated-input.gql", &input)
        }))
        .unwrap_or_else(|_| panic!("parser panicked during repeat for generated seed {seed}"));

        assert_eq!(
            parsed.tree.rowan_root().text().to_string(),
            input,
            "parser lost source bytes for generated seed {seed}"
        );
        assert_eq!(
            parsed.diagnostics, repeated.diagnostics,
            "parser diagnostics are non-deterministic for generated seed {seed}"
        );
        for diagnostic in &parsed.diagnostics {
            let start = diagnostic.span.start as usize;
            let end = diagnostic.span.end as usize;
            assert!(
                start <= end && end <= input.len(),
                "diagnostic {} has out-of-bounds span {}..{} for generated seed {seed}",
                diagnostic.code,
                diagnostic.span.start,
                diagnostic.span.end
            );
            assert!(
                input.is_char_boundary(start) && input.is_char_boundary(end),
                "diagnostic {} splits a UTF-8 scalar at {}..{} for generated seed {seed}",
                diagnostic.code,
                diagnostic.span.start,
                diagnostic.span.end
            );
        }
        assert!(
            parsed.diagnostics.windows(2).all(|pair| {
                (pair[0].span.start, pair[0].span.end, pair[0].code)
                    <= (pair[1].span.start, pair[1].span.end, pair[1].code)
            }),
            "diagnostics are not in source order for generated seed {seed}: {:?}",
            parsed.diagnostics
        );
    }
}

#[test]
fn rowan_typed_view_exposes_graph_and_expression_structure() {
    let input = "MATCH (a)-[:CALLS]->(b) WHERE a = 1 RETURN b";
    let parsed = parse("test.gql", input);
    assert!(
        parsed.diagnostics.is_empty(),
        "diagnostics: {:?}",
        parsed.diagnostics
    );
    let root = parsed.tree.root();

    for kind in [
        SyntaxKind::Query,
        SyntaxKind::MatchClause,
        SyntaxKind::GraphPattern,
        SyntaxKind::NodePattern,
        SyntaxKind::EdgePattern,
        SyntaxKind::WhereClause,
        SyntaxKind::BinaryExpression,
        SyntaxKind::LiteralExpression,
    ] {
        assert!(
            contains_node_kind(&root, kind),
            "missing Rowan node {kind:?}"
        );
    }
}
