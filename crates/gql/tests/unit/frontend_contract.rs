use crate::Compiler;
use crate::catalog::{Catalog, CatalogName};
use crate::syntax::{SyntaxElementKind, SyntaxKind, SyntaxNode, parse};

fn empty_catalog() -> Catalog {
    Catalog::new(
        CatalogName("frontend-contract".into()),
        Vec::new(),
        Vec::new(),
    )
}

fn contains_node_kind(node: &SyntaxNode, expected: SyntaxKind) -> bool {
    node.kind() == expected
        || node
            .children()
            .into_iter()
            .any(|element| match element.kind {
                SyntaxElementKind::Node(child) => contains_node_kind(&child, expected),
                SyntaxElementKind::Token(_) => false,
            })
}

fn count_node_kind(node: &SyntaxNode, expected: SyntaxKind) -> usize {
    usize::from(node.kind() == expected)
        + node
            .children()
            .into_iter()
            .map(|element| match element.kind {
                SyntaxElementKind::Node(child) => count_node_kind(&child, expected),
                SyntaxElementKind::Token(_) => 0,
            })
            .sum::<usize>()
}

#[test]
fn rowan_is_lossless_for_valid_and_recovered_source() {
    for source in [
        "",
        "   \n\t  ",
        "# comment only\n",
        "MATCH (n) RETURN n\n",
        "MATCH (a)-[:CALLS]->(b) /x @",
        "MATCH (a)-[:CALLS:Person->(b) RETURN a",
        "MATCH (a) RETURN a UNION",
    ] {
        let parsed = parse("frontend-contract.gql", source);
        assert_eq!(
            parsed.tree.rowan_root().text().to_string(),
            source,
            "Rowan must preserve source bytes for {source:?}"
        );
    }
}

#[test]
fn typed_view_exposes_structural_rowan_nodes() {
    let parsed = parse(
        "frontend-contract.gql",
        "MATCH (a)-[:CALLS]->(b) WHERE a = 1 RETURN b",
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);

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
            contains_node_kind(&parsed.tree.root(), kind),
            "missing typed Rowan node {kind:?}"
        );
    }
}

#[test]
fn graph_semantic_vertical_slices_do_not_require_relation_catalog_entries() {
    let compiler = Compiler;

    let node_only = compiler.compile("node-only.gql", "MATCH (n) RETURN n", &empty_catalog());
    assert!(node_only.parse.diagnostics.is_empty());
    assert!(node_only.analysis.diagnostics.is_empty());
    assert_eq!(
        node_only
            .analysis
            .ir
            .as_ref()
            .and_then(|query| query.matches.first())
            .and_then(|graph_match| graph_match.paths.first())
            .map(|path| path.elements.len()),
        Some(1)
    );

    let graph_filter = compiler.compile(
        "graph-filter.gql",
        "MATCH (a)-[:CALLS]->(b) WHERE a = 1 RETURN b",
        &empty_catalog(),
    );
    assert!(graph_filter.parse.diagnostics.is_empty());
    assert!(graph_filter.analysis.diagnostics.is_empty());
    let ir = graph_filter.analysis.ir.expect("graph semantic IR");
    assert_eq!(ir.matches[0].paths[0].elements.len(), 3);
    assert_eq!(ir.filters.len(), 1);
    assert_eq!(ir.projection.len(), 1);
}

#[test]
fn node_pattern_properties_survive_cst_ast_and_canonical_ir() {
    let source = "MATCH (n:Person {name: 'Ada', age: 42, active: TRUE}) RETURN n";
    let result = Compiler.compile("node-properties.gql", source, &empty_catalog());

    assert!(
        result.parse.diagnostics.is_empty(),
        "parse: {:?}",
        result.parse.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    let root = result.parse.tree.root();
    assert_eq!(count_node_kind(&root, SyntaxKind::PropertyMap), 1);
    assert_eq!(count_node_kind(&root, SyntaxKind::PropertyEntry), 3);

    let Some(crate::ast::Statement::Query(query)) = &result.statement else {
        panic!("statement is query");
    };
    let Some(crate::ast::QueryClause::Match(match_clause)) = query.clauses.first() else {
        panic!("MATCH clause exists");
    };
    let Some(crate::ast::PatternElement::Node(node)) = match_clause.patterns[0].elements.first()
    else {
        panic!("node pattern exists");
    };
    assert_eq!(
        node.properties
            .iter()
            .map(|property| property.key.text.as_str())
            .collect::<Vec<_>>(),
        ["name", "age", "active"]
    );
    assert!(
        matches!(node.properties[0].value, crate::ast::Expression::String(ref literal) if literal.value == "Ada")
    );
    assert!(matches!(
        node.properties[1].value,
        crate::ast::Expression::Integer(42, _)
    ));
    assert!(matches!(
        node.properties[2].value,
        crate::ast::Expression::Boolean(true, _)
    ));
    assert_eq!(
        node.properties
            .iter()
            .map(|property| &source[property.span.start as usize..property.span.end as usize])
            .collect::<Vec<_>>(),
        ["name: 'Ada'", "age: 42", "active: TRUE"]
    );

    assert!(
        result.analysis.diagnostics.is_empty(),
        "sema: {:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("property-pattern IR");
    let Some(crate::ir::GraphPatternElement::Node(node)) = ir
        .matches
        .into_iter()
        .next()
        .expect("graph match")
        .paths
        .into_iter()
        .next()
        .expect("path pattern")
        .elements
        .into_iter()
        .next()
    else {
        panic!("canonical node pattern exists");
    };
    assert_eq!(
        node.properties
            .iter()
            .map(|property| property.key.as_str())
            .collect::<Vec<_>>(),
        ["NAME", "AGE", "ACTIVE"]
    );
    assert!(
        matches!(node.properties[0].value, crate::ir::Expression::String(ref value) if value == "Ada")
    );
    assert!(matches!(
        node.properties[1].value,
        crate::ir::Expression::Integer(42)
    ));
    assert!(matches!(
        node.properties[2].value,
        crate::ir::Expression::Boolean(true)
    ));
}

#[test]
fn duplicate_node_pattern_property_is_typed_and_emits_no_ir() {
    let result = Compiler.compile(
        "duplicate-node-property.gql",
        "MATCH (n {id: 1, id: 2}) RETURN n",
        &empty_catalog(),
    );

    assert!(
        result.parse.diagnostics.is_empty(),
        "parse: {:?}",
        result.parse.diagnostics
    );
    assert!(
        result.analysis.ir.is_none(),
        "invalid property pattern must not emit IR"
    );
    assert_eq!(
        result
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-DUPLICATE-PATTERN-PROPERTY"]
    );
}

#[test]
fn edge_pattern_properties_survive_cst_ast_and_canonical_ir() {
    let source = "MATCH (a)-[e:KNOWS {since: 2020, weight: 3}]->(b) RETURN e";
    let result = Compiler.compile("edge-properties.gql", source, &empty_catalog());

    assert!(
        result.parse.diagnostics.is_empty(),
        "parse: {:?}",
        result.parse.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    let root = result.parse.tree.root();
    assert_eq!(count_node_kind(&root, SyntaxKind::PropertyMap), 1);
    assert_eq!(count_node_kind(&root, SyntaxKind::PropertyEntry), 2);

    let Some(crate::ast::Statement::Query(query)) = &result.statement else {
        panic!("statement is query");
    };
    let Some(crate::ast::QueryClause::Match(match_clause)) = query.clauses.first() else {
        panic!("MATCH clause exists");
    };
    let Some(crate::ast::PatternElement::Edge(edge)) = match_clause.patterns[0].elements.get(1)
    else {
        panic!("edge pattern exists");
    };
    assert_eq!(
        edge.binding.as_ref().map(|binding| binding.text.as_str()),
        Some("e")
    );
    assert_eq!(
        edge.labels
            .iter()
            .map(|label| label.text.as_str())
            .collect::<Vec<_>>(),
        ["KNOWS"]
    );
    assert_eq!(
        edge.properties
            .iter()
            .map(|property| property.key.text.as_str())
            .collect::<Vec<_>>(),
        ["since", "weight"]
    );
    assert!(matches!(
        edge.properties[0].value,
        crate::ast::Expression::Integer(2020, _)
    ));
    assert!(matches!(
        edge.properties[1].value,
        crate::ast::Expression::Integer(3, _)
    ));
    assert_eq!(
        edge.properties
            .iter()
            .map(|property| &source[property.span.start as usize..property.span.end as usize])
            .collect::<Vec<_>>(),
        ["since: 2020", "weight: 3"]
    );

    assert!(
        result.analysis.diagnostics.is_empty(),
        "sema: {:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("edge-property IR");
    let Some(crate::ir::GraphPatternElement::Edge(edge)) = ir.matches[0].paths[0].elements.get(1)
    else {
        panic!("canonical edge pattern exists");
    };
    assert_eq!(edge.binding.as_deref(), Some("E"));
    assert_eq!(
        edge.properties
            .iter()
            .map(|property| property.key.as_str())
            .collect::<Vec<_>>(),
        ["SINCE", "WEIGHT"]
    );
    assert!(matches!(
        edge.properties[0].value,
        crate::ir::Expression::Integer(2020)
    ));
    assert!(matches!(
        edge.properties[1].value,
        crate::ir::Expression::Integer(3)
    ));
    assert!(
        matches!(ir.projection[0].expression, crate::ir::Expression::Binding(ref name) if name == "E")
    );
}

#[test]
fn duplicate_edge_pattern_property_is_typed_and_emits_no_ir() {
    let result = Compiler.compile(
        "duplicate-edge-property.gql",
        "MATCH (a)-[e:KNOWS {since: 2020, since: 2021}]->(b) RETURN e",
        &empty_catalog(),
    );

    assert!(
        result.parse.diagnostics.is_empty(),
        "parse: {:?}",
        result.parse.diagnostics
    );
    assert!(result.analysis.ir.is_none());
    assert_eq!(
        result
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-DUPLICATE-PATTERN-PROPERTY"]
    );
}

#[test]
fn union_vertical_slice_is_backend_independent() {
    let source = "MATCH (a) RETURN a UNION MATCH (b) RETURN b";
    let result = Compiler.compile("union.gql", source, &empty_catalog());
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse: {:?}",
        result.parse.diagnostics
    );
    assert!(
        result.analysis.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.analysis.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    let ir = result.analysis.ir.expect("UNION IR");
    assert_eq!(ir.set_operations.len(), 1);
    assert_eq!(ir.set_operations[0].right.projection.len(), 1);
}

#[test]
fn limit_vertical_slice_is_backend_independent() {
    let source = "MATCH (n) RETURN n LIMIT 10";
    let result = Compiler.compile("limit.gql", source, &empty_catalog());
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse: {:?}",
        result.parse.diagnostics
    );
    assert!(
        result.analysis.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.analysis.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(
        result.analysis.ir.expect("LIMIT IR").limit,
        Some(crate::ir::NonNegativeIntegerSpecification::Literal(10))
    );
}

#[test]
fn order_by_vertical_slice_is_backend_independent() {
    let source = "MATCH (n) RETURN n ORDER BY n DESC LIMIT 10";
    let result = Compiler.compile("order-by.gql", source, &empty_catalog());
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse: {:?}",
        result.parse.diagnostics
    );
    assert!(
        result.analysis.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.analysis.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    let ir = result.analysis.ir.expect("ORDER BY IR");
    assert_eq!(ir.order_by.len(), 1);
    assert_eq!(
        ir.order_by[0].direction,
        crate::ir::SortDirection::Descending
    );
}

#[test]
fn offset_vertical_slice_is_backend_independent() {
    let source = "MATCH (n) RETURN n ORDER BY n LIMIT 10 OFFSET 2";
    let result = Compiler.compile("offset.gql", source, &empty_catalog());
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse: {:?}",
        result.parse.diagnostics
    );
    assert!(
        result.analysis.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.analysis.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(
        result.analysis.ir.expect("OFFSET IR").offset,
        Some(crate::ir::NonNegativeIntegerSpecification::Literal(2))
    );
}

#[test]
fn property_expression_vertical_slice_reaches_canonical_ir() {
    let compiler = Compiler;
    let result = compiler.compile(
        "property.gql",
        "MATCH (n) WHERE n.name = TRUE RETURN n.name",
        &empty_catalog(),
    );

    assert!(result.parse.diagnostics.is_empty());
    assert!(
        result.analysis.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("property query IR");
    assert!(matches!(
        ir.filters.as_slice(),
        [crate::ir::Expression::Binary { left, right, .. }]
            if matches!(left.as_ref(), crate::ir::Expression::PropertyAccess { property, .. } if property == "NAME")
                && matches!(right.as_ref(), crate::ir::Expression::Boolean(true))
    ));
    assert!(matches!(
        ir.projection.as_slice(),
        [crate::ir::Projection {
            expression: crate::ir::Expression::PropertyAccess { property, .. },
            ..
        }] if property == "NAME"
    ));
}

#[test]
fn decimal_literal_vertical_slice_reaches_canonical_ir() {
    let compiler = Compiler;
    let result = compiler.compile(
        "decimal.gql",
        "MATCH (n) WHERE n.score = 1.25 RETURN n",
        &empty_catalog(),
    );

    assert!(result.parse.diagnostics.is_empty());
    assert!(
        result.analysis.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("decimal query IR");
    assert!(matches!(
        ir.filters.as_slice(),
        [crate::ir::Expression::Binary { right, .. }]
            if matches!(right.as_ref(), crate::ir::Expression::Decimal(value) if value == "1.25")
    ));
}

#[test]
fn named_path_vertical_slice_reaches_canonical_ir() {
    let compiler = Compiler;
    let result = compiler.compile(
        "named-path.gql",
        "MATCH p = (a)-[:CALLS]->(b) RETURN p",
        &empty_catalog(),
    );

    assert!(result.parse.diagnostics.is_empty());
    assert!(
        result.analysis.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("named path query IR");
    assert!(matches!(
        ir.matches[0].paths.as_slice(),
        [path] if path.binding.as_deref() == Some("P")
    ));
    assert!(matches!(
        ir.projection.as_slice(),
        [crate::ir::Projection {
            expression: crate::ir::Expression::Binding(name),
            ..
        }] if name == "P"
    ));
}

#[test]
fn bounded_path_quantifier_vertical_slice_reaches_canonical_ir() {
    let compiler = Compiler;
    let result = compiler.compile(
        "quantified-path.gql",
        "MATCH (a)-[:CALLS]->{1,3}(b) RETURN b",
        &empty_catalog(),
    );

    assert!(result.parse.diagnostics.is_empty());
    assert!(
        result.analysis.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("quantified path query IR");
    assert!(matches!(
        ir.matches[0].paths[0].elements.as_slice(),
        [
            crate::ir::GraphPatternElement::Node(_),
            crate::ir::GraphPatternElement::Edge(edge),
            crate::ir::GraphPatternElement::Node(_)
        ] if edge
            .quantifier
            .as_ref()
            .map(|quantifier| (quantifier.min, quantifier.max))
            == Some((1, Some(3)))
    ));
}

#[test]
fn optional_match_vertical_slice_reaches_canonical_ir() {
    let compiler = Compiler;
    let result = compiler.compile(
        "optional-match.gql",
        "MATCH (a) OPTIONAL MATCH (a)-[:CALLS]->(b) RETURN a, b",
        &empty_catalog(),
    );

    assert!(result.parse.diagnostics.is_empty());
    assert!(
        result.analysis.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("optional match query IR");
    assert_eq!(ir.optional_matches.len(), 1);
    assert_eq!(ir.optional_matches[0].graph_match.paths.len(), 1);
    assert!(matches!(
        ir.projection.as_slice(),
        [
            crate::ir::Projection {
                expression: crate::ir::Expression::Binding(first),
                ..
            },
            crate::ir::Projection {
                expression: crate::ir::Expression::Binding(second),
                ..
            }
        ] if first == "A" && second == "B"
    ));
}

#[test]
fn arithmetic_vertical_slice_reaches_canonical_ir_with_precedence() {
    let compiler = Compiler;
    let grammar = crate::syntax::grammar_projection_receipt();
    assert_eq!(grammar.schema, "mrr.gerbil-grammar-projection.v1");
    assert!(grammar.syntax_shapes.contains(&(
        "BinaryExpression",
        "node",
        &["left", "operator", "right"],
    )));
    let result = compiler.compile(
        "arithmetic.gql",
        "MATCH (n) RETURN 1 + 2 * 3",
        &empty_catalog(),
    );

    assert!(result.parse.diagnostics.is_empty());
    assert!(result.analysis.diagnostics.is_empty());
    assert_eq!(
        result.parse.tree.rowan_root().text().to_string(),
        "MATCH (n) RETURN 1 + 2 * 3"
    );
    let ir = result.analysis.ir.expect("arithmetic query IR");
    assert!(matches!(
        ir.projection.as_slice(),
        [crate::ir::Projection {
            expression: crate::ir::Expression::Binary {
                operator: crate::ir::BinaryOperator::Add,
                right,
                ..
            },
            ..
        }] if matches!(right.as_ref(), crate::ir::Expression::Binary {
            operator: crate::ir::BinaryOperator::Multiply,
            ..
        })
    ));
}

#[test]
fn division_and_modulo_vertical_slice_reaches_canonical_ir() {
    let compiler = Compiler;
    let result = compiler.compile(
        "multiplicative.gql",
        "MATCH (n) RETURN 8 / 2 % 3",
        &empty_catalog(),
    );

    assert!(result.parse.diagnostics.is_empty());
    assert!(result.analysis.diagnostics.is_empty());
    let ir = result.analysis.ir.expect("multiplicative query IR");
    assert!(matches!(
        ir.projection.as_slice(),
        [crate::ir::Projection {
            expression: crate::ir::Expression::Binary {
                operator: crate::ir::BinaryOperator::Modulo,
                left,
                ..
            },
            ..
        }] if matches!(left.as_ref(), crate::ir::Expression::Binary {
            operator: crate::ir::BinaryOperator::Divide,
            ..
        })
    ));
}

#[test]
fn list_value_vertical_slice_reaches_canonical_ir() {
    let compiler = Compiler;
    let result = compiler.compile(
        "list-value.gql",
        "MATCH (n) RETURN [1, 2, [3]]",
        &empty_catalog(),
    );

    assert!(result.parse.diagnostics.is_empty());
    assert!(result.analysis.diagnostics.is_empty());
    let ir = result.analysis.ir.expect("list value query IR");
    assert!(matches!(
        ir.projection.as_slice(),
        [crate::ir::Projection {
            expression: crate::ir::Expression::List(items),
            ..
        }] if items.len() == 3
    ));
}

#[test]
fn collection_subscript_vertical_slice_reaches_canonical_ir() {
    let compiler = Compiler;
    let result = compiler.compile(
        "list-subscript.gql",
        "MATCH (n) LET values = [1, 2] RETURN values[0]",
        &empty_catalog(),
    );

    assert!(result.parse.diagnostics.is_empty());
    assert!(result.analysis.diagnostics.is_empty());
    let ir = result.analysis.ir.expect("subscript query IR");
    assert!(matches!(
        ir.projection.as_slice(),
        [crate::ir::Projection {
            expression: crate::ir::Expression::Subscript { .. },
            ..
        }]
    ));
}

#[test]
fn collection_membership_vertical_slice_reaches_canonical_ir() {
    let compiler = Compiler;
    let result = compiler.compile(
        "list-membership.gql",
        "MATCH (n) RETURN 1 IN [1, 2]",
        &empty_catalog(),
    );

    assert!(result.parse.diagnostics.is_empty());
    assert!(result.analysis.diagnostics.is_empty());
    assert!(matches!(
        result
            .analysis
            .ir
            .expect("membership query IR")
            .projection
            .as_slice(),
        [crate::ir::Projection {
            expression: crate::ir::Expression::Binary {
                operator: crate::ir::BinaryOperator::In,
                ..
            },
            ..
        }]
    ));
}
