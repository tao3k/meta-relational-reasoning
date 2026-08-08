use gql_core::ast::{Query, QueryClause, Statement};
use gql_core::catalog::{
    CatalogName, GqlCatalog, GraphName, PredicateDescriptor,
    RelationAuthority, RelationIdentity, RelationName,
};
use crate::syntax::TokenKind;

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
    #[derive(Default)]
    struct StubCatalog;

    impl StubCatalog {
        fn new() -> Self {
            Self
        }
    }

    impl GqlCatalog for StubCatalog {
        fn relation(&self, name: &RelationName) -> Option<gql_core::catalog::PredicateDescriptor> {
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
    assert_eq!(ir.scans.len(), 1);
    assert_eq!(ir.scans[0].relation.0, "CALLS");
    assert_eq!(ir.scans[0].bindings.len(), 2);
}
