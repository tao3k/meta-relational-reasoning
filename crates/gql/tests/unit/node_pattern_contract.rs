use crate::Compiler;
use crate::ast::{BinaryOperator as AstBinaryOperator, Expression as AstExpression};
use crate::ast::{PatternElement as AstPatternElement, QueryClause, Statement};
use crate::catalog::{Catalog, CatalogName};
use crate::ir::GraphPatternElement as IrPatternElement;
use crate::ir::{BinaryOperator as IrBinaryOperator, Expression as IrExpression};
use crate::syntax::{SyntaxElementKind, SyntaxKind, SyntaxNode};

fn empty_catalog() -> Catalog {
    Catalog::new(
        CatalogName("node-pattern-contract".into()),
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
fn inline_node_where_survives_lossless_cst_ast_and_canonical_ir() {
    let source = "MATCH (n:Person WHERE n.age >= 18) RETURN n";
    let result = Compiler.compile("inline-node-where.gql", source, &empty_catalog());

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
        panic!("inline node WHERE source must remain a query");
    };
    let Some(QueryClause::Match(match_clause)) = query.clauses.first() else {
        panic!("MATCH clause exists");
    };
    let Some(AstPatternElement::Node(node)) = match_clause.patterns[0].elements.first() else {
        panic!("node pattern exists");
    };
    assert_eq!(
        node.binding.as_ref().map(|binding| binding.text.as_str()),
        Some("n")
    );
    assert_eq!(
        node.labels
            .iter()
            .map(|label| label.text.as_str())
            .collect::<Vec<_>>(),
        ["Person"]
    );
    assert!(matches!(
        node.predicate.as_ref(),
        Some(AstExpression::Binary {
            operator: AstBinaryOperator::GreaterThanOrEqual,
            left,
            right,
        }) if matches!(left.as_ref(), AstExpression::PropertyAccess { base, property }
            if matches!(base.as_ref(), AstExpression::Name(binding) if binding.text == "n")
                && property.text == "age")
            && matches!(right.as_ref(), AstExpression::Integer(18, _))
    ));

    assert!(
        result.analysis.diagnostics.is_empty(),
        "semantic diagnostics: {:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("canonical inline predicate IR");
    let Some(IrPatternElement::Node(node)) = ir
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
    assert!(matches!(
        node.predicate,
        Some(IrExpression::Binary {
            operator: IrBinaryOperator::GreaterThanOrEqual,
            left,
            right,
        }) if matches!(left.as_ref(), IrExpression::PropertyAccess { base, property }
            if matches!(base.as_ref(), IrExpression::Binding(binding) if binding == "N")
                && property == "AGE")
            && matches!(right.as_ref(), IrExpression::Integer(18))
    ));
}

#[test]
fn missing_inline_node_where_predicate_is_typed_and_emits_no_ir() {
    let source = "MATCH (n WHERE) RETURN n";
    let result = Compiler.compile("missing-inline-node-where.gql", source, &empty_catalog());

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
        ")"
    );
    assert!(result.analysis.ir.is_none());
}
