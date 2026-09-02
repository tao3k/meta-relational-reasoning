use crate::Compiler;
use crate::ast::{BinaryOperator as AstBinaryOperator, Expression as AstExpression};
use crate::ast::{PatternElement as AstPatternElement, QueryClause, Statement};
use crate::catalog::{Catalog, CatalogName};
use crate::ir::GraphPatternElement as IrPatternElement;
use crate::ir::{BinaryOperator as IrBinaryOperator, Expression as IrExpression};
use crate::syntax::{SyntaxElementKind, SyntaxKind, SyntaxNode};

fn empty_catalog() -> Catalog {
    Catalog::new(
        CatalogName("edge-pattern-contract".into()),
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

#[test]
fn inline_edge_where_survives_lossless_cst_ast_and_canonical_ir() {
    let source = "MATCH (a)-[e:KNOWS WHERE e.since >= 2020]->(b) RETURN e";
    let result = Compiler.compile("inline-edge-where.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse diagnostics: {:?}",
        result.parse.diagnostics
    );
    assert!(contains_node_kind(
        &result.parse.tree.root(),
        SyntaxKind::InlineWhereClause
    ));

    let Some(Statement::Query(query)) = &result.statement else {
        panic!("inline edge WHERE source must remain a query");
    };
    let Some(QueryClause::Match(match_clause)) = query.clauses.first() else {
        panic!("MATCH clause exists");
    };
    let Some(AstPatternElement::Edge(edge)) = match_clause.patterns[0].elements.get(1) else {
        panic!("edge pattern exists");
    };
    assert_eq!(
        edge.binding.as_ref().map(|binding| binding.text.as_str()),
        Some("e")
    );
    assert!(matches!(
        edge.predicate.as_ref(),
        Some(AstExpression::Binary {
            operator: AstBinaryOperator::GreaterThanOrEqual,
            left,
            right,
        }) if matches!(left.as_ref(), AstExpression::PropertyAccess { base, property }
            if matches!(base.as_ref(), AstExpression::Name(binding) if binding.text == "e")
                && property.text == "since")
            && matches!(right.as_ref(), AstExpression::Integer(2020, _))
    ));

    assert!(
        result.analysis.diagnostics.is_empty(),
        "semantic diagnostics: {:?}",
        result.analysis.diagnostics
    );
    let ir = result
        .analysis
        .ir
        .expect("canonical inline edge predicate IR");
    let Some(IrPatternElement::Edge(edge)) = ir
        .graphs
        .into_iter()
        .next()
        .expect("graph")
        .elements
        .into_iter()
        .nth(1)
    else {
        panic!("canonical edge pattern exists");
    };
    assert!(matches!(
        edge.predicate,
        Some(IrExpression::Binary {
            operator: IrBinaryOperator::GreaterThanOrEqual,
            left,
            right,
        }) if matches!(left.as_ref(), IrExpression::PropertyAccess { base, property }
            if matches!(base.as_ref(), IrExpression::Binding(binding) if binding == "E")
                && property == "SINCE")
            && matches!(right.as_ref(), IrExpression::Integer(2020))
    ));
}

#[test]
fn missing_inline_edge_where_predicate_is_typed_and_emits_no_ir() {
    let source = "MATCH (a)-[e WHERE]->(b) RETURN e";
    let result = Compiler.compile("missing-inline-edge-where.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(
        result
            .parse
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-PARSE-INLINE-WHERE-SYNTAX"]
    );
    let diagnostic = &result.parse.diagnostics[0];
    assert_eq!(
        &source[diagnostic.span.start as usize..diagnostic.span.end as usize],
        "]"
    );
    assert!(result.statement.is_none());
    assert!(result.analysis.ir.is_none());
}
