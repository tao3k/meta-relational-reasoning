use crate::Compiler;
use crate::ast::{PatternElement as AstPatternElement, QueryClause, Statement};
use crate::catalog::{Catalog, CatalogName};
use crate::ir::GraphPatternElement as IrPatternElement;
use crate::syntax::{SyntaxElementKind, SyntaxKind, SyntaxNode};

fn empty_catalog() -> Catalog {
    Catalog::new(
        CatalogName("quantified-path-contract".into()),
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
fn every_quantifier_form_reaches_the_same_canonical_bounds() {
    let cases = [
        ("*", 0, None),
        ("+", 1, None),
        ("?", 0, Some(1)),
        ("{0}", 0, Some(0)),
        ("{2}", 2, Some(2)),
        ("{2,}", 2, None),
        ("{,4}", 0, Some(4)),
        ("{2,4}", 2, Some(4)),
    ];

    for (spelling, expected_min, expected_max) in cases {
        let source = format!("MATCH TRAIL (a)-[:KNOWS]->{spelling}(b) RETURN a");
        let result = Compiler.compile("quantified-path.gql", &source, &empty_catalog());
        assert!(
            result.parse.diagnostics.is_empty(),
            "{spelling}: {:?}",
            result.parse.diagnostics
        );
        assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
        assert!(contains_node_kind(
            &result.parse.tree.root(),
            SyntaxKind::PathQuantifier
        ));

        let Some(Statement::Query(query)) = &result.statement else {
            panic!("quantified path must remain a query: {spelling}");
        };
        let Some(QueryClause::Match(match_clause)) = query.clauses.first() else {
            panic!("MATCH clause exists");
        };
        let Some(AstPatternElement::Edge(edge)) = match_clause.patterns[0].elements.get(1) else {
            panic!("edge exists");
        };
        let quantifier = edge.quantifier.as_ref().expect("AST quantifier");
        assert_eq!(
            (quantifier.min, quantifier.max),
            (expected_min, expected_max)
        );
        assert_eq!(
            &source[quantifier.span.start as usize..quantifier.span.end as usize],
            spelling
        );

        let ir = result.analysis.ir.expect("canonical quantified-path IR");
        let Some(IrPatternElement::Edge(edge)) = ir
            .graphs
            .into_iter()
            .next()
            .expect("graph")
            .elements
            .into_iter()
            .nth(1)
        else {
            panic!("canonical edge exists");
        };
        let quantifier = edge.quantifier.expect("IR quantifier");
        assert_eq!(
            (quantifier.min, quantifier.max),
            (expected_min, expected_max)
        );
    }
}

#[test]
fn malformed_or_reversed_quantifier_is_typed_and_emits_no_ir() {
    for source in [
        "MATCH (a)-[:KNOWS]->{,}(b) RETURN a",
        "MATCH (a)-[:KNOWS]->{4,2}(b) RETURN a",
    ] {
        let result = Compiler.compile("invalid-quantifier.gql", source, &empty_catalog());
        assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
        assert_eq!(
            result
                .parse
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect::<Vec<_>>(),
            ["GQL-PARSE-PATH-QUANTIFIER"]
        );
        assert!(result.statement.is_none());
        assert!(result.analysis.ir.is_none());
    }
}
