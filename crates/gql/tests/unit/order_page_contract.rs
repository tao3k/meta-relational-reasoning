use crate::Compiler;
use crate::ast::{
    NonNegativeIntegerSpecification as AstIntegerSpecification, NullOrdering as AstNullOrdering,
    QueryClause, SortDirection as AstSortDirection, Statement,
};
use crate::catalog::{Catalog, CatalogName};
use crate::ir::{
    NonNegativeIntegerSpecification as IrIntegerSpecification, NullOrdering as IrNullOrdering,
    SortDirection as IrSortDirection,
};
use crate::syntax::{SyntaxElementKind, SyntaxKind, SyntaxNode};

fn empty_catalog() -> Catalog {
    Catalog::new(
        CatalogName("order-page-contract".into()),
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
fn iso_ordering_and_pagination_family_crosses_frontend_admission() {
    let source = "MATCH (n) RETURN n.name AS name, n.score AS score ORDER BY score ASCENDING NULLS LAST, name DESCENDING NULLS FIRST SKIP $offset LIMIT 0";
    let result = Compiler.compile("iso-order-page.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(result.parse.diagnostics.is_empty());
    let root = result.parse.tree.root();
    assert_eq!(count_kind(&root, SyntaxKind::SortSpecification), 2);
    assert_eq!(count_kind(&root, SyntaxKind::OrderingSpecification), 2);
    assert_eq!(count_kind(&root, SyntaxKind::NullOrdering), 2);
    assert_eq!(
        count_kind(&root, SyntaxKind::NonNegativeIntegerSpecification),
        2
    );

    let Some(Statement::Query(query)) = &result.statement else {
        panic!("ordering and pagination source must lower to a query");
    };
    let Some(QueryClause::OrderBy { keys, .. }) = query
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::OrderBy { .. }))
    else {
        panic!("ORDER BY clause must be typed");
    };
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0].direction, Some(AstSortDirection::Ascending));
    assert_eq!(keys[0].null_ordering, Some(AstNullOrdering::Last));
    assert_eq!(keys[1].direction, Some(AstSortDirection::Descending));
    assert_eq!(keys[1].null_ordering, Some(AstNullOrdering::First));
    assert!(matches!(
        query
            .clauses
            .iter()
            .find(|clause| matches!(clause, QueryClause::Offset { .. })),
        Some(QueryClause::Offset {
            value: AstIntegerSpecification::Parameter(parameter),
            ..
        }) if parameter.name == "offset"
    ));
    assert!(matches!(
        query
            .clauses
            .iter()
            .find(|clause| matches!(clause, QueryClause::Limit { .. })),
        Some(QueryClause::Limit {
            value: AstIntegerSpecification::Literal(0),
            ..
        })
    ));

    assert!(result.analysis.diagnostics.is_empty());
    let ir = result
        .analysis
        .ir
        .expect("canonical ordering and pagination IR");
    assert_eq!(ir.order_by.len(), 2);
    assert_eq!(ir.order_by[0].direction, IrSortDirection::Ascending);
    assert_eq!(ir.order_by[0].null_ordering, Some(IrNullOrdering::Last));
    assert_eq!(ir.order_by[1].direction, IrSortDirection::Descending);
    assert_eq!(ir.order_by[1].null_ordering, Some(IrNullOrdering::First));
    assert_eq!(
        ir.offset,
        Some(IrIntegerSpecification::Parameter("offset".into()))
    );
    assert_eq!(ir.limit, Some(IrIntegerSpecification::Literal(0)));
}

#[test]
fn offset_synonyms_dynamic_parameters_and_default_ordering_are_canonical() {
    let sources = [
        (
            "MATCH (n) RETURN n ORDER BY n OFFSET $42 LIMIT $limit",
            "42",
            "limit",
        ),
        (
            "MATCH (n) RETURN n ORDER BY n SKIP $offset LIMIT $limit",
            "offset",
            "limit",
        ),
    ];

    for (source, offset, limit) in sources {
        let result = Compiler.compile("dynamic-page.gql", source, &empty_catalog());
        assert!(result.parse.diagnostics.is_empty(), "{source}");
        assert!(result.analysis.diagnostics.is_empty(), "{source}");
        let ir = result.analysis.ir.expect("dynamic pagination IR");
        assert_eq!(ir.order_by[0].direction, IrSortDirection::Ascending);
        assert_eq!(ir.order_by[0].null_ordering, None);
        assert_eq!(
            ir.offset,
            Some(IrIntegerSpecification::Parameter(offset.into()))
        );
        assert_eq!(
            ir.limit,
            Some(IrIntegerSpecification::Parameter(limit.into()))
        );
    }

    let source = "MATCH (first) RETURN first ORDER BY first LIMIT 1";
    let result = Compiler.compile("default-order-identifier.gql", source, &empty_catalog());
    let Some(Statement::Query(query)) = &result.statement else {
        panic!("default ordering source must lower to a query");
    };
    let Some(QueryClause::OrderBy { keys, .. }) = query
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::OrderBy { .. }))
    else {
        panic!("ORDER BY clause must be typed");
    };
    assert_eq!(keys[0].direction, None);
    assert_eq!(keys[0].null_ordering, None);
    assert_eq!(
        result.analysis.ir.expect("default ordering IR").order_by[0].direction,
        IrSortDirection::Ascending
    );
}

#[test]
fn malformed_ordering_and_pagination_emit_exactly_one_typed_terminal() {
    let cases = [
        (
            "MATCH (n) RETURN n ORDER BY n NULLS MIDDLE",
            "GQL-PARSE-ORDER-BY-SYNTAX",
        ),
        (
            "MATCH (n) RETURN n ORDER BY n ASC DESC",
            "GQL-PARSE-ORDER-BY-SYNTAX",
        ),
        (
            "MATCH (n) RETURN n ORDER BY n, LIMIT 1",
            "GQL-PARSE-ORDER-BY-SYNTAX",
        ),
        ("MATCH (n) RETURN n LIMIT -1", "GQL-PARSE-LIMIT-SYNTAX"),
        ("MATCH (n) RETURN n LIMIT 1.5", "GQL-PARSE-LIMIT-SYNTAX"),
        (
            "MATCH (n) RETURN n LIMIT $$catalog",
            "GQL-PARSE-LIMIT-SYNTAX",
        ),
        ("MATCH (n) RETURN n OFFSET", "GQL-PARSE-OFFSET-SYNTAX"),
        ("MATCH (n) RETURN n SKIP nope", "GQL-PARSE-OFFSET-SYNTAX"),
    ];

    for (source, code) in cases {
        let result = Compiler.compile("malformed-order-page.gql", source, &empty_catalog());
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
