use super::{analyze, catalog, edge, identifier, node, query};
use gql_ast::{
    EdgeDirection, EdgePattern, Expression, MatchClause, PathPattern, PatternElement, QueryClause,
};
use gql_ir::{Expression as IrExpression, GraphPatternElement};
use gql_source::Span;

#[test]
fn named_path_binding_has_path_value_semantics() {
    let statement = query(vec![
        QueryClause::Match(MatchClause {
            mode: None,
            patterns: vec![PathPattern {
                binding: Some(identifier("p")),
                prefix: None,
                elements: vec![node("a"), edge("CALLS"), node("b")],
                span: Span::default(),
            }],
            keep: None,
            span: Span::default(),
        }),
        return_clause! { expressions: vec![Expression::Name(identifier("p"))],
        },
    ]);

    let result = analyze(&statement, &catalog());
    assert!(
        result.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.diagnostics
    );
    let ir = result.ir.expect("named path query should produce IR");
    let path = &ir.matches[0].paths[0];
    assert_eq!(path.binding.as_deref(), Some("P"));
    assert_eq!(path.elements.len(), 3);
    assert_eq!(
        ir.projection,
        vec![gql_ir::Projection {
            expression: IrExpression::Binding("P".into()),
            alias: None,
            value_type: gql_types::ValueType::Path,
        }]
    );
}

#[test]
fn bounded_path_quantifier_reaches_graph_ir() {
    let statement = query(vec![
        QueryClause::Match(MatchClause {
            mode: None,
            patterns: vec![PathPattern {
                binding: None,
                prefix: None,
                elements: vec![
                    node("a"),
                    PatternElement::Edge(EdgePattern {
                        binding: None,
                        labels: vec![identifier("CALLS")],
                        properties: Vec::new(),
                        predicate: None,
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
    let ir = result.ir.expect("quantified query should produce IR");
    assert!(matches!(
        ir.matches[0].paths[0].elements.as_slice(),
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
fn optional_match_preserves_one_group_and_exposes_new_binding() {
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
        QueryClause::OptionalMatch(MatchClause {
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
        return_clause! { expressions: vec![Expression::Name(identifier("b"))],
        },
    ]);

    let result = analyze(&statement, &catalog());
    assert!(
        result.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.diagnostics
    );
    let ir = result.ir.expect("optional query should produce IR");
    assert_eq!(ir.optional_matches.len(), 1);
    assert_eq!(ir.optional_matches[0].graph_match.paths.len(), 1);
    assert_eq!(
        ir.projection,
        vec![gql_ir::Projection {
            expression: IrExpression::Binding("B".into()),
            alias: None,
            value_type: gql_types::ValueType::Node,
        }]
    );

    let standalone = analyze(
        &query(vec![
            QueryClause::OptionalMatch(MatchClause {
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
        ]),
        &catalog(),
    );
    assert!(standalone.diagnostics.is_empty());
    assert_eq!(
        standalone
            .ir
            .expect("standalone optional MATCH")
            .optional_matches
            .len(),
        1
    );
}
