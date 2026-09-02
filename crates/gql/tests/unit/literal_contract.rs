use crate::Compiler;
use crate::ast::{CharacterStringForm, Expression as AstExpression, QueryClause, Statement};
use crate::catalog::{Catalog, CatalogName};
use crate::ir::{Expression as IrExpression, RecordField as IrRecordField};
use crate::syntax::TokenKind;

fn empty_catalog() -> Catalog {
    Catalog::new(
        CatalogName("literal-contract".into()),
        Vec::new(),
        Vec::new(),
    )
}

#[test]
fn doubled_quote_string_is_lossless_and_unescaped_through_canonical_ir() {
    let source = "MATCH (n) RETURN 'Ada''s graph'";
    let result = Compiler.compile("doubled-quote-string.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse diagnostics: {:?}",
        result.parse.diagnostics
    );
    let strings = result
        .parse
        .tree
        .tokens()
        .iter()
        .filter(|token| token.kind == TokenKind::String)
        .collect::<Vec<_>>();
    assert_eq!(strings.len(), 1);
    assert_eq!(strings[0].text(), "'Ada''s graph'");

    let Some(Statement::Query(query)) = &result.statement else {
        panic!("literal source must remain a query");
    };
    let Some(QueryClause::Return { projections, .. }) = query.clauses.get(1) else {
        panic!("RETURN clause exists");
    };
    let AstExpression::String(literal) = &projections[0].expression else {
        panic!("RETURN expression is a string literal");
    };
    assert_eq!(literal.value, "Ada's graph");
    assert_eq!(
        &source[literal.span.start as usize..literal.span.end as usize],
        "'Ada''s graph'"
    );

    assert!(
        result.analysis.diagnostics.is_empty(),
        "semantic diagnostics: {:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("canonical literal IR");
    assert_eq!(
        ir.projection[0].expression,
        IrExpression::String("Ada's graph".to_owned())
    );
}

#[test]
fn unterminated_doubled_quote_string_has_one_typed_terminal_and_no_ir() {
    let source = "MATCH (n) RETURN 'Ada''s";
    let result = Compiler.compile("unterminated-string.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(result.parse.diagnostics.len(), 1);
    let diagnostic = &result.parse.diagnostics[0];
    assert_eq!(diagnostic.code, "GQL-SYNTAX-UNTERMINATED-STRING");
    assert_eq!(
        &source[diagnostic.span.start as usize..diagnostic.span.end as usize],
        "'Ada''s"
    );
    assert!(result.analysis.ir.is_none());
}

#[test]
fn exact_numeric_family_is_one_lossless_token_family_and_canonical_ir() {
    let source = "RETURN 1_000, 12., .5, 6.02E23M, 7M, 8.5M, 9E+2M, 0xCA_FE, 0o7_5, 0b10_10";
    let result = Compiler.compile("exact-numeric-family.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse diagnostics: {:?}",
        result.parse.diagnostics
    );
    let numbers = result
        .parse
        .tree
        .tokens()
        .iter()
        .filter(|token| token.kind == TokenKind::Number)
        .map(|token| token.text())
        .collect::<Vec<_>>();
    assert_eq!(
        numbers,
        [
            "1_000", "12.", ".5", "6.02E23M", "7M", "8.5M", "9E+2M", "0xCA_FE", "0o7_5", "0b10_10",
        ]
    );

    let Some(Statement::Query(query)) = &result.statement else {
        panic!("exact numeric source must remain a query");
    };
    let Some(QueryClause::Return { projections, .. }) = query.clauses.first() else {
        panic!("RETURN clause exists");
    };
    assert_eq!(projections.len(), 10, "projections: {projections:?}");
    assert!(matches!(
        projections[0].expression,
        AstExpression::Integer(1_000, _)
    ));
    for (projection, expected) in projections[1..7]
        .iter()
        .zip(["12.", ".5", "6.02E23", "7", "8.5", "9E+2"])
    {
        assert!(
            matches!(&projection.expression, AstExpression::Decimal(value, _) if value == expected),
            "unexpected exact decimal projection: {:?}",
            projection.expression
        );
    }
    for (projection, expected) in projections[7..].iter().zip([0xCA_FE, 0o7_5, 0b10_10]) {
        assert!(
            matches!(projection.expression, AstExpression::Integer(value, _) if value == expected)
        );
    }

    let ir = result.analysis.ir.expect("canonical exact numeric IR");
    assert_eq!(
        ir.projection
            .iter()
            .map(|projection| projection.expression.clone())
            .collect::<Vec<_>>(),
        vec![
            IrExpression::Integer(1_000),
            IrExpression::Decimal("12.".into()),
            IrExpression::Decimal(".5".into()),
            IrExpression::Decimal("6.02E23".into()),
            IrExpression::Decimal("7".into()),
            IrExpression::Decimal("8.5".into()),
            IrExpression::Decimal("9E+2".into()),
            IrExpression::Integer(0xCA_FE),
            IrExpression::Integer(0o7_5),
            IrExpression::Integer(0b10_10),
        ]
    );
}

#[test]
fn malformed_exact_numeric_forms_have_one_typed_terminal_and_no_ir() {
    for source in [
        "RETURN 1__0",
        "RETURN 1E+",
        "RETURN 1.2MM",
        "RETURN 0XFF",
        "RETURN 0xFF_",
        "RETURN 0x__FF",
        "RETURN 1_e2",
        "RETURN 1e_2",
    ] {
        let result = Compiler.compile("invalid-exact-numeric.gql", source, &empty_catalog());

        assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
        assert_eq!(
            result
                .parse
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect::<Vec<_>>(),
            ["GQL-SYNTAX-INVALID-NUMERIC-LITERAL"],
            "unexpected diagnostics for {source}: {:?}",
            result.parse.diagnostics
        );
        assert!(
            result.analysis.ir.is_none(),
            "invalid source admitted: {source}"
        );
    }
}

#[test]
fn numeric_case_and_radix_separator_rules_are_canonical_and_lossless() {
    let source = "RETURN 1.2e3m, 3.4e-2f, 0x_FF";
    let result = Compiler.compile("numeric-case-and-separators.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(result.parse.diagnostics.is_empty());
    let ir = result.analysis.ir.expect("canonical numeric IR");
    assert_eq!(
        ir.projection
            .iter()
            .map(|projection| projection.expression.clone())
            .collect::<Vec<_>>(),
        [
            IrExpression::Decimal("1.2E3".into()),
            IrExpression::ApproximateNumeric("3.4E-2F".into()),
            IrExpression::Integer(255),
        ]
    );
}

#[test]
fn approximate_numeric_family_is_lossless_and_reaches_canonical_ir() {
    let source = "RETURN 1F, 2D, .5F, 6E2, 6E2D";
    let result = Compiler.compile("approximate-numeric-family.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse diagnostics: {:?}",
        result.parse.diagnostics
    );
    assert_eq!(
        result
            .parse
            .tree
            .tokens()
            .iter()
            .filter(|token| token.kind == TokenKind::Number)
            .map(|token| token.text())
            .collect::<Vec<_>>(),
        ["1F", "2D", ".5F", "6E2", "6E2D"]
    );

    let Some(Statement::Query(query)) = &result.statement else {
        panic!("approximate numeric source must remain a query");
    };
    let Some(QueryClause::Return { projections, .. }) = query.clauses.first() else {
        panic!("RETURN clause exists");
    };
    for (projection, expected) in projections.iter().zip(["1F", "2D", ".5F", "6E2", "6E2D"]) {
        assert!(
            matches!(&projection.expression, AstExpression::ApproximateNumeric(value, _) if value == expected),
            "unexpected approximate projection: {:?}",
            projection.expression
        );
    }
    let ir = result
        .analysis
        .ir
        .expect("canonical approximate numeric IR");
    assert!(
        ir.projection
            .iter()
            .all(|projection| projection.value_type == crate::types::ValueType::Float)
    );
    assert_eq!(
        ir.projection
            .iter()
            .map(|projection| projection.expression.clone())
            .collect::<Vec<_>>(),
        ["1F", "2D", ".5F", "6E2", "6E2D"]
            .map(|value| IrExpression::ApproximateNumeric(value.into()))
    );
}

#[test]
fn approximate_arithmetic_dominates_exact_numeric_type_without_backend_coercion() {
    let source = "RETURN 1F + 2M";
    let result = Compiler.compile("mixed-numeric-arithmetic.gql", source, &empty_catalog());

    assert!(result.parse.diagnostics.is_empty());
    assert!(result.analysis.diagnostics.is_empty());
    let ir = result.analysis.ir.expect("mixed numeric canonical IR");
    assert_eq!(ir.projection[0].value_type, crate::types::ValueType::Float);
    assert!(matches!(
        &ir.projection[0].expression,
        IrExpression::Binary { left, right, .. }
            if matches!(left.as_ref(), IrExpression::ApproximateNumeric(value) if value == "1F")
                && matches!(right.as_ref(), IrExpression::Decimal(value) if value == "2")
    ));
}

#[test]
fn contextual_double_quoted_string_and_delimited_identifier_are_distinct() {
    let source = "RETURN \"Ada\"\"s\" AS \"Display\"";
    let result = Compiler.compile(
        "contextual-double-quoted-string.gql",
        source,
        &empty_catalog(),
    );

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse diagnostics: {:?}",
        result.parse.diagnostics
    );
    assert!(
        result.analysis.diagnostics.is_empty(),
        "semantic diagnostics: {:?}",
        result.analysis.diagnostics
    );
    assert!(result.statement.is_some(), "source-level AST must exist");
    assert!(result.analysis.ir.is_some(), "canonical IR must exist");
}

#[test]
fn general_literal_families_are_lossless_and_canonical() {
    let source = concat!(
        "RETURN X'CA FE', DATE '2026-09-02', TIME '12:34:56.789Z', ",
        "TIME '12:34:56+08:00', TIMESTAMP '2026-09-02T12:34:56', ",
        "DATETIME '2026-09-02T12:34:56Z', DURATION 'P1DT2H', ",
        "[1, 'Ada', [2]], RECORD {name: 'Ada', age: 42}, {status: 'active'}"
    );
    let result = Compiler.compile("general-literal-families.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse diagnostics: {:?}",
        result.parse.diagnostics
    );
    let Some(Statement::Query(query)) = &result.statement else {
        panic!("general literals must lower to a query AST");
    };
    let Some(QueryClause::Return { projections, .. }) = query.clauses.first() else {
        panic!("RETURN clause exists");
    };
    assert_eq!(projections.len(), 10, "projections: {projections:?}");
    assert!(matches!(
        &projections[0].expression,
        AstExpression::ByteString(value, span)
            if value == &[0xCA, 0xFE]
                && &source[span.start as usize..span.end as usize] == "X'CA FE'"
    ));
    assert!(matches!(
        &projections[1].expression,
        AstExpression::Date(value, span)
            if value == "2026-09-02"
                && &source[span.start as usize..span.end as usize] == "DATE '2026-09-02'"
    ));
    assert!(matches!(
        &projections[2].expression,
        AstExpression::Time(value, _) if value == "12:34:56.789Z"
    ));
    assert!(matches!(
        &projections[3].expression,
        AstExpression::Time(value, _) if value == "12:34:56+08:00"
    ));
    assert!(matches!(
        &projections[4].expression,
        AstExpression::Timestamp(value, _) if value == "2026-09-02T12:34:56"
    ));
    assert!(matches!(
        &projections[5].expression,
        AstExpression::Timestamp(value, _) if value == "2026-09-02T12:34:56Z"
    ));
    assert!(matches!(
        &projections[6].expression,
        AstExpression::Duration(value, _) if value == "P1DT2H"
    ));
    assert!(matches!(
        &projections[7].expression,
        AstExpression::List(values, _) if matches!(
            values.as_slice(),
            [
                AstExpression::Integer(1, _),
                AstExpression::String(literal),
                AstExpression::List(nested, _),
            ] if literal.value == "Ada"
                && matches!(nested.as_slice(), [AstExpression::Integer(2, _)])
        )
    ));
    let AstExpression::Record(fields, record_span) = &projections[8].expression else {
        panic!("ninth projection is a prefixed record literal");
    };
    assert_eq!(
        &source[record_span.start as usize..record_span.end as usize],
        "RECORD {name: 'Ada', age: 42}"
    );
    assert_eq!(
        fields
            .iter()
            .map(|field| field.name.text.as_str())
            .collect::<Vec<_>>(),
        ["name", "age"]
    );
    assert!(
        matches!(fields[0].value, AstExpression::String(ref literal) if literal.value == "Ada")
    );
    assert!(matches!(fields[1].value, AstExpression::Integer(42, _)));
    let AstExpression::Record(bare_fields, bare_span) = &projections[9].expression else {
        panic!("tenth projection is a bare record literal");
    };
    assert_eq!(
        &source[bare_span.start as usize..bare_span.end as usize],
        "{status: 'active'}"
    );
    assert_eq!(bare_fields.len(), 1);
    assert_eq!(bare_fields[0].name.text, "status");
    assert!(matches!(
        bare_fields[0].value,
        AstExpression::String(ref literal) if literal.value == "active"
    ));
    assert!(
        result.analysis.diagnostics.is_empty(),
        "semantic diagnostics: {:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("canonical general-literal IR");
    assert_eq!(
        ir.projection
            .iter()
            .map(|projection| projection.expression.clone())
            .collect::<Vec<_>>(),
        vec![
            IrExpression::ByteString(vec![0xCA, 0xFE]),
            IrExpression::Date("2026-09-02".into()),
            IrExpression::Time("12:34:56.789Z".into()),
            IrExpression::Time("12:34:56+08:00".into()),
            IrExpression::Timestamp("2026-09-02T12:34:56".into()),
            IrExpression::Timestamp("2026-09-02T12:34:56Z".into()),
            IrExpression::Duration("P1DT2H".into()),
            IrExpression::List(vec![
                IrExpression::Integer(1),
                IrExpression::String("Ada".into()),
                IrExpression::List(vec![IrExpression::Integer(2)]),
            ]),
            IrExpression::Record(vec![
                IrRecordField {
                    name: "NAME".into(),
                    value: IrExpression::String("Ada".into()),
                },
                IrRecordField {
                    name: "AGE".into(),
                    value: IrExpression::Integer(42),
                },
            ]),
            IrExpression::Record(vec![IrRecordField {
                name: "STATUS".into(),
                value: IrExpression::String("active".into()),
            }]),
        ]
    );
}

#[test]
fn character_string_escape_sequences_cross_frontend_admission() {
    let source = r#"RETURN 'A\nB', "A\nB", @'A\nB', '\t\b\r\f\\\'\"\u0041\U01F642', DATE "2026-09-02", DURATION "P1DT2H""#;
    let result = Compiler.compile("character-string-escapes.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse diagnostics: {:?}",
        result.parse.diagnostics
    );
    assert!(
        result.analysis.diagnostics.is_empty(),
        "semantic diagnostics: {:?}",
        result.analysis.diagnostics
    );
    let Some(Statement::Query(query)) = &result.statement else {
        panic!("escaped character strings must lower to a query AST");
    };
    let Some(QueryClause::Return { projections, .. }) = query.clauses.first() else {
        panic!("RETURN clause exists");
    };
    assert_eq!(projections.len(), 6, "projections: {projections:?}");
    assert!(matches!(
        &projections[0].expression,
        AstExpression::String(literal)
            if literal.form == CharacterStringForm::SingleQuoted
                && !literal.no_escape
                && &source[literal.span.start as usize..literal.span.end as usize] == r"'A\nB'"
    ));
    assert!(matches!(
        &projections[1].expression,
        AstExpression::String(literal)
            if literal.form == CharacterStringForm::DoubleQuoted && !literal.no_escape
    ));
    assert!(matches!(
        &projections[2].expression,
        AstExpression::String(literal)
            if literal.form == CharacterStringForm::SingleQuoted && literal.no_escape
    ));
    assert!(matches!(
        &projections[0].expression,
        AstExpression::String(literal) if literal.value == "A\nB"
    ));
    assert!(matches!(
        &projections[1].expression,
        AstExpression::String(literal) if literal.value == "A\nB"
    ));
    assert!(matches!(
        &projections[2].expression,
        AstExpression::String(literal) if literal.value == r"A\nB"
    ));
    assert!(matches!(
        &projections[3].expression,
        AstExpression::String(literal)
            if literal.value == "\t\u{0008}\r\u{000C}\\'\"A\u{1f642}"
    ));
    assert!(matches!(
        &projections[4].expression,
        AstExpression::Date(value, _) if value == "2026-09-02"
    ));
    assert!(matches!(
        &projections[5].expression,
        AstExpression::Duration(value, _) if value == "P1DT2H"
    ));
    let ir = result.analysis.ir.expect("escaped character-string IR");
    assert_eq!(ir.projection.len(), 6);
    assert_eq!(
        ir.projection[0].expression,
        IrExpression::String("A\nB".into())
    );
    assert_eq!(ir.projection[0].expression, ir.projection[1].expression);
    assert_eq!(
        ir.projection[2].expression,
        IrExpression::String(r"A\nB".into())
    );
}

#[test]
fn malformed_character_string_escapes_emit_one_typed_terminal_and_no_ir() {
    for (source, expected_code) in [
        (
            r"RETURN '\q'",
            "GQL-SYNTAX-INVALID-CHARACTER-STRING-LITERAL",
        ),
        (
            r"RETURN '\u12'",
            "GQL-SYNTAX-INVALID-CHARACTER-STRING-LITERAL",
        ),
        (
            r"RETURN '\u12G4'",
            "GQL-SYNTAX-INVALID-CHARACTER-STRING-LITERAL",
        ),
        (
            r"RETURN '\U110000'",
            "GQL-SYNTAX-INVALID-CHARACTER-STRING-LITERAL",
        ),
        (
            "RETURN 'line\nbreak'",
            "GQL-SYNTAX-INVALID-CHARACTER-STRING-LITERAL",
        ),
        ("RETURN 'unterminated", "GQL-SYNTAX-UNTERMINATED-STRING"),
    ] {
        let result = Compiler.compile("invalid-character-string.gql", source, &empty_catalog());
        assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
        assert_eq!(
            result
                .parse
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect::<Vec<_>>(),
            [expected_code],
            "unexpected diagnostics for {source:?}"
        );
        assert!(
            result.statement.is_none(),
            "invalid AST admitted: {source:?}"
        );
        assert!(
            result.analysis.ir.is_none(),
            "invalid IR admitted: {source:?}"
        );
    }
}

#[test]
fn malformed_general_literals_emit_exact_terminal_and_no_ir() {
    for (source, expected_code) in [
        ("RETURN X'ABC'", "GQL-SYNTAX-INVALID-BYTE-STRING"),
        (
            "RETURN DATE '2026-13-40'",
            "GQL-SYNTAX-INVALID-TEMPORAL-LITERAL",
        ),
        (
            "RETURN DURATION 'tomorrow'",
            "GQL-SYNTAX-INVALID-DURATION-LITERAL",
        ),
        ("RETURN [1,]", "GQL-PARSE-LIST-SYNTAX"),
        ("RETURN RECORD {name 'Ada'}", "GQL-PARSE-RECORD-SYNTAX"),
        ("RETURN RECORD {name: 'Ada',}", "GQL-PARSE-RECORD-SYNTAX"),
    ] {
        let result = Compiler.compile("invalid-general-literal.gql", source, &empty_catalog());

        assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
        let codes = result
            .parse
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>();
        assert_eq!(
            codes,
            [expected_code],
            "unexpected diagnostics for {source}"
        );
        assert!(
            result.analysis.ir.is_none(),
            "invalid source admitted: {source}"
        );
    }
}

#[test]
fn duplicate_record_fields_are_rejected_without_ir() {
    let source = "RETURN RECORD {name: 'Ada', name: 'Grace'}";
    let result = Compiler.compile("duplicate-record-field.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(result.parse.diagnostics.is_empty());
    assert_eq!(
        result
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-DUPLICATE-RECORD-FIELD"]
    );
    assert!(result.analysis.ir.is_none());
}
