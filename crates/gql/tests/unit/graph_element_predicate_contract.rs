use crate::Compiler;
use crate::ast::{
    ElementIdentityKind as AstIdentityKind, EndpointKind as AstEndpointKind,
    Expression as AstExpression, QueryClause, Statement,
};
use crate::catalog::{Catalog, CatalogName};
use crate::ir::{
    ElementIdentityKind as IrIdentityKind, EndpointKind as IrEndpointKind,
    Expression as IrExpression,
};
use crate::syntax::{SyntaxElementKind, SyntaxKind, SyntaxNode};

fn empty_catalog() -> Catalog {
    Catalog::new(
        CatalogName("graph-element-predicate-contract".into()),
        Vec::new(),
        Vec::new(),
    )
}

fn count_kind(node: &SyntaxNode, expected: SyntaxKind) -> usize {
    usize::from(node.kind() == expected)
        + node
            .children()
            .into_iter()
            .map(|element| match element.kind {
                SyntaxElementKind::Node(child) => count_kind(&child, expected),
                SyntaxElementKind::Token(_) => 0,
            })
            .sum::<usize>()
}

#[test]
fn iso_graph_element_predicate_family_crosses_lossless_cst_ast_and_canonical_ir() {
    let source = concat!(
        "MATCH (a)-[e]->(b), (c)-[f]->(d) RETURN ",
        "e IS DIRECTED, e IS NOT DIRECTED, ",
        "a IS SOURCE OF e, b IS NOT DESTINATION OF e, ",
        "ALL_DIFFERENT(a, b, c), SAME(a, a), PROPERTY_EXISTS(a, name)"
    );
    let result = Compiler.compile("iso-graph-element-predicates.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse: {:?}",
        result.parse.diagnostics
    );
    assert!(
        result.analysis.diagnostics.is_empty(),
        "sema: {:?}",
        result.analysis.diagnostics
    );
    assert_eq!(
        count_kind(
            &result.parse.tree.root(),
            SyntaxKind::DirectedPredicateExpression
        ),
        2
    );
    assert_eq!(
        count_kind(
            &result.parse.tree.root(),
            SyntaxKind::EndpointPredicateExpression
        ),
        2
    );
    assert_eq!(
        count_kind(
            &result.parse.tree.root(),
            SyntaxKind::ElementIdentityPredicateExpression
        ),
        2
    );
    assert_eq!(
        count_kind(
            &result.parse.tree.root(),
            SyntaxKind::PropertyExistsPredicateExpression
        ),
        1
    );

    let Some(Statement::Query(query)) = &result.statement else {
        panic!("graph-element predicates must lower to a query AST");
    };
    let Some(QueryClause::Return { projections, .. }) = query.clauses.last() else {
        panic!("RETURN clause must be retained");
    };
    assert_eq!(projections.len(), 7);
    assert!(matches!(
        &projections[0].expression,
        AstExpression::DirectedPredicate {
            edge,
            negated: false,
            span,
        } if matches!(edge.as_ref(), AstExpression::Name(name) if name.text == "e")
            && &source[span.start as usize..span.end as usize] == "e IS DIRECTED"
    ));
    assert!(matches!(
        &projections[1].expression,
        AstExpression::DirectedPredicate { negated: true, .. }
    ));
    assert!(matches!(
        &projections[2].expression,
        AstExpression::EndpointPredicate {
            endpoint: AstEndpointKind::Source,
            negated: false,
            node,
            edge,
            ..
        } if matches!(node.as_ref(), AstExpression::Name(name) if name.text == "a")
            && matches!(edge.as_ref(), AstExpression::Name(name) if name.text == "e")
    ));
    assert!(matches!(
        &projections[3].expression,
        AstExpression::EndpointPredicate {
            endpoint: AstEndpointKind::Destination,
            negated: true,
            ..
        }
    ));
    assert!(matches!(
        &projections[4].expression,
        AstExpression::ElementIdentityPredicate {
            kind: AstIdentityKind::AllDifferent,
            elements,
            ..
        } if elements.iter().map(|element| match element {
            AstExpression::Name(name) => name.text.as_str(),
            other => panic!("expected element name, got {other:?}"),
        }).collect::<Vec<_>>() == ["a", "b", "c"]
    ));
    assert!(matches!(
        &projections[5].expression,
        AstExpression::ElementIdentityPredicate {
            kind: AstIdentityKind::Same,
            elements,
            ..
        } if elements.len() == 2
    ));
    assert!(matches!(
        &projections[6].expression,
        AstExpression::PropertyExistsPredicate { element, property, .. }
            if matches!(element.as_ref(), AstExpression::Name(name) if name.text == "a")
                && property.text == "name"
    ));

    let ir = result.analysis.ir.expect("canonical IR must be complete");
    assert_eq!(ir.projection.len(), 7);
    assert!(matches!(
        &ir.projection[0].expression,
        IrExpression::IsDirected {
            edge,
            negated: false,
        } if matches!(edge.as_ref(), IrExpression::Binding(name) if name == "E")
    ));
    assert!(matches!(
        &ir.projection[2].expression,
        IrExpression::IsEndpoint {
            endpoint: IrEndpointKind::Source,
            negated: false,
            ..
        }
    ));
    assert!(matches!(
        &ir.projection[3].expression,
        IrExpression::IsEndpoint {
            endpoint: IrEndpointKind::Destination,
            negated: true,
            ..
        }
    ));
    assert!(matches!(
        &ir.projection[4].expression,
        IrExpression::ElementIdentity {
            kind: IrIdentityKind::AllDifferent,
            elements,
        } if elements == &[
            IrExpression::Binding("A".into()),
            IrExpression::Binding("B".into()),
            IrExpression::Binding("C".into()),
        ]
    ));
    assert!(matches!(
        &ir.projection[5].expression,
        IrExpression::ElementIdentity {
            kind: IrIdentityKind::Same,
            ..
        }
    ));
    assert!(matches!(
        &ir.projection[6].expression,
        IrExpression::PropertyExists { property, .. } if property == "NAME"
    ));
}

#[test]
fn malformed_graph_element_predicates_emit_exactly_one_terminal_and_no_ir() {
    for (file, source) in [
        (
            "all-different-arity.gql",
            "MATCH (a) RETURN ALL_DIFFERENT(a)",
        ),
        ("same-arity.gql", "MATCH (a) RETURN SAME(a)"),
        (
            "property-exists-arity.gql",
            "MATCH (a) RETURN PROPERTY_EXISTS(a, name, extra)",
        ),
        (
            "source-missing-of.gql",
            "MATCH (a)-[e]->(b) RETURN a IS SOURCE e",
        ),
    ] {
        let result = Compiler.compile(file, source, &empty_catalog());
        assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
        assert_eq!(
            result
                .parse
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect::<Vec<_>>(),
            ["GQL-PARSE-GRAPH-ELEMENT-PREDICATE-SYNTAX"],
            "{file}: {:?}",
            result.parse.diagnostics
        );
        assert!(result.statement.is_none(), "{file}");
        assert!(result.analysis.ir.is_none(), "{file}");
    }
}

#[test]
fn graph_element_predicate_kind_mismatch_is_typed_and_emits_no_ir() {
    for (file, source, code) in [
        (
            "directed-node.gql",
            "MATCH (a)-[e]->(b) RETURN a IS DIRECTED",
            "GQL-SEMA-DIRECTED-PREDICATE-NON-EDGE",
        ),
        (
            "source-kind.gql",
            "MATCH (a)-[e]->(b) RETURN e IS SOURCE OF a",
            "GQL-SEMA-ENDPOINT-PREDICATE-KIND",
        ),
    ] {
        let result = Compiler.compile(file, source, &empty_catalog());
        assert!(result.parse.diagnostics.is_empty(), "{file}");
        assert_eq!(
            result
                .analysis
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect::<Vec<_>>(),
            [code],
            "{file}: {:?}",
            result.analysis.diagnostics
        );
        assert!(result.analysis.ir.is_none(), "{file}");
    }
}
