use crate::Compiler;
use crate::ast::{PatternElement as AstPatternElement, QueryClause, Statement};
use crate::catalog::{Catalog, CatalogName};
use crate::ir::{GraphPatternElement as IrPatternElement, PathMode as IrPathMode};
use crate::syntax::{SyntaxElementKind, SyntaxKind, SyntaxNode};

fn empty_catalog() -> Catalog {
    Catalog::new(CatalogName("match-contract".into()), Vec::new(), Vec::new())
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
fn comma_separated_match_patterns_preserve_components_and_shared_bindings() {
    let source =
        "MATCH TRAIL (a:Person)-[:KNOWS]->(b), (b)-[:WORKS_AT]->(c:Company) RETURN a, b, c";
    let result = Compiler.compile("match-pattern-list.gql", source, &empty_catalog());

    assert!(
        result.parse.diagnostics.is_empty(),
        "{:?}",
        result.parse.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    let root = result.parse.tree.root();
    assert_eq!(count_kind(&root, SyntaxKind::GraphPatternList), 1);
    assert_eq!(count_kind(&root, SyntaxKind::GraphPattern), 2);

    let Some(Statement::Query(query)) = &result.statement else {
        panic!("query statement");
    };
    let Some(QueryClause::Match(matched)) = query.clauses.first() else {
        panic!("MATCH clause");
    };
    assert_eq!(matched.patterns.len(), 2);
    assert!(matches!(
        matched.patterns[0].elements.first(),
        Some(AstPatternElement::Node(_))
    ));
    assert!(
        matches!(matched.patterns[1].elements.first(), Some(AstPatternElement::Node(node))
        if node.binding.as_ref().is_some_and(|binding| binding.text == "b"))
    );

    assert!(
        result.analysis.diagnostics.is_empty(),
        "{:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("canonical MATCH IR");
    assert_eq!(ir.graphs.len(), 2);
    assert!(
        ir.graphs
            .iter()
            .all(|graph| graph.mode == IrPathMode::Trail)
    );
    assert!(
        matches!(ir.graphs[1].elements.first(), Some(IrPatternElement::Node(node))
        if node.binding.as_deref() == Some("B"))
    );
}

#[test]
fn successive_match_clauses_extend_the_canonical_graph_sequence() {
    let source = "MATCH (a) MATCH (b) RETURN a, b";
    let result = Compiler.compile("successive-match.gql", source, &empty_catalog());

    assert!(
        result.parse.diagnostics.is_empty(),
        "{:?}",
        result.parse.diagnostics
    );
    assert!(
        result.analysis.diagnostics.is_empty(),
        "{:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("canonical MATCH IR");
    assert_eq!(ir.graphs.len(), 2);
    assert!(
        matches!(ir.graphs[0].elements.as_slice(), [IrPatternElement::Node(node)]
        if node.binding.as_deref() == Some("A"))
    );
    assert!(
        matches!(ir.graphs[1].elements.as_slice(), [IrPatternElement::Node(node)]
        if node.binding.as_deref() == Some("B"))
    );
}

#[test]
fn trailing_match_pattern_separator_is_exactly_once_typed_and_emits_no_ir() {
    let source = "MATCH (a), RETURN a";
    let result = Compiler.compile("trailing-match-pattern.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(result.statement.is_none());
    assert!(result.analysis.ir.is_none());
    assert_eq!(
        result
            .parse
            .diagnostics
            .iter()
            .map(|item| item.code)
            .collect::<Vec<_>>(),
        ["GQL-PARSE-MATCH-PATTERN-LIST"]
    );
}

#[test]
fn match_binding_kind_conflict_is_typed_and_emits_no_ir() {
    let source = "MATCH (x), ()-[x]->() RETURN x";
    let result = Compiler.compile("match-binding-kind-conflict.gql", source, &empty_catalog());

    assert!(
        result.parse.diagnostics.is_empty(),
        "{:?}",
        result.parse.diagnostics
    );
    assert!(result.analysis.ir.is_none());
    assert_eq!(
        result
            .analysis
            .diagnostics
            .iter()
            .map(|item| item.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-BINDING-KIND-CONFLICT"]
    );
}
