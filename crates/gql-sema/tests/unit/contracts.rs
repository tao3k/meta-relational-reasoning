use crate::analyze;
use gql_ast::{
    BinaryOperator, EdgeDirection, EdgePattern, Expression, Identifier, IdentifierForm,
    MatchClause, NodePattern, PathPattern, PatternElement, Query, QueryClause, Statement,
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
        form: IdentifierForm::Undelimited,
    }
}

fn node(binding: &str) -> PatternElement {
    PatternElement::Node(NodePattern {
        binding: Some(identifier(binding)),
        labels: Vec::new(),
        properties: Vec::new(),
        predicate: None,
        span: Span::default(),
    })
}

fn edge(label: &str) -> PatternElement {
    PatternElement::Edge(EdgePattern {
        binding: None,
        labels: vec![identifier(label)],
        properties: Vec::new(),
        predicate: None,
        direction: EdgeDirection::Out,
        quantifier: None,
        span: Span::default(),
    })
}

fn query(clauses: Vec<QueryClause>) -> Statement {
    Statement::Query(Query {
        clauses,
        span: Span::default(),
    })
}

macro_rules! return_clause {
    (expressions: $expressions:expr $(,)?) => {
        QueryClause::Return {
            quantifier: None,
            all_bindings: false,
            projections: $expressions
                .into_iter()
                .map(|expression| gql_ast::ReturnProjection {
                    expression,
                    alias: None,
                })
                .collect(),
            span: Span::default(),
        }
    };
    (projections: $projections:expr $(,)?) => {
        QueryClause::Return {
            quantifier: None,
            all_bindings: false,
            projections: $projections,
            span: Span::default(),
        }
    };
}

#[path = "path_contracts.rs"]
mod path_contracts;

macro_rules! let_clause {
    (binding: $binding:expr, value: $value:expr $(,)?) => {
        QueryClause::Let {
            bindings: vec![gql_ast::LetBinding {
                binding: $binding,
                value: $value,
                span: Span::default(),
            }],
            span: Span::default(),
        }
    };
}

#[path = "collection_membership.rs"]
mod collection_membership;

#[test]
fn node_only_match_is_valid_without_relation_catalog_entries() {
    let statement = query(vec![
        QueryClause::Match(MatchClause {
            mode: None,
            patterns: vec![PathPattern {
                binding: None,
                prefix: None,
                elements: vec![node("n")],
                span: Span::default(),
            }],
            keep: None,
            span: Span::default(),
        }),
        return_clause! { expressions: vec![Expression::Name(identifier("n"))],
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
        ir.matches[0]
            .paths
            .first()
            .expect("graph pattern")
            .elements
            .as_slice(),
        [GraphPatternElement::Node(_)]
    ));
    assert!(
        matches!(ir.projection.as_slice(), [projection] if projection.expression == IrExpression::Binding("N".into()))
    );
}

#[test]
fn return_projection_alias_reaches_canonical_ir() {
    let statement = query(vec![
        QueryClause::Match(MatchClause {
            mode: None,
            patterns: vec![PathPattern {
                binding: None,
                prefix: None,
                elements: vec![node("n")],
                span: Span::default(),
            }],
            keep: None,
            span: Span::default(),
        }),
        return_clause! { projections: vec![gql_ast::ReturnProjection {
                expression: Expression::Name(identifier("n")),
                alias: Some(identifier("person")),
            }],
        },
    ]);
    let result = analyze(&statement, &catalog());
    assert!(
        result.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.diagnostics
    );
    assert_eq!(
        result.ir.expect("alias IR").projection,
        vec![gql_ir::Projection {
            expression: IrExpression::Binding("N".into()),
            alias: Some("PERSON".into()),
            value_type: gql_types::ValueType::Node,
        }]
    );
}

#[test]
fn duplicate_return_projection_alias_is_rejected() {
    let statement = query(vec![return_clause! { projections: vec![
            gql_ast::ReturnProjection {
                expression: Expression::Integer(1, Span::default()),
                alias: Some(identifier("value")),
            },
            gql_ast::ReturnProjection {
                expression: Expression::Integer(2, Span::default()),
                alias: Some(identifier("value")),
            },
        ],
    }]);
    let result = analyze(&statement, &catalog());
    assert!(result.ir.is_none());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "GQL-SEMA-DUPLICATE-PROJECTION-ALIAS" })
    );
}

#[test]
fn union_query_has_independent_graph_native_branches() {
    let statement = query(vec![
        QueryClause::Match(MatchClause {
            mode: None,
            patterns: vec![PathPattern {
                binding: None,
                prefix: None,
                elements: vec![node("a")],
                span: Span::default(),
            }],
            keep: None,
            span: Span::default(),
        }),
        return_clause! { expressions: vec![Expression::Name(identifier("a"))],
        },
        QueryClause::Union {
            span: Span::default(),
        },
        QueryClause::Match(MatchClause {
            mode: None,
            patterns: vec![PathPattern {
                binding: None,
                prefix: None,
                elements: vec![node("b")],
                span: Span::default(),
            }],
            keep: None,
            span: Span::default(),
        }),
        return_clause! { expressions: vec![Expression::Name(identifier("b"))],
        },
    ]);
    let result = analyze(&statement, &catalog());
    assert!(
        result.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.diagnostics
    );
    let ir = result.ir.expect("UNION IR");
    assert_eq!(ir.set_operations.len(), 1);
    assert!(
        matches!(ir.projection.as_slice(), [projection] if projection.expression == IrExpression::Binding("A".into()))
    );
    assert!(
        matches!(ir.set_operations[0].right.projection.as_slice(), [projection] if projection.expression == IrExpression::Binding("B".into()))
    );
}

#[test]
fn union_requires_a_query_block_on_both_sides() {
    let statement = query(vec![
        return_clause! { expressions: vec![Expression::Integer(1, Span::default())],
        },
        QueryClause::Union {
            span: Span::default(),
        },
    ]);
    let result = analyze(&statement, &catalog());
    assert!(result.ir.is_none());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "GQL-SEMA-UNION-MISSING-BRANCH" })
    );
}

#[test]
fn union_requires_matching_projection_arity() {
    let statement = query(vec![
        return_clause! { expressions: vec![Expression::Integer(1, Span::default())],
        },
        QueryClause::Union {
            span: Span::default(),
        },
        return_clause! { expressions: vec![
                Expression::Integer(2, Span::default()),
                Expression::Integer(3, Span::default()),
            ],
        },
    ]);
    let result = analyze(&statement, &catalog());
    assert!(result.ir.is_none());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "GQL-SEMA-UNION-PROJECTION-ARITY" })
    );
}

#[test]
fn union_does_not_leak_bindings_between_branches() {
    let statement = query(vec![
        QueryClause::Match(MatchClause {
            mode: None,
            patterns: vec![PathPattern {
                binding: None,
                prefix: None,
                elements: vec![node("a")],
                span: Span::default(),
            }],
            keep: None,
            span: Span::default(),
        }),
        return_clause! { expressions: vec![Expression::Name(identifier("a"))],
        },
        QueryClause::Union {
            span: Span::default(),
        },
        return_clause! { expressions: vec![Expression::Name(identifier("a"))],
        },
    ]);
    let result = analyze(&statement, &catalog());
    assert!(result.ir.is_none());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "GQL-SEMA-UNRESOLVED-BINDING" })
    );
}

#[test]
fn limit_reaches_branch_local_canonical_ir() {
    let statement = query(vec![
        return_clause! { expressions: vec![Expression::Integer(1, Span::default())],
        },
        QueryClause::Limit {
            value: gql_ast::NonNegativeIntegerSpecification::Literal(10),
            span: Span::default(),
        },
        QueryClause::Union {
            span: Span::default(),
        },
        return_clause! { expressions: vec![Expression::Integer(2, Span::default())],
        },
        QueryClause::Limit {
            value: gql_ast::NonNegativeIntegerSpecification::Literal(5),
            span: Span::default(),
        },
    ]);
    let result = analyze(&statement, &catalog());
    assert!(
        result.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.diagnostics
    );
    let ir = result.ir.expect("LIMIT IR");
    assert_eq!(
        ir.limit,
        Some(gql_ir::NonNegativeIntegerSpecification::Literal(10))
    );
    assert_eq!(
        ir.set_operations[0].right.limit,
        Some(gql_ir::NonNegativeIntegerSpecification::Literal(5))
    );
}

#[test]
fn limit_accepts_zero_and_rejects_duplicates() {
    let statement = query(vec![
        return_clause! { expressions: vec![Expression::Integer(1, Span::default())],
        },
        QueryClause::Limit {
            value: gql_ast::NonNegativeIntegerSpecification::Literal(0),
            span: Span::default(),
        },
        QueryClause::Limit {
            value: gql_ast::NonNegativeIntegerSpecification::Literal(1),
            span: Span::default(),
        },
        QueryClause::Limit {
            value: gql_ast::NonNegativeIntegerSpecification::Literal(2),
            span: Span::default(),
        },
    ]);
    let result = analyze(&statement, &catalog());
    assert!(result.ir.is_none());
    assert_eq!(
        result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-DUPLICATE-LIMIT", "GQL-SEMA-DUPLICATE-LIMIT"]
    );
}

#[test]
fn order_by_reaches_branch_local_canonical_ir() {
    let statement = query(vec![
        QueryClause::Match(MatchClause {
            mode: None,
            patterns: vec![PathPattern {
                binding: None,
                prefix: None,
                elements: vec![node("n")],
                span: Span::default(),
            }],
            keep: None,
            span: Span::default(),
        }),
        return_clause! { expressions: vec![Expression::Name(identifier("n"))],
        },
        QueryClause::OrderBy {
            keys: vec![gql_ast::SortKey {
                expression: Expression::Name(identifier("n")),
                direction: Some(gql_ast::SortDirection::Descending),
                null_ordering: None,
            }],
            span: Span::default(),
        },
    ]);
    let result = analyze(&statement, &catalog());
    assert!(
        result.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.diagnostics
    );
    let ir = result.ir.expect("ORDER BY IR");
    assert_eq!(ir.order_by.len(), 1);
    assert_eq!(ir.order_by[0].direction, gql_ir::SortDirection::Descending);
    assert_eq!(ir.order_by[0].expression, IrExpression::Binding("N".into()));
}

#[test]
fn order_by_requires_at_least_one_expression() {
    let statement = query(vec![QueryClause::OrderBy {
        keys: Vec::new(),
        span: Span::default(),
    }]);
    let result = analyze(&statement, &catalog());
    assert!(result.ir.is_none());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "GQL-SEMA-ORDER-BY-MISSING-EXPRESSION" })
    );
}

#[test]
fn offset_reaches_branch_local_canonical_ir() {
    let statement = query(vec![
        return_clause! { expressions: vec![Expression::Integer(1, Span::default())],
        },
        QueryClause::Limit {
            value: gql_ast::NonNegativeIntegerSpecification::Literal(10),
            span: Span::default(),
        },
        QueryClause::Offset {
            value: gql_ast::NonNegativeIntegerSpecification::Literal(2),
            span: Span::default(),
        },
    ]);
    let result = analyze(&statement, &catalog());
    assert!(
        result.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.diagnostics
    );
    assert_eq!(
        result.ir.expect("OFFSET IR").offset,
        Some(gql_ir::NonNegativeIntegerSpecification::Literal(2))
    );
}

#[test]
fn offset_requires_limit_and_rejects_duplicates() {
    let statement = query(vec![
        QueryClause::Offset {
            value: gql_ast::NonNegativeIntegerSpecification::Literal(0),
            span: Span::default(),
        },
        QueryClause::Offset {
            value: gql_ast::NonNegativeIntegerSpecification::Literal(1),
            span: Span::default(),
        },
    ]);
    let result = analyze(&statement, &catalog());
    assert!(result.ir.is_none());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "GQL-SEMA-OFFSET-WITHOUT-LIMIT" })
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "GQL-SEMA-DUPLICATE-OFFSET" })
    );
}

#[test]
fn graph_pattern_filter_and_projection_are_canonical_ir() {
    let statement = query(vec![
        QueryClause::Match(MatchClause {
            mode: None,
            patterns: vec![PathPattern {
                binding: None,
                prefix: None,
                elements: vec![node("a"), edge("CALLS"), node("b")],
                span: Span::default(),
            }],
            keep: None,
            span: Span::default(),
        }),
        QueryClause::Where {
            expression: Expression::Binary {
                operator: BinaryOperator::Equals,
                left: Box::new(Expression::Name(identifier("a"))),
                right: Box::new(Expression::Integer(1, Span::default())),
            },
            span: Span::default(),
        },
        return_clause! { expressions: vec![Expression::Name(identifier("b"))],
        },
    ]);

    let result = analyze(&statement, &catalog());
    assert!(
        result.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.diagnostics
    );
    let ir = result.ir.expect("graph query should produce IR");
    assert_eq!(
        ir.matches[0]
            .paths
            .first()
            .expect("graph pattern")
            .elements
            .len(),
        3
    );
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
            mode: None,
            patterns: vec![PathPattern {
                binding: None,
                prefix: None,
                elements: vec![node("n")],
                span: Span::default(),
            }],
            keep: None,
            span: Span::default(),
        }),
        let_clause! { binding: identifier("limit"),
            value: Expression::Integer(1, Span::default()),
        },
        QueryClause::Where {
            expression: Expression::Binary {
                operator: BinaryOperator::Equals,
                left: Box::new(Expression::Name(identifier("limit"))),
                right: Box::new(Expression::Integer(1, Span::default())),
            },
            span: Span::default(),
        },
        return_clause! { expressions: vec![Expression::Name(identifier("n"))],
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
    assert_eq!(ir.let_bindings[0].binding.name, "LIMIT");
    assert!(matches!(
        ir.filters.as_slice(),
        [IrExpression::Binary {
            operator: IrBinaryOperator::Equals,
            ..
        }]
    ));
}

#[test]
fn unresolved_expression_binding_is_rejected_without_backend_lookup() {
    let statement = query(vec![
        QueryClause::Match(MatchClause {
            mode: None,
            patterns: vec![PathPattern {
                binding: None,
                prefix: None,
                elements: vec![node("n")],
                span: Span::default(),
            }],
            keep: None,
            span: Span::default(),
        }),
        QueryClause::Where {
            expression: Expression::Name(identifier("missing")),
            span: Span::default(),
        },
    ]);

    let result = analyze(&statement, &catalog());
    assert!(result.ir.is_none());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-SEMA-UNRESOLVED-BINDING")
    );
}

#[test]
fn property_access_and_scalar_literals_lower_to_graph_ir() {
    let property = Expression::PropertyAccess {
        base: Box::new(Expression::Name(identifier("n"))),
        property: identifier("name"),
    };
    let statement = query(vec![
        QueryClause::Match(MatchClause {
            mode: None,
            patterns: vec![PathPattern {
                binding: None,
                prefix: None,
                elements: vec![node("n")],
                span: Span::default(),
            }],
            keep: None,
            span: Span::default(),
        }),
        QueryClause::Where {
            expression: Expression::Binary {
                operator: BinaryOperator::Equals,
                left: Box::new(property),
                right: Box::new(Expression::Boolean(true, Span::default())),
            },
            span: Span::default(),
        },
        return_clause! { expressions: vec![Expression::Null(Span::default())],
        },
    ]);

    let result = analyze(&statement, &catalog());
    assert!(
        result.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.diagnostics
    );
    let ir = result.ir.expect("property query should produce IR");
    assert!(matches!(
        ir.filters.as_slice(),
        [IrExpression::Binary {
            left,
            right,
            operator: IrBinaryOperator::Equals,
        }] if matches!(left.as_ref(), IrExpression::PropertyAccess { property, .. } if property == "NAME")
            && matches!(right.as_ref(), IrExpression::Boolean(true))
    ));
    assert_eq!(
        ir.projection,
        vec![gql_ir::Projection {
            expression: IrExpression::Null,
            alias: None,
            value_type: gql_types::ValueType::Null,
        }]
    );
}

#[test]
fn decimal_literal_preserves_decimal_value_type_in_ir() {
    let statement = query(vec![
        return_clause! { expressions: vec![Expression::Decimal("1.25".into(), Span::default())],
        },
    ]);

    let result = analyze(&statement, &catalog());
    assert!(
        result.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.diagnostics
    );
    let ir = result.ir.expect("decimal query should produce IR");
    assert_eq!(
        ir.projection,
        vec![gql_ir::Projection {
            expression: IrExpression::Decimal("1.25".into()),
            alias: None,
            value_type: gql_types::ValueType::Decimal,
        }]
    );
}

#[test]
fn arithmetic_is_numeric_and_rejects_boolean_operands() {
    let arithmetic = Expression::Binary {
        operator: BinaryOperator::Add,
        left: Box::new(Expression::Integer(1, Span::default())),
        right: Box::new(Expression::Decimal("2.5".into(), Span::default())),
    };
    let result = analyze(
        &query(vec![return_clause! { expressions: vec![arithmetic],
        }]),
        &catalog(),
    );
    assert!(result.diagnostics.is_empty());
    assert!(matches!(
        result.ir.expect("arithmetic IR").projection.as_slice(),
        [gql_ir::Projection {
            expression: IrExpression::Binary {
                operator: gql_ir::BinaryOperator::Add,
                ..
            },
            ..
        }]
    ));

    let invalid = analyze(
        &query(vec![
            return_clause! { expressions: vec![Expression::Binary {
                    operator: BinaryOperator::Add,
                    left: Box::new(Expression::Boolean(true, Span::default())),
                    right: Box::new(Expression::Integer(1, Span::default())),
                }],
            },
        ]),
        &catalog(),
    );
    assert!(invalid.ir.is_none());
    assert!(
        invalid
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "GQL-SEMA-NON-NUMERIC-ARITHMETIC" })
    );
}

#[test]
fn division_and_modulo_lower_to_numeric_ir() {
    let result = analyze(
        &query(vec![
            return_clause! { expressions: vec![Expression::Binary {
                    operator: BinaryOperator::Modulo,
                    left: Box::new(Expression::Binary {
                        operator: BinaryOperator::Divide,
                        left: Box::new(Expression::Integer(8, Span::default())),
                        right: Box::new(Expression::Integer(2, Span::default())),
                    }),
                    right: Box::new(Expression::Integer(3, Span::default())),
                }],
            },
        ]),
        &catalog(),
    );
    assert!(result.diagnostics.is_empty());
    assert!(matches!(
        result.ir.expect("multiplicative IR").projection.as_slice(),
        [gql_ir::Projection {
            expression: IrExpression::Binary {
                operator: gql_ir::BinaryOperator::Modulo,
                left,
                ..
            },
            ..
        }] if matches!(left.as_ref(), IrExpression::Binary {
            operator: gql_ir::BinaryOperator::Divide,
            ..
        })
    ));
}

#[test]
fn list_value_lowers_to_ir_and_list_type() {
    let result = analyze(
        &query(vec![
            let_clause! { binding: identifier("values"),
                value: Expression::List(
                    vec![
                        Expression::Integer(1, Span::default()),
                        Expression::List(
                            vec![Expression::Integer(2, Span::default())],
                            Span::default(),
                        ),
                    ],
                    Span::default(),
                ),
            },
            return_clause! { expressions: vec![Expression::Name(identifier("values"))],
            },
        ]),
        &catalog(),
    );

    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let ir = result.ir.expect("list value IR");
    assert_eq!(
        ir.let_bindings[0].binding.value_type,
        gql_types::ValueType::List
    );
    assert!(matches!(
        ir.let_bindings[0].value,
        IrExpression::List(ref items) if items.len() == 2
    ));
}

#[test]
fn collection_subscript_requires_integer_index_and_lowers_to_ir() {
    let result = analyze(
        &query(vec![
            return_clause! { expressions: vec![Expression::Subscript {
                    base: Box::new(Expression::List(
                        vec![Expression::Integer(1, Span::default())],
                        Span::default(),
                    )),
                    index: Box::new(Expression::Integer(0, Span::default())),
                }],
            },
        ]),
        &catalog(),
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert!(matches!(
        result.ir.expect("subscript IR").projection.as_slice(),
        [gql_ir::Projection {
            expression: IrExpression::Subscript { .. },
            ..
        }]
    ));

    let invalid = analyze(
        &query(vec![
            return_clause! { expressions: vec![Expression::Subscript {
                    base: Box::new(Expression::List(
                        vec![Expression::Integer(1, Span::default())],
                        Span::default(),
                    )),
                    index: Box::new(Expression::Boolean(true, Span::default())),
                }],
            },
        ]),
        &catalog(),
    );
    assert!(invalid.ir.is_none());
    assert!(
        invalid
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-SEMA-NON-INTEGER-SUBSCRIPT")
    );
}
