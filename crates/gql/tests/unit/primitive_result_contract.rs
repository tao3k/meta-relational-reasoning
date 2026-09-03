use crate::Compiler;
use crate::ast::{QueryClause, SetQuantifier as AstSetQuantifier, Statement};
use crate::catalog::{Catalog, CatalogName};
use crate::ir::{Expression as IrExpression, SetQuantifier as IrSetQuantifier};
use crate::syntax::{SyntaxElementKind, SyntaxKind, SyntaxNode};

fn empty_catalog() -> Catalog {
    Catalog::new(
        CatalogName("primitive-result-contract".into()),
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
fn iso_return_set_quantifier_and_asterisk_reach_canonical_ir_in_binding_order() {
    let source = "MATCH (a)-[e]->(b) RETURN DISTINCT *";
    let result = Compiler.compile("return-distinct-star.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "{:?}",
        result.parse.diagnostics
    );
    let root = result.parse.tree.root();
    assert_eq!(count_kind(&root, SyntaxKind::ReturnClause), 1);
    assert_eq!(count_kind(&root, SyntaxKind::SetQuantifier), 1);

    let Some(Statement::Query(query)) = &result.statement else {
        panic!("RETURN DISTINCT * must lower to a typed query");
    };
    let Some(QueryClause::Return {
        quantifier,
        all_bindings,
        projections,
        ..
    }) = query.clauses.last()
    else {
        panic!("typed RETURN clause");
    };
    assert_eq!(*quantifier, Some(AstSetQuantifier::Distinct));
    assert!(*all_bindings);
    assert!(projections.is_empty());

    assert!(
        result.analysis.diagnostics.is_empty(),
        "{:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("canonical result IR");
    assert_eq!(ir.projection_quantifier, IrSetQuantifier::Distinct);
    assert_eq!(
        ir.projection
            .iter()
            .map(|projection| &projection.expression)
            .collect::<Vec<_>>(),
        [
            &IrExpression::Binding("A".into()),
            &IrExpression::Binding("E".into()),
            &IrExpression::Binding("B".into()),
        ]
    );
    assert!(!ir.is_finish);
}

#[test]
fn iso_finish_is_a_typed_projection_free_result_terminal() {
    let source = "MATCH (n) FINISH";
    let result = Compiler.compile("finish.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "{:?}",
        result.parse.diagnostics
    );
    assert_eq!(
        count_kind(&result.parse.tree.root(), SyntaxKind::FinishStatement),
        1
    );
    let Some(Statement::Query(query)) = &result.statement else {
        panic!("FINISH must lower to a typed query");
    };
    assert!(matches!(
        query.clauses.last(),
        Some(QueryClause::Finish { .. })
    ));
    assert!(
        result.analysis.diagnostics.is_empty(),
        "{:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("canonical FINISH IR");
    assert!(ir.is_finish);
    assert!(ir.projection.is_empty());
}

#[test]
fn explicit_all_and_distinct_result_quantifiers_are_canonical() {
    for (source, expected) in [
        ("MATCH (n) RETURN ALL n", IrSetQuantifier::All),
        ("MATCH (n) RETURN DISTINCT n", IrSetQuantifier::Distinct),
    ] {
        let result = Compiler.compile("return-quantifier.gql", source, &empty_catalog());
        assert!(result.parse.diagnostics.is_empty(), "source={source}");
        assert!(result.analysis.diagnostics.is_empty(), "source={source}");
        assert_eq!(
            result
                .analysis
                .ir
                .expect("canonical result IR")
                .projection_quantifier,
            expected,
            "source={source}"
        );
    }
}

#[test]
fn return_star_without_visible_bindings_fails_closed() {
    let result = Compiler.compile("return-star-empty.gql", "RETURN *", &empty_catalog());
    assert!(result.parse.diagnostics.is_empty());
    assert_eq!(
        result
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-RETURN-STAR-WITHOUT-BINDINGS"]
    );
    assert!(result.analysis.ir.is_none());
}

#[test]
fn malformed_primitive_result_statements_emit_exactly_one_typed_terminal() {
    let cases = [
        ("MATCH (n) RETURN DISTINCT", "GQL-PARSE-RETURN-SYNTAX"),
        ("MATCH (n) FINISH n", "GQL-PARSE-FINISH-SYNTAX"),
        ("MATCH (n) RETURN * n", "GQL-PARSE-RETURN-SYNTAX"),
    ];

    for (source, code) in cases {
        let result = Compiler.compile("malformed-result.gql", source, &empty_catalog());
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
