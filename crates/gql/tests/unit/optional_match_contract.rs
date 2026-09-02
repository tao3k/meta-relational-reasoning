use crate::Compiler;
use crate::catalog::{Catalog, CatalogName};
use crate::ir::{Expression as IrExpression, GraphPatternElement, PathMode};

fn empty_catalog() -> Catalog {
    Catalog::new(
        CatalogName("optional-match-contract".into()),
        Vec::new(),
        Vec::new(),
    )
}

#[test]
fn standalone_optional_match_is_one_canonical_left_outer_group() {
    let source = "OPTIONAL MATCH (a)-[:KNOWS]->(b) RETURN a, b";
    let result = Compiler.compile("standalone-optional-match.gql", source, &empty_catalog());

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
    let ir = result.analysis.ir.expect("standalone optional MATCH IR");
    assert!(ir.graphs.is_empty());
    assert_eq!(ir.optional_matches.len(), 1);
    assert_eq!(ir.optional_matches[0].graphs.len(), 1);
    assert!(ir.optional_matches[0].predicate.is_none());
    assert!(matches!(
        ir.optional_matches[0].graphs[0].elements.as_slice(),
        [
            GraphPatternElement::Node(_),
            GraphPatternElement::Edge(_),
            GraphPatternElement::Node(_),
        ]
    ));
}

#[test]
fn optional_match_pattern_list_remains_one_group_with_shared_bindings() {
    let source = "MATCH (a) OPTIONAL MATCH TRAIL (a)-[:KNOWS]->(b), (b)-[:WORKS_AT]->(c) RETURN c";
    let result = Compiler.compile("optional-pattern-list.gql", source, &empty_catalog());

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
    let ir = result.analysis.ir.expect("optional pattern-list IR");
    assert_eq!(ir.optional_matches.len(), 1);
    assert_eq!(ir.optional_matches[0].graphs.len(), 2);
    assert!(
        ir.optional_matches[0]
            .graphs
            .iter()
            .all(|graph| graph.mode == PathMode::Trail)
    );
}

#[test]
fn optional_match_where_stays_inside_the_left_outer_group() {
    let source = "MATCH (a) OPTIONAL MATCH (a)-[e:KNOWS]->(b) WHERE e.active = TRUE RETURN b";
    let result = Compiler.compile("optional-match-where.gql", source, &empty_catalog());

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
    let ir = result.analysis.ir.expect("optional WHERE IR");
    assert!(ir.filters.is_empty());
    assert_eq!(ir.optional_matches.len(), 1);
    assert!(matches!(
        ir.optional_matches[0].predicate,
        Some(IrExpression::Binary { .. })
    ));
}

#[test]
fn optional_match_binding_kind_conflict_is_typed_and_emits_no_ir() {
    let source = "OPTIONAL MATCH (x), ()-[x]->() RETURN x";
    let result = Compiler.compile("optional-kind-conflict.gql", source, &empty_catalog());

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

#[test]
fn malformed_optional_match_is_exactly_once_typed_and_emits_no_ir() {
    let source = "OPTIONAL RETURN a";
    let result = Compiler.compile("malformed-optional-match.gql", source, &empty_catalog());

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
        ["GQL-PARSE-OPTIONAL-MATCH-SYNTAX"]
    );
}
