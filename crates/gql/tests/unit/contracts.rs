use crate::catalog::{Catalog, CatalogName};
use crate::syntax::TokenKind;

#[path = "frontend_contract.rs"]
mod frontend_contract;
#[path = "graph_model_contract.rs"]
mod graph_model_contract;

#[path = "case_expression_contract.rs"]
mod case_expression_contract;
#[path = "catalog_schema_contract.rs"]
mod catalog_schema_contract;
#[path = "data_management_contract.rs"]
mod data_management_contract;
#[path = "edge_pattern_contract.rs"]
mod edge_pattern_contract;
#[path = "edge_type_specification_contract.rs"]
mod edge_type_specification_contract;
#[path = "expression_language_contract.rs"]
mod expression_language_contract;

#[path = "identifier_contract.rs"]
mod identifier_contract;

#[path = "literal_contract.rs"]
mod literal_contract;
#[path = "match_contract.rs"]
mod match_contract;
#[path = "node_pattern_contract.rs"]
mod node_pattern_contract;
#[path = "optional_match_contract.rs"]
mod optional_match_contract;
#[path = "path_pattern_contract.rs"]
mod path_pattern_contract;
#[path = "property_label_expression_contract.rs"]
mod property_label_expression_contract;
#[path = "quantified_path_contract.rs"]
mod quantified_path_contract;
#[path = "query_pipeline_contract.rs"]
mod query_pipeline_contract;
#[path = "trivia_contract.rs"]
mod trivia_contract;
#[path = "value_type_specification_contract.rs"]
mod value_type_specification_contract;
#[path = "values_contract.rs"]
mod values_contract;
#[path = "where_contract.rs"]
mod where_contract;
#[path = "zero_fallback_contract.rs"]
mod zero_fallback_contract;

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
            .graphs
            .into_iter()
            .next()
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
    assert_eq!(
        ir.graphs.into_iter().next().expect("graph").elements.len(),
        3
    );
    assert_eq!(ir.filters.len(), 1);
}
