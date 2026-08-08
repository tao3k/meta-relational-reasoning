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
        Statement::Query(query) => query,
        _ => panic!("statement is query"),
    };
    let clause = query
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::Where { .. }))
        .expect("where clause exists");

    let QueryClause::Where { expression } = clause else {
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
fn lower_match_clause_with_incoming_edge_direction() {
    let parsed = parse("q", "MATCH (a)<-[:CALLS]-(b) RETURN a");
    let lowered = lower_from_syntax(&parsed);
    let query = match lowered.statement {
        Statement::Query(query) => query,
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
    let edge = match_clause
        .pattern
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
        Statement::Query(query) => query,
        _ => panic!("statement is query"),
    };
    let clause = query
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::Where { .. }))
        .expect("where clause exists");

    let QueryClause::Where { expression } = clause else {
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
        Statement::Query(query) => query,
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
    let edge = match_clause
        .pattern
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
        Statement::Query(query) => query,
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
    let edge = match_clause
        .pattern
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
        Statement::Query(query) => query,
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
    let edge = match_clause
        .pattern
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
        Statement::Query(query) => query,
        _ => panic!("statement is query"),
    };
    let clause = query
        .clauses
        .get(1)
        .expect("query has where clause");

    let QueryClause::Where { expression } = clause else {
        panic!("expected where clause");
    };

    match expression {
        Expression::String(value, _) => assert_eq!(value, "hello"),
        other => panic!("unexpected expression: {other:?}"),
    }

    let parsed = parse("q", "LET a = 42");
    let lowered = lower_from_syntax(&parsed);

    let query = match lowered.statement {
        Statement::Query(query) => query,
        _ => panic!("statement is query"),
    };
    let clause = query
        .clauses
        .first()
        .expect("query has let clause");

    let QueryClause::Let { value, .. } = clause else {
        panic!("expected let clause");
    };

    match value {
        Expression::Integer(value, _) => assert_eq!(*value, 42),
        other => panic!("unexpected expression: {other:?}"),
    }
}

#[test]
fn lower_where_clause_with_missing_expression_emits_diagnostic() {
    let parsed = parse("q", "MATCH (a)-[:CALLS]->(b) WHERE RETURN a");
    let lowered = lower_from_syntax(&parsed);

    assert!(
        lowered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-AST-WHERE-MISSING-EXPRESSION")
    );
    let statement = match lowered.statement {
        Statement::Query(query) => query,
        _ => panic!("statement is query"),
    };
    let clause = statement
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::Where { .. }))
        .expect("where clause exists");

    let QueryClause::Where { expression } = clause else {
        unreachable!()
    };
    let Expression::Name(identifier) = expression else {
        panic!("unexpected expression: {expression:?}");
    };
    assert_eq!(identifier.text, "");
}

#[test]
fn lower_let_clause_missing_binding_and_value_emit_diagnostics() {
    let parsed = parse("q", "LET");
    let lowered = lower_from_syntax(&parsed);

    assert!(
        lowered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-AST-LET-BINDING-MISSING")
    );
    assert!(
        lowered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-AST-LET-VALUE-MISSING")
    );
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
    let query = match lowered.statement {
        Statement::Query(query) => query,
        _ => panic!("statement is query"),
    };
    let clause = query
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::Let { .. }))
        .expect("let clause exists");

    let QueryClause::Let { binding, .. } = clause else {
        unreachable!()
    };
    assert_eq!(binding.text, "");
    let QueryClause::Let { value, .. } = clause else {
        unreachable!()
    };
    assert!(matches!(value, Expression::Integer(2, _)));
}

#[test]
fn lower_let_clause_with_missing_binding_but_numeric_value() {
    let parsed = parse("q", "LET = 1");
    let lowered = lower_from_syntax(&parsed);

    assert!(
        lowered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-AST-LET-BINDING-MISSING")
    );
    assert!(
        lowered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-AST-LET-BINDING-EXPECTED")
    );
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
    let statement = match lowered.statement {
        Statement::Query(statement) => statement,
        _ => panic!("statement is query"),
    };
    let clause = statement
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::Let { .. }))
        .expect("let clause exists");
    let QueryClause::Let { binding, value, .. } = clause else {
        unreachable!()
    };

    assert_eq!(binding.text, "a");
    assert!(matches!(value, Expression::Name(_)));
}
