use crate::Compiler;
use gql_catalog::{Catalog, CatalogName};
use gql_ir::Expression;

fn catalog() -> Catalog {
    Catalog::new(CatalogName("test-catalog".into()), Vec::new(), Vec::new())
}

#[test]
fn compiler_preserves_rowan_source_for_node_only_vertical_slice() {
    let source = "MATCH (n) RETURN n";
    let compiler = Compiler;
    let result = compiler.compile("node-only.gql", source, &catalog());

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
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    let ir = result
        .analysis
        .ir
        .expect("node-only query should produce IR");
    assert_eq!(ir.graph.expect("graph pattern").elements.len(), 1);
    assert_eq!(ir.projection[0].expression, Expression::Binding("n".into()));
}

#[test]
fn compiler_preserves_graph_filter_and_projection_vertical_slice() {
    let source = "MATCH (a)-[:CALLS]->(b) WHERE a = 1 RETURN b";
    let compiler = Compiler;
    let result = compiler.compile("graph-filter.gql", source, &catalog());

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
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    let ir = result.analysis.ir.expect("graph query should produce IR");
    assert_eq!(ir.graph.expect("graph pattern").elements.len(), 3);
    assert_eq!(ir.filters.len(), 1);
    assert_eq!(ir.projection[0].expression, Expression::Binding("b".into()));
}

#[test]
fn compiler_reports_invalid_return_while_preserving_rowan_source() {
    let source = "MATCH (a) RETURN";
    let compiler = Compiler;
    let result = compiler.compile("invalid-return.gql", source, &catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result
            .analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-PARSE-RETURN-SYNTAX")
    );
}
