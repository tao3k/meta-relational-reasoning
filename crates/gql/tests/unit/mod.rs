use crate::catalog::{Catalog, CatalogName};
use crate::syntax::TokenKind;

fn catalog() -> Catalog {
    Catalog::new(CatalogName("test-catalog".into()), Vec::new(), Vec::new())
}

#[test]
fn backend_feature_does_not_change_iso_parse_surface() {
    let source = "MATCH (a)-[:CALLS]->(b) RETURN a, b";
    let parsed = crate::syntax::parse("feature-invariance.gql", source);

    assert!(parsed.diagnostics.is_empty());
    assert_eq!(parsed.tree.rowan_root().text().to_string(), source);
}

#[test]
fn ascent_is_not_a_parser_keyword() {
    let parsed = crate::syntax::parse("purity.gql", "MATCH (a) RETURN ascent");

    assert!(parsed.diagnostics.is_empty());
    assert!(
        parsed
            .tree
            .tokens()
            .iter()
            .any(|token| token.kind == TokenKind::Identifier)
    );
}

#[test]
fn node_only_vertical_slice_is_backend_independent() {
    let source = "MATCH (n) RETURN n";
    let compiler = crate::Compiler;
    let result = compiler.compile("node-only.gql", source, &catalog());

    assert!(result.parse.diagnostics.is_empty());
    assert!(
        result.analysis.diagnostics.is_empty(),
        "sema: {:?}",
        result.analysis.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(
        result
            .analysis
            .ir
            .expect("IR")
            .graph
            .expect("graph")
            .elements
            .len(),
        1
    );
}

#[test]
fn graph_filter_vertical_slice_is_backend_independent() {
    let source = "MATCH (a)-[:CALLS]->(b) WHERE a = 1 RETURN b";
    let compiler = crate::Compiler;
    let result = compiler.compile("graph-filter.gql", source, &catalog());

    assert!(result.parse.diagnostics.is_empty());
    assert!(
        result.analysis.diagnostics.is_empty(),
        "sema: {:?}",
        result.analysis.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    let ir = result.analysis.ir.expect("IR");
    assert_eq!(ir.graph.expect("graph").elements.len(), 3);
    assert_eq!(ir.filters.len(), 1);
}
