use crate::{
    BinaryOperator, EdgeDirection, Expression, PatternElement, QueryClause, Statement,
    UnaryOperator, lower_from_syntax,
};
use gql_syntax::parse;

#[test]
fn lower_where_clause_with_equality_expression() {
    let parsed = parse("q", "MATCH (a)-[:CALLS]->(b) WHERE a = 1 RETURN b");
    let lowered = lower_from_syntax(&parsed);
    let query = match lowered.statement {
        Some(Statement::Query(query)) => query,
        _ => panic!("statement is query"),
    };
    let clause = query
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::Where { .. }))
        .expect("where clause exists");

    let QueryClause::Where { expression, .. } = clause else {
        unreachable!()
    };

    if let Expression::Binary {
        operator: BinaryOperator::Equals,
        left,
        right,
    } = expression
    {
        assert!(matches!(**left, Expression::Name(_)));
        assert!(matches!(**right, Expression::Integer(_, _)));
    } else {
        panic!("unexpected expression: {expression:?}");
    }
}

#[test]
fn lower_return_projection_alias() {
    let parsed = parse("q", "MATCH (n) RETURN n AS person");
    let lowered = lower_from_syntax(&parsed);
    let Some(Statement::Query(query)) = lowered.statement else {
        panic!("statement is query");
    };
    let Some(QueryClause::Return { projections, .. }) = query.clauses.last() else {
        panic!("aliased return clause exists");
    };
    assert!(matches!(projections[0].expression, Expression::Name(_)));
    assert_eq!(
        projections[0]
            .alias
            .as_ref()
            .map(|alias| alias.text.as_str()),
        Some("person")
    );
}

#[test]
fn lower_reserved_keyword_expression_does_not_create_a_name_projection() {
    let parsed = parse("q", "MATCH (n) RETURN CALL");
    let lowered = lower_from_syntax(&parsed);

    assert!(
        lowered
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "GQL-PARSE-UNSUPPORTED-KEYWORD-EXPRESSION" })
    );

    assert!(lowered.statement.is_none());
}

#[test]
fn generated_inputs_never_panic_during_typed_ast_lowering() {
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

    for seed in 0_u64..1_024 {
        let mut state = seed.wrapping_add(1);
        let mut input = String::new();
        for _ in 0..48 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            input.push_str(FRAGMENTS[(state as usize) % FRAGMENTS.len()]);
        }

        let lower = || {
            let parsed = parse("generated-input.gql", &input);
            lower_from_syntax(&parsed)
        };
        let lowered = std::panic::catch_unwind(std::panic::AssertUnwindSafe(lower))
            .unwrap_or_else(|_| panic!("AST lowering panicked for generated seed {seed}"));
        let repeated = std::panic::catch_unwind(std::panic::AssertUnwindSafe(lower))
            .unwrap_or_else(|_| panic!("AST lowering repeat panicked for generated seed {seed}"));

        assert_eq!(
            lowered.diagnostics, repeated.diagnostics,
            "AST diagnostics are non-deterministic for generated seed {seed}"
        );
        for diagnostic in &lowered.diagnostics {
            let start = diagnostic.span.start as usize;
            let end = diagnostic.span.end as usize;
            assert!(
                start <= end && end <= input.len(),
                "AST diagnostic {} has out-of-bounds span {}..{} for generated seed {seed}",
                diagnostic.code,
                diagnostic.span.start,
                diagnostic.span.end
            );
            assert!(
                input.is_char_boundary(start) && input.is_char_boundary(end),
                "AST diagnostic {} splits UTF-8 at {}..{} for generated seed {seed}",
                diagnostic.code,
                diagnostic.span.start,
                diagnostic.span.end
            );
        }
    }
}

#[test]
fn lower_union_query_boundary() {
    let parsed = parse("q", "MATCH (a) RETURN a UNION MATCH (b) RETURN b");
    let lowered = lower_from_syntax(&parsed);
    let Some(Statement::Query(query)) = lowered.statement else {
        panic!("statement is query");
    };
    assert_eq!(
        query
            .clauses
            .iter()
            .filter(|clause| matches!(clause, QueryClause::Union { .. }))
            .count(),
        1
    );
}

#[test]
fn lower_limit_clause() {
    let parsed = parse("q", "MATCH (n) RETURN n LIMIT 10");
    let lowered = lower_from_syntax(&parsed);
    let Some(Statement::Query(query)) = lowered.statement else {
        panic!("statement is query");
    };
    assert!(matches!(
        query.clauses.last(),
        Some(QueryClause::Limit {
            value: crate::NonNegativeIntegerSpecification::Literal(10),
            ..
        })
    ));
}

#[test]
fn lower_order_by_clause_with_directions() {
    let parsed = parse("q", "MATCH (n) RETURN n ORDER BY n DESC, n ASC");
    let lowered = lower_from_syntax(&parsed);
    let Some(Statement::Query(query)) = lowered.statement else {
        panic!("statement is query");
    };
    let Some(QueryClause::OrderBy { keys, .. }) = query.clauses.last() else {
        panic!("ORDER BY clause exists");
    };
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0].direction, Some(crate::SortDirection::Descending));
    assert_eq!(keys[1].direction, Some(crate::SortDirection::Ascending));
}

#[test]
fn lower_offset_clause() {
    let parsed = parse("q", "MATCH (n) RETURN n LIMIT 10 OFFSET 2");
    let lowered = lower_from_syntax(&parsed);
    let Some(Statement::Query(query)) = lowered.statement else {
        panic!("statement is query");
    };
    assert!(matches!(
        query.clauses.last(),
        Some(QueryClause::Offset {
            value: crate::NonNegativeIntegerSpecification::Literal(2),
            ..
        })
    ));
}

#[test]
fn lower_match_clause_with_incoming_edge_direction() {
    let parsed = parse("q", "MATCH (a)<-[:CALLS]-(b) RETURN a");
    let lowered = lower_from_syntax(&parsed);
    let query = match lowered.statement {
        Some(Statement::Query(query)) => query,
        _ => panic!("statement is query"),
    };
    let clause = query
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::Match(_)))
        .expect("match clause exists");

    let QueryClause::Match(match_clause) = clause else {
        unreachable!()
    };
    let edge = match_clause.patterns[0]
        .elements
        .iter()
        .find_map(|element| match element {
            PatternElement::Edge(edge) => Some(edge),
            _ => None,
        })
        .expect("edge pattern exists");

    assert_eq!(edge.direction, EdgeDirection::In);
    assert_eq!(edge.labels.first().expect("has relation").text, "CALLS");
}

#[test]
fn lower_where_clause_with_not_parentheses_preserves_structure() {
    let parsed = parse("q", "MATCH (a)-[:CALLS]->(b) WHERE NOT (a = 1) RETURN b");
    let lowered = lower_from_syntax(&parsed);
    let query = match lowered.statement {
        Some(Statement::Query(query)) => query,
        _ => panic!("statement is query"),
    };
    let clause = query
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::Where { .. }))
        .expect("where clause exists");

    let QueryClause::Where { expression, .. } = clause else {
        unreachable!()
    };

    match expression {
        Expression::Unary {
            operator: UnaryOperator::Not,
            operand,
        } => match **operand {
            Expression::Binary {
                operator: BinaryOperator::Equals,
                ..
            } => {}
            _ => panic!("unexpected operand: {operand:?}"),
        },
        _ => panic!("unexpected expression: {expression:?}"),
    }
}

#[test]
fn lower_match_clause_with_outgoing_edge_direction() {
    let parsed = parse("q", "MATCH (a)-[:CALLS]->(b) RETURN a");
    let lowered = lower_from_syntax(&parsed);
    let query = match lowered.statement {
        Some(Statement::Query(query)) => query,
        _ => panic!("statement is query"),
    };
    let clause = query
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::Match(_)))
        .expect("match clause exists");

    let QueryClause::Match(match_clause) = clause else {
        unreachable!()
    };
    let edge = match_clause.patterns[0]
        .elements
        .iter()
        .find_map(|element| match element {
            PatternElement::Edge(edge) => Some(edge),
            _ => None,
        })
        .expect("edge pattern exists");

    assert_eq!(edge.direction, EdgeDirection::Out);
}

#[test]
fn lower_match_clause_with_labeled_edge_direction_ignores_label_list_punctuation() {
    let parsed = parse("q", "MATCH (a)-[:CALLS:Person]->(b) RETURN a");
    let lowered = lower_from_syntax(&parsed);
    let query = match lowered.statement {
        Some(Statement::Query(query)) => query,
        _ => panic!("statement is query"),
    };
    let clause = query
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::Match(_)))
        .expect("match clause exists");

    let QueryClause::Match(match_clause) = clause else {
        unreachable!()
    };
    let edge = match_clause.patterns[0]
        .elements
        .iter()
        .find_map(|element| match element {
            PatternElement::Edge(edge) => Some(edge),
            _ => None,
        })
        .expect("edge pattern exists");

    assert_eq!(edge.direction, EdgeDirection::Out);
    assert_eq!(edge.labels[0].text, "CALLS");
    assert_eq!(edge.labels[1].text, "Person");
}

#[test]
fn lower_match_clause_with_incoming_labeled_edge_direction_ignores_label_list_punctuation() {
    let parsed = parse("q", "MATCH (a)<-[:CALLS:Person]-(b) RETURN a");
    let lowered = lower_from_syntax(&parsed);
    let query = match lowered.statement {
        Some(Statement::Query(query)) => query,
        _ => panic!("statement is query"),
    };
    let clause = query
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::Match(_)))
        .expect("match clause exists");

    let QueryClause::Match(match_clause) = clause else {
        unreachable!()
    };
    let edge = match_clause.patterns[0]
        .elements
        .iter()
        .find_map(|element| match element {
            PatternElement::Edge(edge) => Some(edge),
            _ => None,
        })
        .expect("edge pattern exists");

    assert_eq!(edge.direction, EdgeDirection::In);
    assert_eq!(edge.labels[0].text, "CALLS");
    assert_eq!(edge.labels[1].text, "Person");
}

#[test]
fn lower_where_clause_with_string_and_integer_expression() {
    let parsed = parse("q", "MATCH (a)-[:CALLS]->(b) WHERE 'hello' RETURN b");
    let lowered = lower_from_syntax(&parsed);
    let query = match lowered.statement {
        Some(Statement::Query(query)) => query,
        _ => panic!("statement is query"),
    };
    let clause = query.clauses.get(1).expect("query has where clause");

    let QueryClause::Where { expression, .. } = clause else {
        panic!("expected where clause");
    };

    match expression {
        Expression::String(literal) => assert_eq!(literal.value, "hello"),
        other => panic!("unexpected expression: {other:?}"),
    }

    let parsed = parse("q", "LET a = 42");
    let lowered = lower_from_syntax(&parsed);

    let query = match lowered.statement {
        Some(Statement::Query(query)) => query,
        _ => panic!("statement is query"),
    };
    let clause = query.clauses.first().expect("query has let clause");

    let QueryClause::Let { bindings, .. } = clause else {
        panic!("expected let clause");
    };

    match &bindings[0].value {
        Expression::Integer(value, _) => assert_eq!(*value, 42),
        other => panic!("unexpected expression: {other:?}"),
    }
}

#[test]
fn lower_where_clause_with_missing_expression_emits_diagnostic() {
    let parsed = parse("q", "MATCH (a)-[:CALLS]->(b) WHERE RETURN a");
    let lowered = lower_from_syntax(&parsed);

    assert_eq!(
        lowered
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-PARSE-WHERE-SYNTAX"]
    );
    assert!(lowered.statement.is_none());
}

#[test]
fn lower_let_clause_missing_binding_and_value_emit_diagnostics() {
    let parsed = parse("q", "LET");
    let lowered = lower_from_syntax(&parsed);

    assert_eq!(
        lowered
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-AST-LET-BINDING-MISSING"]
    );
    assert!(lowered.statement.is_none());
}

#[test]
fn lower_let_clause_with_binding_as_non_identifier_is_diagnostic() {
    let parsed = parse("q", "LET 1 = 2");
    let lowered = lower_from_syntax(&parsed);

    assert!(
        lowered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-AST-LET-BINDING-EXPECTED")
    );
    assert!(lowered.statement.is_none());
}

#[test]
fn lower_let_clause_with_missing_binding_but_numeric_value() {
    let parsed = parse("q", "LET = 1");
    let lowered = lower_from_syntax(&parsed);

    assert_eq!(
        lowered
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-AST-LET-BINDING-EXPECTED"]
    );
    assert!(lowered.statement.is_none());
}

#[test]
fn lower_let_clause_with_missing_value_is_diagnostic() {
    let parsed = parse("q", "LET a");
    let lowered = lower_from_syntax(&parsed);

    assert!(
        lowered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-AST-LET-VALUE-MISSING")
    );
    assert!(lowered.statement.is_none());
}

#[test]
fn lower_property_access_and_scalar_literals() {
    let parsed = parse("q", "MATCH (n) WHERE n.name = TRUE RETURN n.name");
    let lowered = lower_from_syntax(&parsed);
    assert!(
        lowered.diagnostics.is_empty(),
        "diagnostics: {:?}",
        lowered.diagnostics
    );
    let Some(Statement::Query(query)) = lowered.statement else {
        panic!("expected query");
    };
    let QueryClause::Where { expression, .. } = query
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::Where { .. }))
        .expect("where clause")
    else {
        unreachable!()
    };

    let Expression::Binary { left, right, .. } = expression else {
        panic!("expected comparison");
    };
    assert!(matches!(**left, Expression::PropertyAccess { .. }));
    assert!(matches!(**right, Expression::Boolean(true, _)));
}

#[test]
fn lower_named_path_pattern_preserves_binding_and_nested_elements() {
    let parsed = parse("q", "MATCH p = (a)-[:CALLS]->(b) RETURN p");
    let lowered = lower_from_syntax(&parsed);
    let Some(Statement::Query(query)) = lowered.statement else {
        panic!("statement is query");
    };
    let Some(QueryClause::Match(match_clause)) = query
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::Match(_)))
    else {
        panic!("match clause exists");
    };

    let path = &match_clause.patterns[0];
    assert_eq!(
        path.binding.as_ref().map(|binding| binding.text.as_str()),
        Some("p")
    );
    assert_eq!(path.elements.len(), 3);
}

#[test]
fn lower_bounded_path_quantifier_on_edge_pattern() {
    let parsed = parse("q", "MATCH (a)-[:CALLS]->{1,3}(b) RETURN b");
    let lowered = lower_from_syntax(&parsed);
    let Some(Statement::Query(query)) = lowered.statement else {
        panic!("statement is query");
    };
    let Some(QueryClause::Match(match_clause)) = query
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::Match(_)))
    else {
        panic!("match clause exists");
    };
    let Some(PatternElement::Edge(edge)) = match_clause.patterns[0]
        .elements
        .iter()
        .find(|element| matches!(element, PatternElement::Edge(_)))
    else {
        panic!("edge pattern exists");
    };

    assert_eq!(
        edge.quantifier
            .as_ref()
            .map(|quantifier| (quantifier.min, quantifier.max)),
        Some((1, Some(3)))
    );
}

#[test]
fn lower_optional_match_clause_preserves_optional_graph() {
    let parsed = parse(
        "q",
        "MATCH (a) OPTIONAL MATCH (a)-[:CALLS]->(b) RETURN a, b",
    );
    let lowered = lower_from_syntax(&parsed);
    let Some(Statement::Query(query)) = lowered.statement else {
        panic!("statement is query");
    };

    assert!(query.clauses.iter().any(|clause| {
        matches!(clause, QueryClause::OptionalMatch(match_clause)
            if match_clause.patterns[0].elements.len() == 3)
    }));
}

#[test]
fn lower_arithmetic_expression_preserves_precedence() {
    let parsed = parse("q", "MATCH (n) RETURN 1 + 2 * 3");
    let lowered = lower_from_syntax(&parsed);
    let Some(Statement::Query(query)) = lowered.statement else {
        panic!("statement is query");
    };
    let Some(QueryClause::Return { projections, .. }) = query
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::Return { .. }))
    else {
        panic!("return clause exists");
    };
    let Expression::Binary {
        operator: BinaryOperator::Add,
        left,
        right,
    } = &projections[0].expression
    else {
        panic!(
            "expected addition expression: {:?}",
            projections[0].expression
        );
    };
    assert!(matches!(left.as_ref(), Expression::Integer(1, _)));
    assert!(matches!(
        right.as_ref(),
        Expression::Binary {
            operator: BinaryOperator::Multiply,
            ..
        }
    ));
}

#[test]
fn lower_division_and_modulo_preserve_left_associativity() {
    let parsed = parse("q", "MATCH (n) RETURN 8 / 2 % 3");
    let lowered = lower_from_syntax(&parsed);
    let Some(Statement::Query(query)) = lowered.statement else {
        panic!("statement is query");
    };
    let Some(QueryClause::Return { projections, .. }) = query
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::Return { .. }))
    else {
        panic!("return clause exists");
    };
    assert!(matches!(
        projections.first().map(|projection| &projection.expression),
        Some(Expression::Binary {
            operator: BinaryOperator::Modulo,
            left,
            ..
        }) if matches!(left.as_ref(), Expression::Binary {
            operator: BinaryOperator::Divide,
            ..
        })
    ));
}

#[test]
fn lower_list_value_preserves_nested_items() {
    let parsed = parse("q", "MATCH (n) RETURN [1, 2, [3]]");
    let lowered = lower_from_syntax(&parsed);
    let Some(Statement::Query(query)) = lowered.statement else {
        panic!("statement is query");
    };
    let Some(QueryClause::Return { projections, .. }) = query
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::Return { .. }))
    else {
        panic!("return clause exists");
    };
    let Expression::List(items, _) = &projections[0].expression else {
        panic!("expected list expression: {:?}", projections[0].expression);
    };
    assert_eq!(items.len(), 3);
    assert!(matches!(
        &items[2],
        Expression::List(nested, _) if nested.len() == 1
    ));
}

#[test]
fn lower_collection_subscript_preserves_base_and_index() {
    let parsed = parse("q", "MATCH (n) RETURN [1, 2][0]");
    let lowered = lower_from_syntax(&parsed);
    let Some(Statement::Query(query)) = lowered.statement else {
        panic!("statement is query");
    };
    let Some(QueryClause::Return { projections, .. }) = query
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::Return { .. }))
    else {
        panic!("return clause exists");
    };
    assert!(matches!(
        projections.first().map(|projection| &projection.expression),
        Some(Expression::Subscript { base, index })
            if matches!(base.as_ref(), Expression::List(items, _) if items.len() == 2)
                && matches!(index.as_ref(), Expression::Integer(0, _))
    ));
}

#[test]
fn lower_in_collection_predicate_preserves_operator() {
    let parsed = parse("q", "MATCH (n) RETURN 1 IN [1, 2]");
    let lowered = lower_from_syntax(&parsed);
    let Some(Statement::Query(query)) = lowered.statement else {
        panic!("statement is query");
    };
    let Some(QueryClause::Return { projections, .. }) = query
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::Return { .. }))
    else {
        panic!("return clause exists");
    };
    assert!(matches!(
        projections.first().map(|projection| &projection.expression),
        Some(Expression::Binary {
            operator: BinaryOperator::In,
            right,
            ..
        }) if matches!(right.as_ref(), Expression::List(items, _) if items.len() == 2)
    ));
}
