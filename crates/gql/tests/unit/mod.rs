use gql_core::ast::{Query, QueryClause, Statement};
use gql_core::catalog::{
    CatalogName, GqlCatalog, GraphName, PredicateDescriptor,
    RelationAuthority, RelationIdentity, RelationName,
};
use crate::syntax::TokenKind;

#[derive(Default)]
struct StubCatalog;

impl StubCatalog {
    fn new() -> Self {
        Self
    }
}

impl GqlCatalog for StubCatalog {
    fn relation(&self, name: &RelationName) -> Option<gql_core::catalog::PredicateDescriptor> {
        (name.0 == "CALLS").then(|| PredicateDescriptor {
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
    }
}

#[test]
fn iso_parse_surface_is_feature_invariant() {
    let source = "MATCH (a)-[:CALLS]->(b) RETURN a, b";
    let parsed = crate::syntax::parse("feature-invariance.gql", source);
    assert!(parsed.diagnostics.is_empty());
    assert_eq!(parsed.tree.source().text(), source);
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
fn pipeline_matches_vertical_slice_from_source_to_ir() {
    let compiler = crate::compiler::Compiler;
    let catalog = StubCatalog::new();
    let source = "MATCH (a)-[:CALLS]->(b) RETURN a";

    let result = compiler.compile("pipeline.gql", source, &catalog);

    assert!(result.parse.diagnostics.is_empty());
    assert_eq!(result.parse.tree.source().text(), source);
    let statement = result.statement;
    match statement {
        Statement::Query(Query { clauses, .. }) => {
            assert!(clauses
                .iter()
                .any(|clause| matches!(clause, QueryClause::Match(_))));
        }
        _ => panic!("unexpected statement kind"),
    }

    let ir = result
        .analysis
        .ir
        .expect("analysis should produce IR");
    assert!(result.analysis.diagnostics.is_empty());
    let graph = ir.graph.expect("graph pattern");
    assert_eq!(graph.elements.len(), 3);
}

#[test]
fn pipeline_matches_required_path_simple_projection() {
    let compiler = crate::compiler::Compiler;
    let catalog = StubCatalog::new();
    let source = "MATCH (n) RETURN n";

    let result = compiler.compile("required-path-simple.gql", source, &catalog);

    assert_eq!(result.parse.diagnostics.len(), 0);
    assert_eq!(result.parse.tree.source().text(), source);
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    let ir = result
        .analysis
        .ir
        .expect("analysis should produce IR");
    assert!(result.analysis.diagnostics.is_empty());
    assert_eq!(
        ir.graph.expect("graph pattern").elements.len(),
        1,
        "simple projection should preserve single node pattern"
    );
}

#[test]
fn pipeline_matches_required_path_where_filtering() {
    let compiler = crate::compiler::Compiler;
    let catalog = StubCatalog::new();
    let source = "MATCH (a)-[:CALLS]->(b) WHERE a = 1 RETURN b";

    let result = compiler.compile("required-path-where.gql", source, &catalog);

    assert_eq!(result.parse.diagnostics.len(), 0);
    assert_eq!(result.parse.tree.source().text(), source);
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(result.analysis.diagnostics.len(), 0);
    let ir = result
        .analysis
        .ir
        .expect("analysis should produce IR");
    assert_eq!(ir.graph.expect("graph pattern").elements.len(), 3);
    assert_eq!(ir.predicates.len(), 1);
}

#[test]
fn pipeline_matches_vertical_slice_without_edge_relation() {
    let compiler = crate::compiler::Compiler;
    let catalog = StubCatalog::new();
    let source = "MATCH (a) RETURN a";

    let result = compiler.compile("pipeline-no-relation.gql", source, &catalog);

    assert!(result.analysis.diagnostics.is_empty());
    let ir = result
        .analysis
        .ir
        .expect("analysis should produce IR");
    let graph = ir.graph.expect("graph pattern");
    assert_eq!(graph.elements.len(), 1);
}
