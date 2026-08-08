use crate::analyze;
use gql_ir::Predicate;
use gql_ast::{
    EdgeDirection, EdgePattern, GraphPattern, Identifier, MatchClause, NodePattern,
    PatternElement, Query, QueryClause, Statement,
};
use gql_catalog::{
    CatalogName, GqlCatalog, GraphName, PredicateDescriptor, RelationAuthority,
    RelationIdentity, RelationName,
};
use gql_source::Span;

#[derive(Default)]
struct StubCatalog;

impl StubCatalog {
    fn new() -> Self {
        Self
    }
}

impl GqlCatalog for StubCatalog {
    fn relation(&self, _name: &RelationName) -> Option<PredicateDescriptor> {
        if _name.0 == "CALLS" {
            Some(PredicateDescriptor {
                name: _name.clone(),
                columns: Vec::new(),
                relation_identity: RelationIdentity {
                    catalog: CatalogName("default-catalog".into()),
                    graph: GraphName("default-graph".into()),
                    schema: None,
                    node_types: Vec::new(),
                    edge_types: Vec::new(),
                },
                authority: RelationAuthority::Asserted {
                    source: "unit-test".into(),
                },
            })
        } else {
            None
        }
    }
}

fn identifier(name: &str) -> Identifier {
    Identifier {
        text: name.to_string(),
        span: Span::default(),
    }
}

#[test]
fn match_clause_supports_single_edge_relation_only() {
    let statement = Statement::Query(Query {
        clauses: vec![QueryClause::Match(MatchClause {
            pattern: GraphPattern {
                elements: vec![
                    PatternElement::Node(NodePattern {
                        binding: Some(identifier("a")),
                        labels: vec![identifier("Person")],
                        span: Span::default(),
                    }),
                    PatternElement::Edge(EdgePattern {
                        labels: vec![identifier("CALLS")],
                        direction: EdgeDirection::Out,
                        span: Span::default(),
                    }),
                ],
                span: Span::default(),
            },
            span: Span::default(),
        })],
        span: Span::default(),
    });

    let result = analyze(&statement, &StubCatalog::new());

    assert!(result.diagnostics.is_empty());
    assert_eq!(result.ir.as_ref().expect("ir built").scans.len(), 1);
    assert_eq!(result.ir.unwrap().scans[0].relation.0, "CALLS");
}

#[test]
fn match_clause_errors_on_node_labels_without_edges() {
    let statement = Statement::Query(Query {
        clauses: vec![QueryClause::Match(MatchClause {
            pattern: GraphPattern {
                elements: vec![PatternElement::Node(NodePattern {
                    binding: Some(identifier("a")),
                    labels: vec![identifier("Person")],
                    span: Span::default(),
                })],
                span: Span::default(),
            },
            span: Span::default(),
        })],
        span: Span::default(),
    });

    let result = analyze(&statement, &StubCatalog::new());

    assert_eq!(result.ir, None);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code, "GQL-SEMA-NO-RELATION-HINT");
}

#[test]
fn match_clause_errors_on_unsupported_multi_relation_pattern() {
    let statement = Statement::Query(Query {
        clauses: vec![QueryClause::Match(MatchClause {
            pattern: GraphPattern {
                elements: vec![
                    PatternElement::Node(NodePattern {
                        binding: Some(identifier("a")),
                        labels: Vec::new(),
                        span: Span::default(),
                    }),
                    PatternElement::Edge(EdgePattern {
                        labels: vec![identifier("CALLS")],
                        direction: EdgeDirection::Out,
                        span: Span::default(),
                    }),
                    PatternElement::Node(NodePattern {
                        binding: Some(identifier("b")),
                        labels: Vec::new(),
                        span: Span::default(),
                    }),
                    PatternElement::Edge(EdgePattern {
                        labels: vec![identifier("DEPENDS_ON")],
                        direction: EdgeDirection::Out,
                        span: Span::default(),
                    }),
                    PatternElement::Node(NodePattern {
                        binding: Some(identifier("c")),
                        labels: Vec::new(),
                        span: Span::default(),
                    }),
                ],
                span: Span::default(),
            },
            span: Span::default(),
        })],
        span: Span::default(),
    });

    let result = analyze(&statement, &StubCatalog::new());

    assert_eq!(result.ir, None);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code, "GQL-SEMA-MULTI-RELATIONS-HINT");
}

#[test]
fn where_clause_with_bound_identifier_is_supported() {
    let statement = Statement::Query(Query {
        clauses: vec![
            QueryClause::Match(MatchClause {
                pattern: GraphPattern {
                    elements: vec![
                        PatternElement::Node(NodePattern {
                            binding: Some(identifier("a")),
                            labels: Vec::new(),
                            span: Span::default(),
                        }),
                        PatternElement::Edge(EdgePattern {
                            labels: vec![identifier("CALLS")],
                            direction: EdgeDirection::Out,
                            span: Span::default(),
                        }),
                        PatternElement::Node(NodePattern {
                            binding: Some(identifier("b")),
                            labels: Vec::new(),
                            span: Span::default(),
                        }),
                    ],
                    span: Span::default(),
                },
                span: Span::default(),
            }),
            QueryClause::Where {
                expression: gql_ast::Expression::Name(gql_ast::Identifier {
                    text: "a".into(),
                    span: Span::default(),
                }),
            },
            QueryClause::Return { expressions: vec![] },
        ],
        span: Span::default(),
    });

    let result = analyze(&statement, &StubCatalog::new());

    assert!(result.diagnostics.is_empty());
    assert!(result.ir.is_some());
    let ir = result.ir.expect("ir should be present");
    assert_eq!(ir.scans.len(), 1);
    assert_eq!(ir.projection.len(), 0);
}

#[test]
fn where_clause_with_unsupported_expression_reports_diagnostic() {
    let statement = Statement::Query(Query {
        clauses: vec![
            QueryClause::Match(MatchClause {
                pattern: GraphPattern {
                    elements: vec![
                        PatternElement::Node(NodePattern {
                            binding: Some(identifier("a")),
                            labels: Vec::new(),
                            span: Span::default(),
                        }),
                        PatternElement::Edge(EdgePattern {
                            labels: vec![identifier("CALLS")],
                            direction: EdgeDirection::Out,
                            span: Span::default(),
                        }),
                        PatternElement::Node(NodePattern {
                            binding: Some(identifier("b")),
                            labels: Vec::new(),
                            span: Span::default(),
                        }),
                    ],
                    span: Span::default(),
                },
                span: Span::default(),
            }),
            QueryClause::Where {
                expression: gql_ast::Expression::Integer(1, Span::default()),
            },
            QueryClause::Return { expressions: vec![] },
        ],
        span: Span::default(),
    });

    let result = analyze(&statement, &StubCatalog::new());

    assert_eq!(result.ir, None);
    assert!(result
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "GQL-SEMA-WHERE-UNSUPPORTED-EXPRESSION"));
}

#[test]
fn where_clause_with_binding_equality_to_integer_is_supported() {
    let statement = Statement::Query(Query {
        clauses: vec![
            QueryClause::Match(MatchClause {
                pattern: GraphPattern {
                    elements: vec![
                        PatternElement::Node(NodePattern {
                            binding: Some(identifier("a")),
                            labels: Vec::new(),
                            span: Span::default(),
                        }),
                        PatternElement::Edge(EdgePattern {
                            labels: vec![identifier("CALLS")],
                            direction: EdgeDirection::Out,
                            span: Span::default(),
                        }),
                        PatternElement::Node(NodePattern {
                            binding: Some(identifier("b")),
                            labels: Vec::new(),
                            span: Span::default(),
                        }),
                    ],
                    span: Span::default(),
                },
                span: Span::default(),
            }),
            QueryClause::Where {
                expression: gql_ast::Expression::Binary {
                    operator: gql_ast::BinaryOperator::Equals,
                    left: Box::new(gql_ast::Expression::Name(gql_ast::Identifier {
                        text: "a".into(),
                        span: Span::default(),
                    })),
                    right: Box::new(gql_ast::Expression::Integer(1, Span::default())),
                },
            },
            QueryClause::Return { expressions: vec![] },
        ],
        span: Span::default(),
    });

    let result = analyze(&statement, &StubCatalog::new());

    assert!(result.diagnostics.is_empty());
    assert!(result.ir.is_some());
    assert_eq!(result.ir.as_ref().expect("ir").predicates.len(), 1);
    match &result.ir.as_ref().expect("ir").predicates[0] {
        Predicate::Equals(binding, value) => {
            assert_eq!(binding.name, "a");
    assert_eq!(value, &gql_types::Value::Integer(1));
        }
        predicate => panic!("unexpected predicate: {predicate:?}"),
    }
}

#[test]
fn where_clause_with_binding_equality_to_string_is_supported() {
    let statement = Statement::Query(Query {
        clauses: vec![
            QueryClause::Match(MatchClause {
                pattern: GraphPattern {
                    elements: vec![
                        PatternElement::Node(NodePattern {
                            binding: Some(identifier("a")),
                            labels: Vec::new(),
                            span: Span::default(),
                        }),
                        PatternElement::Edge(EdgePattern {
                            labels: vec![identifier("CALLS")],
                            direction: EdgeDirection::Out,
                            span: Span::default(),
                        }),
                        PatternElement::Node(NodePattern {
                            binding: Some(identifier("b")),
                            labels: Vec::new(),
                            span: Span::default(),
                        }),
                    ],
                    span: Span::default(),
                },
                span: Span::default(),
            }),
            QueryClause::Where {
                expression: gql_ast::Expression::Binary {
                    operator: gql_ast::BinaryOperator::Equals,
                    left: Box::new(gql_ast::Expression::Name(gql_ast::Identifier {
                        text: "a".into(),
                        span: Span::default(),
                    })),
                    right: Box::new(gql_ast::Expression::String("x".into(), Span::default())),
                },
            },
            QueryClause::Return { expressions: vec![] },
        ],
        span: Span::default(),
    });

    let result = analyze(&statement, &StubCatalog::new());

    assert!(result.diagnostics.is_empty());
    assert!(result.ir.is_some());
    assert_eq!(result.ir.as_ref().expect("ir").predicates.len(), 1);
    match &result.ir.as_ref().expect("ir").predicates[0] {
        Predicate::Equals(binding, value) => {
            assert_eq!(binding.name, "a");
            assert_eq!(value, &gql_types::Value::String("x".into()));
        }
        predicate => panic!("unexpected predicate: {predicate:?}"),
    }
}

#[test]
fn where_clause_with_string_equality_to_binding_is_supported() {
    let statement = Statement::Query(Query {
        clauses: vec![
            QueryClause::Match(MatchClause {
                pattern: GraphPattern {
                    elements: vec![
                        PatternElement::Node(NodePattern {
                            binding: Some(identifier("a")),
                            labels: Vec::new(),
                            span: Span::default(),
                        }),
                        PatternElement::Edge(EdgePattern {
                            labels: vec![identifier("CALLS")],
                            direction: EdgeDirection::Out,
                            span: Span::default(),
                        }),
                        PatternElement::Node(NodePattern {
                            binding: Some(identifier("b")),
                            labels: Vec::new(),
                            span: Span::default(),
                        }),
                    ],
                    span: Span::default(),
                },
                span: Span::default(),
            }),
            QueryClause::Where {
                expression: gql_ast::Expression::Binary {
                    operator: gql_ast::BinaryOperator::Equals,
                    left: Box::new(gql_ast::Expression::String("x".into(), Span::default())),
                    right: Box::new(gql_ast::Expression::Name(gql_ast::Identifier {
                        text: "a".into(),
                        span: Span::default(),
                    })),
                },
            },
            QueryClause::Return { expressions: vec![] },
        ],
        span: Span::default(),
    });

    let result = analyze(&statement, &StubCatalog::new());

    assert!(result.diagnostics.is_empty());
    assert!(result.ir.is_some());
    assert_eq!(result.ir.as_ref().expect("ir").predicates.len(), 1);
    match &result.ir.as_ref().expect("ir").predicates[0] {
        Predicate::Equals(binding, value) => {
            assert_eq!(binding.name, "a");
            assert_eq!(value, &gql_types::Value::String("x".into()));
        }
        predicate => panic!("unexpected predicate: {predicate:?}"),
    }
}

#[test]
fn where_clause_with_name_to_name_equality_is_unsupported() {
    let statement = Statement::Query(Query {
        clauses: vec![
            QueryClause::Match(MatchClause {
                pattern: GraphPattern {
                    elements: vec![
                        PatternElement::Node(NodePattern {
                            binding: Some(identifier("a")),
                            labels: Vec::new(),
                            span: Span::default(),
                        }),
                        PatternElement::Edge(EdgePattern {
                            labels: vec![identifier("CALLS")],
                            direction: EdgeDirection::Out,
                            span: Span::default(),
                        }),
                        PatternElement::Node(NodePattern {
                            binding: Some(identifier("b")),
                            labels: Vec::new(),
                            span: Span::default(),
                        }),
                    ],
                    span: Span::default(),
                },
                span: Span::default(),
            }),
            QueryClause::Where {
                expression: gql_ast::Expression::Binary {
                    operator: gql_ast::BinaryOperator::Equals,
                    left: Box::new(gql_ast::Expression::Name(gql_ast::Identifier {
                        text: "a".into(),
                        span: Span::default(),
                    })),
                    right: Box::new(gql_ast::Expression::Name(gql_ast::Identifier {
                        text: "b".into(),
                        span: Span::default(),
                    })),
                },
            },
            QueryClause::Return { expressions: vec![] },
        ],
        span: Span::default(),
    });

    let result = analyze(&statement, &StubCatalog::new());

    assert_eq!(result.ir, None);
    assert!(result
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "GQL-SEMA-WHERE-UNSUPPORTED-EXPRESSION"));
}

#[test]
fn where_clause_with_unsupported_string_expression_reports_diagnostic() {
    let statement = Statement::Query(Query {
        clauses: vec![
            QueryClause::Match(MatchClause {
                pattern: GraphPattern {
                    elements: vec![
                        PatternElement::Node(NodePattern {
                            binding: Some(identifier("a")),
                            labels: Vec::new(),
                            span: Span::default(),
                        }),
                        PatternElement::Edge(EdgePattern {
                            labels: vec![identifier("CALLS")],
                            direction: EdgeDirection::Out,
                            span: Span::default(),
                        }),
                        PatternElement::Node(NodePattern {
                            binding: Some(identifier("b")),
                            labels: Vec::new(),
                            span: Span::default(),
                        }),
                    ],
                    span: Span::default(),
                },
                span: Span::default(),
            }),
            QueryClause::Where {
                expression: gql_ast::Expression::String("hello".into(), Span::default()),
            },
            QueryClause::Return { expressions: vec![] },
        ],
        span: Span::default(),
    });

    let result = analyze(&statement, &StubCatalog::new());

    assert_eq!(result.ir, None);
    assert!(result
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "GQL-SEMA-WHERE-UNSUPPORTED-EXPRESSION"));
}

#[test]
fn where_clause_with_boolean_operator_is_unsupported() {
    let statement = Statement::Query(Query {
        clauses: vec![
            QueryClause::Match(MatchClause {
                pattern: GraphPattern {
                    elements: vec![
                        PatternElement::Node(NodePattern {
                            binding: Some(identifier("a")),
                            labels: Vec::new(),
                            span: Span::default(),
                        }),
                        PatternElement::Edge(EdgePattern {
                            labels: vec![identifier("CALLS")],
                            direction: EdgeDirection::Out,
                            span: Span::default(),
                        }),
                        PatternElement::Node(NodePattern {
                            binding: Some(identifier("b")),
                            labels: Vec::new(),
                            span: Span::default(),
                        }),
                    ],
                    span: Span::default(),
                },
                span: Span::default(),
            }),
            QueryClause::Where {
                expression: gql_ast::Expression::Binary {
                    operator: gql_ast::BinaryOperator::And,
                    left: Box::new(gql_ast::Expression::Binary {
                        operator: gql_ast::BinaryOperator::Equals,
                        left: Box::new(gql_ast::Expression::Name(gql_ast::Identifier {
                            text: "a".into(),
                            span: Span::default(),
                        })),
                        right: Box::new(gql_ast::Expression::Integer(1, Span::default())),
                    }),
                    right: Box::new(gql_ast::Expression::Binary {
                        operator: gql_ast::BinaryOperator::Equals,
                        left: Box::new(gql_ast::Expression::Name(gql_ast::Identifier {
                            text: "b".into(),
                            span: Span::default(),
                        })),
                        right: Box::new(gql_ast::Expression::Integer(2, Span::default())),
                    }),
                },
            },
            QueryClause::Return { expressions: vec![] },
        ],
        span: Span::default(),
    });

    let result = analyze(&statement, &StubCatalog::new());

    assert_eq!(result.ir, None);
    assert!(result
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "GQL-SEMA-WHERE-UNSUPPORTED-EXPRESSION"));
}

#[test]
fn where_clause_reports_unresolved_binding() {
    let statement = Statement::Query(Query {
        clauses: vec![QueryClause::Where {
            expression: gql_ast::Expression::Name(gql_ast::Identifier {
                text: "missing".into(),
                span: Span::default(),
            }),
        }],
        span: Span::default(),
    });

    let result = analyze(&statement, &StubCatalog::new());

    assert_eq!(result.ir, None);
    assert!(result
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "GQL-SEMA-WHERE-UNRESOLVED-BINDING"));
}

#[test]
fn where_clause_with_equality_to_unbound_rhs_name_reports_unresolved_binding() {
    let statement = Statement::Query(Query {
        clauses: vec![
            QueryClause::Match(MatchClause {
                pattern: GraphPattern {
                    elements: vec![
                        PatternElement::Node(NodePattern {
                            binding: Some(identifier("a")),
                            labels: Vec::new(),
                            span: Span::default(),
                        }),
                        PatternElement::Edge(EdgePattern {
                            labels: vec![identifier("CALLS")],
                            direction: EdgeDirection::Out,
                            span: Span::default(),
                        }),
                        PatternElement::Node(NodePattern {
                            binding: Some(identifier("b")),
                            labels: Vec::new(),
                            span: Span::default(),
                        }),
                    ],
                    span: Span::default(),
                },
                span: Span::default(),
            }),
            QueryClause::Where {
                expression: gql_ast::Expression::Binary {
                    operator: gql_ast::BinaryOperator::Equals,
                    left: Box::new(gql_ast::Expression::Name(gql_ast::Identifier {
                        text: "a".into(),
                        span: Span::default(),
                    })),
                    right: Box::new(gql_ast::Expression::Name(gql_ast::Identifier {
                        text: "missing".into(),
                        span: Span::default(),
                    })),
                },
            },
        ],
        span: Span::default(),
    });

    let result = analyze(&statement, &StubCatalog::new());

    assert_eq!(result.ir, None);
    assert!(result
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "GQL-SEMA-WHERE-UNRESOLVED-BINDING"));
}

#[test]
fn let_clause_bindings_add_to_scope_for_where() {
    let statement = Statement::Query(Query {
        clauses: vec![
            QueryClause::Let {
                binding: gql_ast::Identifier {
                    text: "a".into(),
                    span: Span::default(),
                },
                value: gql_ast::Expression::Integer(1, Span::default()),
            },
            QueryClause::Where {
                expression: gql_ast::Expression::Name(gql_ast::Identifier {
                    text: "a".into(),
                    span: Span::default(),
                }),
            },
        ],
        span: Span::default(),
    });

    let result = analyze(&statement, &StubCatalog::new());

    assert!(result.diagnostics.is_empty());
    assert!(result.ir.is_some());
}

#[test]
fn let_clause_reports_duplicate_binding() {
    let statement = Statement::Query(Query {
        clauses: vec![
            QueryClause::Match(MatchClause {
                pattern: GraphPattern {
                    elements: vec![
                        PatternElement::Node(NodePattern {
                            binding: Some(identifier("a")),
                            labels: Vec::new(),
                            span: Span::default(),
                        }),
                        PatternElement::Edge(EdgePattern {
                            labels: vec![identifier("CALLS")],
                            direction: EdgeDirection::Out,
                            span: Span::default(),
                        }),
                        PatternElement::Node(NodePattern {
                            binding: Some(identifier("b")),
                            labels: Vec::new(),
                            span: Span::default(),
                        }),
                    ],
                    span: Span::default(),
                },
                span: Span::default(),
            }),
            QueryClause::Let {
                binding: gql_ast::Identifier {
                    text: "a".into(),
                    span: Span::default(),
                },
                value: gql_ast::Expression::Integer(1, Span::default()),
            },
        ],
        span: Span::default(),
    });

    let result = analyze(&statement, &StubCatalog::new());

    assert_eq!(result.ir, None);
    assert!(result
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "GQL-SEMA-LET-DUPLICATE-BINDING"));
}

#[test]
fn let_clause_reports_duplicate_binding_with_previous_let() {
    let statement = Statement::Query(Query {
        clauses: vec![
            QueryClause::Let {
                binding: gql_ast::Identifier {
                    text: "a".into(),
                    span: Span::default(),
                },
                value: gql_ast::Expression::Integer(1, Span::default()),
            },
            QueryClause::Let {
                binding: gql_ast::Identifier {
                    text: "a".into(),
                    span: Span::default(),
                },
                value: gql_ast::Expression::Integer(2, Span::default()),
            },
        ],
        span: Span::default(),
    });

    let result = analyze(&statement, &StubCatalog::new());

    assert_eq!(result.ir, None);
    assert!(result
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "GQL-SEMA-LET-DUPLICATE-BINDING"));
}

#[test]
fn let_clause_value_references_unknown_binding() {
    let statement = Statement::Query(Query {
        clauses: vec![QueryClause::Let {
            binding: gql_ast::Identifier {
                text: "a".into(),
                span: Span::default(),
            },
            value: gql_ast::Expression::Name(gql_ast::Identifier {
                text: "missing".into(),
                span: Span::default(),
            }),
        }],
        span: Span::default(),
    });

    let result = analyze(&statement, &StubCatalog::new());

    assert_eq!(result.ir, None);
    assert!(result
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "GQL-SEMA-LET-VALUE-UNRESOLVED-BINDING"));
}

#[test]
fn let_clause_with_unresolved_value_does_not_bind_for_where() {
    let statement = Statement::Query(Query {
        clauses: vec![
            QueryClause::Let {
                binding: gql_ast::Identifier {
                    text: "a".into(),
                    span: Span::default(),
                },
                value: gql_ast::Expression::Name(gql_ast::Identifier {
                    text: "missing".into(),
                    span: Span::default(),
                }),
            },
            QueryClause::Where {
                expression: gql_ast::Expression::Name(gql_ast::Identifier {
                    text: "a".into(),
                    span: Span::default(),
                }),
            },
        ],
        span: Span::default(),
    });

    let result = analyze(&statement, &StubCatalog::new());

    assert_eq!(result.ir, None);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-SEMA-LET-VALUE-UNRESOLVED-BINDING")
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-SEMA-WHERE-UNRESOLVED-BINDING")
    );
}
