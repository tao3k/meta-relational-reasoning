use crate::analyze;
use gql_ast::{
    BinaryOperator, EdgeDirection, EdgePattern, Expression, GraphPattern, Identifier, MatchClause,
    NodePattern, PatternElement, Query, QueryClause, Statement,
};
use gql_catalog::{Catalog, CatalogName};
use gql_ir::{BinaryOperator as IrBinaryOperator, Expression as IrExpression, GraphPatternElement};
use gql_source::Span;

fn catalog() -> Catalog {
    Catalog::new(CatalogName("test-catalog".into()), Vec::new(), Vec::new())
}

fn identifier(name: &str) -> Identifier {
    Identifier {
        text: name.into(),
        span: Span::default(),
    }
}

fn node(binding: &str) -> PatternElement {
    PatternElement::Node(NodePattern {
        binding: Some(identifier(binding)),
        labels: Vec::new(),
        span: Span::default(),
    })
}

fn edge(label: &str) -> PatternElement {
    PatternElement::Edge(EdgePattern {
        labels: vec![identifier(label)],
        direction: EdgeDirection::Out,
        span: Span::default(),
    })
}

fn query(clauses: Vec<QueryClause>) -> Statement {
    Statement::Query(Query {
        clauses,
        span: Span::default(),
    })
}

#[test]
fn node_only_match_is_valid_without_relation_catalog_entries() {
    let statement = query(vec![
        QueryClause::Match(MatchClause {
            pattern: GraphPattern {
                elements: vec![node("n")],
                span: Span::default(),
            },
            span: Span::default(),
        }),
        QueryClause::Return {
            expressions: vec![Expression::Name(identifier("n"))],
        },
    ]);

    let result = analyze(&statement, &catalog());
    assert!(
        result.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.diagnostics
    );
    let ir = result.ir.expect("node-only query should produce IR");
    assert!(matches!(
        ir.graph
            .as_ref()
            .expect("graph pattern")
            .elements
            .as_slice(),
        [GraphPatternElement::Node(_)]
    ));
    assert!(
        matches!(ir.projection.as_slice(), [projection] if projection.expression == IrExpression::Binding("n".into()))
    );
}

#[test]
fn graph_pattern_filter_and_projection_are_canonical_ir() {
    let statement = query(vec![
        QueryClause::Match(MatchClause {
            pattern: GraphPattern {
                elements: vec![node("a"), edge("CALLS"), node("b")],
                span: Span::default(),
            },
            span: Span::default(),
        }),
        QueryClause::Where {
            expression: Expression::Binary {
                operator: BinaryOperator::Equals,
                left: Box::new(Expression::Name(identifier("a"))),
                right: Box::new(Expression::Integer(1, Span::default())),
            },
        },
        QueryClause::Return {
            expressions: vec![Expression::Name(identifier("b"))],
        },
    ]);

    let result = analyze(&statement, &catalog());
    assert!(
        result.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.diagnostics
    );
    let ir = result.ir.expect("graph query should produce IR");
    assert_eq!(ir.graph.expect("graph pattern").elements.len(), 3);
    assert!(matches!(
        ir.filters.as_slice(),
        [IrExpression::Binary {
            operator: IrBinaryOperator::Equals,
            ..
        }]
    ));
    assert_eq!(ir.projection.len(), 1);
}

#[test]
fn let_binding_is_preserved_as_semantic_ir() {
    let statement = query(vec![
        QueryClause::Match(MatchClause {
            pattern: GraphPattern {
                elements: vec![node("n")],
                span: Span::default(),
            },
            span: Span::default(),
        }),
        QueryClause::Let {
            binding: identifier("limit"),
            value: Expression::Integer(1, Span::default()),
        },
        QueryClause::Where {
            expression: Expression::Name(identifier("limit")),
        },
        QueryClause::Return {
            expressions: vec![Expression::Name(identifier("n"))],
        },
    ]);

    let result = analyze(&statement, &catalog());
    assert!(
        result.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.diagnostics
    );
    let ir = result.ir.expect("LET query should produce IR");
    assert_eq!(ir.let_bindings.len(), 1);
    assert_eq!(ir.let_bindings[0].binding.name, "limit");
    assert_eq!(ir.filters, vec![IrExpression::Binding("limit".into())]);
}

#[test]
fn unresolved_expression_binding_is_rejected_without_backend_lookup() {
    let statement = query(vec![QueryClause::Where {
        expression: Expression::Name(identifier("missing")),
    }]);

    let result = analyze(&statement, &catalog());
    assert!(result.ir.is_none());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-SEMA-UNRESOLVED-BINDING")
    );
}
