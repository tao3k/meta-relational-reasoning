use crate::Compiler;
use crate::ast::{ForOrdinalityKind as AstPositionKind, QueryClause, Statement};
use crate::catalog::{Catalog, CatalogName};
use crate::ir::{
    BinaryOperator as IrBinaryOperator, Expression as IrExpression,
    ForPositionKind as IrPositionKind,
};
use crate::syntax::{SyntaxElementKind, SyntaxKind, SyntaxNode};
use crate::types::ValueType;

fn empty_catalog() -> Catalog {
    Catalog::new(
        CatalogName("filter-for-contract".into()),
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
fn iso_filter_and_for_primitive_query_statements_reach_canonical_ir() {
    let source = "LET values = [1, 2, 3] FOR value IN values WITH ORDINALITY position FILTER value > 1 RETURN value, position";
    let result = Compiler.compile("filter-for.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "FILTER/FOR must be accepted by the ISO grammar: {:?}",
        result.parse.diagnostics
    );
    let root = result.parse.tree.root();
    assert_eq!(count_kind(&root, SyntaxKind::ForStatement), 1);
    assert_eq!(count_kind(&root, SyntaxKind::ForItem), 1);
    assert_eq!(count_kind(&root, SyntaxKind::ForOrdinalityOrOffset), 1);
    assert_eq!(count_kind(&root, SyntaxKind::FilterStatement), 1);

    let Some(Statement::Query(query)) = &result.statement else {
        panic!("FILTER/FOR source must lower to one typed query");
    };
    let Some(QueryClause::For { item, .. }) = query
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::For { .. }))
    else {
        panic!("FOR statement must be retained in the AST");
    };
    assert_eq!(item.binding.text, "value");
    assert_eq!(
        &source[item.binding.span.start as usize..item.binding.span.end as usize],
        "value"
    );
    let position = item
        .ordinality
        .as_ref()
        .expect("WITH ORDINALITY binding must be retained");
    assert_eq!(position.kind, AstPositionKind::Ordinality);
    assert_eq!(position.binding.text, "position");
    assert!(item.span.end <= position.span.end);
    assert!(matches!(
        query
            .clauses
            .iter()
            .find(|clause| matches!(clause, QueryClause::Filter { .. })),
        Some(QueryClause::Filter { .. })
    ));

    assert!(
        result.analysis.diagnostics.is_empty(),
        "FILTER/FOR must be admitted semantically: {:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("canonical IR must be complete");
    assert_eq!(ir.for_bindings.len(), 1);
    assert_eq!(ir.for_bindings[0].binding.name, "VALUE");
    assert_eq!(ir.for_bindings[0].binding.value_type, ValueType::Any);
    assert_eq!(
        ir.for_bindings[0].source,
        IrExpression::Binding("VALUES".into())
    );
    let position = ir.for_bindings[0]
        .position
        .as_ref()
        .expect("canonical position binding");
    assert_eq!(position.kind, IrPositionKind::Ordinality);
    assert_eq!(position.binding.name, "POSITION");
    assert_eq!(position.binding.value_type, ValueType::Integer);
    assert!(matches!(
        ir.filters.as_slice(),
        [IrExpression::Binary {
            operator: IrBinaryOperator::GreaterThan,
            left,
            right,
        }] if **left == IrExpression::Binding("VALUE".into())
            && **right == IrExpression::Integer(1)
    ));
}

#[test]
fn malformed_filter_and_for_emit_exactly_one_typed_terminal() {
    let cases = [
        ("FILTER RETURN 1", "GQL-PARSE-FILTER-SYNTAX"),
        ("FOR value values RETURN value", "GQL-PARSE-FOR-SYNTAX"),
        (
            "FOR value IN [1] WITH ORDINALITY RETURN value",
            "GQL-PARSE-FOR-SYNTAX",
        ),
    ];

    for (source, code) in cases {
        let result = Compiler.compile("malformed-filter-for.gql", source, &empty_catalog());
        assert_eq!(
            result
                .parse
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect::<Vec<_>>(),
            [code],
            "source={source}; diagnostics={:?}",
            result.parse.diagnostics
        );
        assert!(result.statement.is_none(), "source={source}");
        assert!(result.analysis.ir.is_none(), "source={source}");
    }
}

#[test]
fn for_offset_spelling_and_semantic_failures_are_typed_and_fail_closed() {
    let source = "FOR value IN [1] WITH OFFSET position RETURN value, position";
    let result = Compiler.compile("for-offset.gql", source, &empty_catalog());
    assert!(result.parse.diagnostics.is_empty());
    assert!(result.analysis.diagnostics.is_empty());
    let ir = result.analysis.ir.expect("FOR WITH OFFSET canonical IR");
    let position = ir.for_bindings[0]
        .position
        .as_ref()
        .expect("WITH OFFSET position binding");
    assert_eq!(position.kind, IrPositionKind::Offset);
    assert_eq!(position.binding.name, "POSITION");

    let cases = [
        ("FILTER 1 RETURN 1", "GQL-SEMA-FILTER-NOT-BOOLEAN"),
        (
            "FOR value IN 1 RETURN value",
            "GQL-SEMA-FOR-SOURCE-NOT-LIST",
        ),
    ];
    for (source, code) in cases {
        let result = Compiler.compile("invalid-filter-for-semantics.gql", source, &empty_catalog());
        assert!(result.parse.diagnostics.is_empty(), "source={source}");
        assert_eq!(
            result
                .analysis
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect::<Vec<_>>(),
            [code],
            "source={source}; diagnostics={:?}",
            result.analysis.diagnostics
        );
        assert!(result.analysis.ir.is_none(), "source={source}");
    }
}
