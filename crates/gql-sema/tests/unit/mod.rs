use crate::analyze;
use gql_ast::{
    BinaryOperator, EdgeDirection, EdgePattern, Expression, GraphPattern, Identifier, MatchClause,
    NodePattern, PathPattern, PatternElement, Query, QueryClause, Statement,
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
        properties: Vec::new(),
        span: Span::default(),
    })
}

fn edge(label: &str) -> PatternElement {
    PatternElement::Edge(EdgePattern {
        binding: None,
        labels: vec![identifier(label)],
        properties: Vec::new(),
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
fn return_projection_alias_reaches_canonical_ir() {
    let statement = query(vec![
        QueryClause::Match(MatchClause {
            pattern: GraphPattern {
                elements: vec![node("n")],
                span: Span::default(),
            },
            span: Span::default(),
        }),
        QueryClause::ReturnAliased {
            projections: vec![gql_ast::ReturnProjection {
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
            expression: IrExpression::Binding("n".into()),
            alias: Some("person".into()),
        }]
    );
}

#[test]
fn duplicate_return_projection_alias_is_rejected() {
    let statement = query(vec![QueryClause::ReturnAliased {
        projections: vec![
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
            pattern: GraphPattern {
                elements: vec![node("a")],
                span: Span::default(),
            },
            span: Span::default(),
        }),
        QueryClause::Return {
            expressions: vec![Expression::Name(identifier("a"))],
        },
        QueryClause::Union {
            span: Span::default(),
        },
        QueryClause::Match(MatchClause {
            pattern: GraphPattern {
                elements: vec![node("b")],
                span: Span::default(),
            },
            span: Span::default(),
        }),
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
    let ir = result.ir.expect("UNION IR");
    assert_eq!(ir.union_branches.len(), 1);
    assert!(
        matches!(ir.projection.as_slice(), [projection] if projection.expression == IrExpression::Binding("a".into()))
    );
    assert!(
        matches!(ir.union_branches[0].projection.as_slice(), [projection] if projection.expression == IrExpression::Binding("b".into()))
    );
}

#[test]
fn union_requires_a_query_block_on_both_sides() {
    let statement = query(vec![
        QueryClause::Return {
            expressions: vec![Expression::Integer(1, Span::default())],
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
        QueryClause::Return {
            expressions: vec![Expression::Integer(1, Span::default())],
        },
        QueryClause::Union {
            span: Span::default(),
        },
        QueryClause::Return {
            expressions: vec![
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
            pattern: GraphPattern {
                elements: vec![node("a")],
                span: Span::default(),
            },
            span: Span::default(),
        }),
        QueryClause::Return {
            expressions: vec![Expression::Name(identifier("a"))],
        },
        QueryClause::Union {
            span: Span::default(),
        },
        QueryClause::Return {
            expressions: vec![Expression::Name(identifier("a"))],
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
        QueryClause::Return {
            expressions: vec![Expression::Integer(1, Span::default())],
        },
        QueryClause::Limit {
            value: Some(10),
            span: Span::default(),
        },
        QueryClause::Union {
            span: Span::default(),
        },
        QueryClause::Return {
            expressions: vec![Expression::Integer(2, Span::default())],
        },
        QueryClause::Limit {
            value: Some(5),
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
    assert_eq!(ir.limit, Some(10));
    assert_eq!(ir.union_branches[0].limit, Some(5));
}

#[test]
fn limit_rejects_zero_and_duplicates() {
    let statement = query(vec![
        QueryClause::Limit {
            value: Some(0),
            span: Span::default(),
        },
        QueryClause::Limit {
            value: Some(1),
            span: Span::default(),
        },
        QueryClause::Limit {
            value: Some(2),
            span: Span::default(),
        },
    ]);
    let result = analyze(&statement, &catalog());
    assert!(result.ir.is_none());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "GQL-SEMA-LIMIT-NONPOSITIVE" })
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "GQL-SEMA-DUPLICATE-LIMIT" })
    );
}

#[test]
fn order_by_reaches_branch_local_canonical_ir() {
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
        QueryClause::OrderBy {
            keys: vec![gql_ast::SortKey {
                expression: Expression::Name(identifier("n")),
                direction: gql_ast::SortDirection::Descending,
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
    assert_eq!(ir.order_by[0].expression, IrExpression::Binding("n".into()));
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
        QueryClause::Return {
            expressions: vec![Expression::Integer(1, Span::default())],
        },
        QueryClause::Limit {
            value: Some(10),
            span: Span::default(),
        },
        QueryClause::Offset {
            value: Some(2),
            span: Span::default(),
        },
    ]);
    let result = analyze(&statement, &catalog());
    assert!(
        result.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.diagnostics
    );
    assert_eq!(result.ir.expect("OFFSET IR").offset, Some(2));
}

#[test]
fn offset_requires_limit_and_rejects_duplicates() {
    let statement = query(vec![
        QueryClause::Offset {
            value: Some(0),
            span: Span::default(),
        },
        QueryClause::Offset {
            value: Some(1),
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

#[test]
fn property_access_and_scalar_literals_lower_to_graph_ir() {
    let property = Expression::PropertyAccess {
        base: Box::new(Expression::Name(identifier("n"))),
        property: identifier("name"),
    };
    let statement = query(vec![
        QueryClause::Match(MatchClause {
            pattern: GraphPattern {
                elements: vec![node("n")],
                span: Span::default(),
            },
            span: Span::default(),
        }),
        QueryClause::Where {
            expression: Expression::Binary {
                operator: BinaryOperator::Equals,
                left: Box::new(property),
                right: Box::new(Expression::Boolean(true, Span::default())),
            },
        },
        QueryClause::Return {
            expressions: vec![Expression::Null(Span::default())],
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
        }] if matches!(left.as_ref(), IrExpression::PropertyAccess { property, .. } if property == "name")
            && matches!(right.as_ref(), IrExpression::Boolean(true))
    ));
    assert_eq!(
        ir.projection,
        vec![gql_ir::Projection {
            expression: IrExpression::Null,
            alias: None,
        }]
    );
}

#[test]
fn decimal_literal_preserves_decimal_value_type_in_ir() {
    let statement = query(vec![QueryClause::Return {
        expressions: vec![Expression::Decimal("1.25".into(), Span::default())],
    }]);

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
        }]
    );
}

#[test]
fn named_path_binding_has_path_value_semantics() {
    let statement = query(vec![
        QueryClause::Match(MatchClause {
            pattern: GraphPattern {
                elements: vec![PatternElement::Path(PathPattern {
                    binding: Some(identifier("p")),
                    elements: vec![node("a"), edge("CALLS"), node("b")],
                    span: Span::default(),
                })],
                span: Span::default(),
            },
            span: Span::default(),
        }),
        QueryClause::Return {
            expressions: vec![Expression::Name(identifier("p"))],
        },
    ]);

    let result = analyze(&statement, &catalog());
    assert!(
        result.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.diagnostics
    );
    let ir = result.ir.expect("named path query should produce IR");
    assert!(matches!(
        ir.graph.as_ref().expect("graph pattern").elements.as_slice(),
        [GraphPatternElement::Path(path)]
            if path.binding.as_deref() == Some("p") && path.elements.len() == 3
    ));
    assert_eq!(
        ir.projection,
        vec![gql_ir::Projection {
            expression: IrExpression::Binding("p".into()),
            alias: None,
        }]
    );
}

#[test]
fn bounded_path_quantifier_reaches_graph_ir() {
    let statement = query(vec![
        QueryClause::Match(MatchClause {
            pattern: GraphPattern {
                elements: vec![
                    node("a"),
                    PatternElement::Edge(EdgePattern {
                        binding: None,
                        labels: vec![identifier("CALLS")],
                        properties: Vec::new(),
                        direction: EdgeDirection::Out,
                        quantifier: Some(gql_ast::PathQuantifier {
                            min: 1,
                            max: Some(3),
                            span: Span::default(),
                        }),
                        span: Span::default(),
                    }),
                    node("b"),
                ],
                span: Span::default(),
            },
            span: Span::default(),
        }),
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
    let ir = result.ir.expect("quantified query should produce IR");
    assert!(matches!(
        ir.graph.as_ref().expect("graph pattern").elements.as_slice(),
        [
            GraphPatternElement::Node(_),
            GraphPatternElement::Edge(edge),
            GraphPatternElement::Node(_)
        ] if edge.quantifier
            .as_ref()
            .map(|quantifier| (quantifier.min, quantifier.max))
            == Some((1, Some(3)))
    ));
}

#[test]
fn optional_match_requires_mandatory_match_and_exposes_new_binding() {
    let statement = query(vec![
        QueryClause::Match(MatchClause {
            pattern: GraphPattern {
                elements: vec![node("a")],
                span: Span::default(),
            },
            span: Span::default(),
        }),
        QueryClause::OptionalMatch(MatchClause {
            pattern: GraphPattern {
                elements: vec![node("a"), edge("CALLS"), node("b")],
                span: Span::default(),
            },
            span: Span::default(),
        }),
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
    let ir = result.ir.expect("optional query should produce IR");
    assert_eq!(ir.optional_graphs.len(), 1);
    assert_eq!(
        ir.projection,
        vec![gql_ir::Projection {
            expression: IrExpression::Binding("b".into()),
            alias: None,
        }]
    );

    let invalid = analyze(
        &query(vec![
            QueryClause::OptionalMatch(MatchClause {
                pattern: GraphPattern {
                    elements: vec![node("a")],
                    span: Span::default(),
                },
                span: Span::default(),
            }),
            QueryClause::Return {
                expressions: vec![Expression::Name(identifier("a"))],
            },
        ]),
        &catalog(),
    );
    assert!(invalid.ir.is_none());
    assert!(
        invalid
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "GQL-SEMA-OPTIONAL-MATCH-WITHOUT-MANDATORY" })
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
        &query(vec![QueryClause::Return {
            expressions: vec![arithmetic],
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
        &query(vec![QueryClause::Return {
            expressions: vec![Expression::Binary {
                operator: BinaryOperator::Add,
                left: Box::new(Expression::Boolean(true, Span::default())),
                right: Box::new(Expression::Integer(1, Span::default())),
            }],
        }]),
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
        &query(vec![QueryClause::Return {
            expressions: vec![Expression::Binary {
                operator: BinaryOperator::Modulo,
                left: Box::new(Expression::Binary {
                    operator: BinaryOperator::Divide,
                    left: Box::new(Expression::Integer(8, Span::default())),
                    right: Box::new(Expression::Integer(2, Span::default())),
                }),
                right: Box::new(Expression::Integer(3, Span::default())),
            }],
        }]),
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
            QueryClause::Let {
                binding: identifier("values"),
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
            QueryClause::Return {
                expressions: vec![Expression::Name(identifier("values"))],
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
        &query(vec![QueryClause::Return {
            expressions: vec![Expression::Subscript {
                base: Box::new(Expression::List(
                    vec![Expression::Integer(1, Span::default())],
                    Span::default(),
                )),
                index: Box::new(Expression::Integer(0, Span::default())),
            }],
        }]),
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
        &query(vec![QueryClause::Return {
            expressions: vec![Expression::Subscript {
                base: Box::new(Expression::List(
                    vec![Expression::Integer(1, Span::default())],
                    Span::default(),
                )),
                index: Box::new(Expression::Boolean(true, Span::default())),
            }],
        }]),
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

#[test]
fn collection_membership_requires_list_rhs_and_lowers_to_ir() {
    let result = analyze(
        &query(vec![QueryClause::Return {
            expressions: vec![Expression::Binary {
                operator: BinaryOperator::In,
                left: Box::new(Expression::Integer(1, Span::default())),
                right: Box::new(Expression::List(
                    vec![Expression::Integer(1, Span::default())],
                    Span::default(),
                )),
            }],
        }]),
        &catalog(),
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert!(matches!(
        result.ir.expect("membership IR").projection.as_slice(),
        [gql_ir::Projection {
            expression: IrExpression::Binary {
                operator: gql_ir::BinaryOperator::In,
                ..
            },
            ..
        }]
    ));

    let invalid = analyze(
        &query(vec![QueryClause::Return {
            expressions: vec![Expression::Binary {
                operator: BinaryOperator::In,
                left: Box::new(Expression::Integer(1, Span::default())),
                right: Box::new(Expression::Integer(2, Span::default())),
            }],
        }]),
        &catalog(),
    );
    assert!(invalid.ir.is_none());
    assert!(
        invalid
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-SEMA-NON-LIST-MEMBERSHIP")
    );
}
