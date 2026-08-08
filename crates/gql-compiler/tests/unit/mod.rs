//! Unit tests for compiler frontend orchestration.

use crate::Compiler;
use gql_ir::Predicate;
use gql_catalog::{
    CatalogName,
    GqlCatalog,
    GraphName,
    PredicateDescriptor,
    RelationAuthority,
    RelationIdentity,
    RelationName,
};

#[derive(Default)]
struct StubCatalog;

impl StubCatalog {
    fn new() -> Self {
        Self
    }
}

impl GqlCatalog for StubCatalog {
    fn relation(&self, name: &RelationName) -> Option<gql_catalog::PredicateDescriptor> {
        if name.0 == "CALLS" {
            Some(PredicateDescriptor {
                name: name.clone(),
                columns: Vec::new(),
                relation_identity: RelationIdentity {
                    catalog: CatalogName("default-catalog".into()),
                    graph: GraphName("default-graph".into()),
                    schema: None,
                    node_types: Vec::new(),
                    edge_types: Vec::new(),
                },
                authority: RelationAuthority::Asserted {
                    source: "unit-test".into(),
                },
            })
        } else {
            None
        }
    }
}

#[test]
fn compile_match_return_vertical_slice() {
    let compiler = Compiler;
    let catalog = StubCatalog::new();
    let result = compiler.compile("q", "MATCH (a)-[:CALLS]->(b) RETURN a", &catalog);

    assert_eq!(result.analysis.diagnostics.len(), 0);
    assert!(result.analysis.ir.is_some());
    let ir = result
        .analysis
        .ir
        .expect("analysis should produce a query block");
    assert!(ir.graph.is_some());
    let graph = ir.graph.expect("graph pattern");
    assert_eq!(graph.elements.len(), 3);
}

#[test]
fn compile_accepts_match_with_node_only_pattern() {
    let compiler = Compiler;
    let catalog = StubCatalog::new();
    let result = compiler.compile("q", "MATCH (a) RETURN a", &catalog);

    assert!(result.analysis.ir.is_some());
    assert!(result.analysis.diagnostics.is_empty());
    let ir = result
        .analysis
        .ir
        .expect("analysis should produce a query block");
    assert!(ir.graph.is_some());
    assert_eq!(ir.graph.expect("graph pattern").elements.len(), 1);
}

#[test]
fn compile_recovers_from_invalid_return_clause() {
    let compiler = Compiler;
    let catalog = StubCatalog::new();
    let result = compiler.compile("q", "MATCH (a)-[:CALLS]->(b) RETURN", &catalog);

    assert!(result.analysis.ir.is_none());
    assert!(
        result
            .analysis
            .diagnostics
            .iter()
            .any(|d| d.code == "GQL-PARSE-RETURN-SYNTAX")
    );
}

#[test]
fn compile_accepts_where_clause_with_bound_identifier() {
    let compiler = Compiler;
    let catalog = StubCatalog::new();
    let result = compiler.compile(
        "q",
        "MATCH (a)-[:CALLS]->(b) WHERE a RETURN b",
        &catalog,
    );

    assert!(result.analysis.ir.is_some());
    assert!(
        result
            .analysis
            .diagnostics
            .is_empty(),
        "expected no diagnostics for basic where support"
    );
}

#[test]
fn compile_accepts_where_clause_with_binding_equality_to_integer() {
    let compiler = Compiler;
    let catalog = StubCatalog::new();
    let result = compiler.compile(
        "q",
        "MATCH (a)-[:CALLS]->(b) WHERE a = 1 RETURN b",
        &catalog,
    );

    assert!(result.analysis.ir.is_some());
    assert!(result.analysis.diagnostics.is_empty());
    let ir = result
        .analysis
        .ir
        .expect("analysis should produce a query block");
    assert_eq!(ir.predicates.len(), 1);
    match &ir.predicates[0] {
        Predicate::Equals(binding, value) => {
            assert_eq!(binding.name, "a");
            assert_eq!(value, &gql_types::Value::Integer(1));
        }
        predicate => panic!("unexpected predicate: {predicate:?}"),
    }
}

#[test]
fn compile_accepts_where_clause_with_binding_equality_to_string() {
    let compiler = Compiler;
    let catalog = StubCatalog::new();
    let result = compiler.compile(
        "q",
        "MATCH (a)-[:CALLS]->(b) WHERE a = 'x' RETURN b",
        &catalog,
    );

    assert!(result.analysis.ir.is_some());
    assert!(result.analysis.diagnostics.is_empty());
    let ir = result
        .analysis
        .ir
        .expect("analysis should produce a query block");
    assert_eq!(ir.predicates.len(), 1);
    match &ir.predicates[0] {
        Predicate::Equals(binding, value) => {
            assert_eq!(binding.name, "a");
            assert_eq!(value, &gql_types::Value::String("x".into()));
        }
        predicate => panic!("unexpected predicate: {predicate:?}"),
    }
}

#[test]
fn compile_accepts_where_clause_with_string_equality_to_binding() {
    let compiler = Compiler;
    let catalog = StubCatalog::new();
    let result = compiler.compile(
        "q",
        "MATCH (a)-[:CALLS]->(b) WHERE 'x' = a RETURN b",
        &catalog,
    );

    assert!(result.analysis.ir.is_some());
    assert!(result.analysis.diagnostics.is_empty());
    let ir = result
        .analysis
        .ir
        .expect("analysis should produce a query block");
    assert_eq!(ir.predicates.len(), 1);
    match &ir.predicates[0] {
        Predicate::Equals(binding, value) => {
            assert_eq!(binding.name, "a");
            assert_eq!(value, &gql_types::Value::String("x".into()));
        }
        predicate => panic!("unexpected predicate: {predicate:?}"),
    }
}
