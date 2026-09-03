use crate::Compiler;
use crate::ast::{
    GraphMatchMode as AstGraphMatchMode,
    NonNegativeIntegerSpecification as AstNonNegativeIntegerSpecification, PathMode as AstPathMode,
    PathSearch as AstPathSearch, PathTarget, QueryClause, Statement,
};
use crate::catalog::{Catalog, CatalogName};
use crate::ir::{
    GraphMatchMode as IrGraphMatchMode,
    NonNegativeIntegerSpecification as IrNonNegativeIntegerSpecification, PathMode as IrPathMode,
    PathSearch as IrPathSearch,
};
use crate::syntax::{SyntaxElementKind, SyntaxKind, SyntaxNode};

fn empty_catalog() -> Catalog {
    Catalog::new(
        CatalogName("path-search-prefix-contract".into()),
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
fn iso_match_mode_path_search_and_keep_family_crosses_frontend_admission() {
    let source = "MATCH REPEATABLE ELEMENT BINDINGS p = ALL SHORTEST TRAIL PATHS (a)-[e]->(b), q = SHORTEST 2 SIMPLE PATHS (c)-[f]->(d) KEEP ANY 3 WALK PATHS RETURN p, q";
    let result = Compiler.compile("repeatable-search-and-keep.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(result.parse.diagnostics.is_empty());
    let root = result.parse.tree.root();
    assert_eq!(count_kind(&root, SyntaxKind::GraphMatchMode), 1);
    assert_eq!(count_kind(&root, SyntaxKind::PathPrefix), 3);
    assert_eq!(count_kind(&root, SyntaxKind::PathSearch), 3);
    assert_eq!(count_kind(&root, SyntaxKind::KeepClause), 1);

    let Some(Statement::Query(query)) = &result.statement else {
        panic!("path-search source must remain a query");
    };
    let Some(QueryClause::Match(matched)) = query.clauses.first() else {
        panic!("MATCH clause exists");
    };
    assert_eq!(matched.mode, Some(AstGraphMatchMode::RepeatableElements));
    assert_eq!(matched.patterns.len(), 2);
    assert_eq!(
        matched.patterns[0]
            .binding
            .as_ref()
            .map(|item| item.text.as_str()),
        Some("p")
    );
    assert_eq!(
        matched.patterns[1]
            .binding
            .as_ref()
            .map(|item| item.text.as_str()),
        Some("q")
    );
    let first = matched.patterns[0].prefix.as_ref().expect("first prefix");
    assert_eq!(first.search, Some(AstPathSearch::AllShortest));
    assert_eq!(first.mode, Some(AstPathMode::Trail));
    assert_eq!(first.target, Some(PathTarget::Paths));
    assert_eq!(
        source[first.span.start as usize..first.span.end as usize].trim(),
        "ALL SHORTEST TRAIL PATHS"
    );
    let second = matched.patterns[1].prefix.as_ref().expect("second prefix");
    assert_eq!(
        second.search,
        Some(AstPathSearch::Shortest {
            count: AstNonNegativeIntegerSpecification::Literal(2)
        })
    );
    assert_eq!(second.mode, Some(AstPathMode::Simple));
    assert_eq!(second.target, Some(PathTarget::Paths));
    let keep = matched.keep.as_ref().expect("KEEP prefix");
    assert_eq!(
        keep.search,
        Some(AstPathSearch::Any {
            count: Some(AstNonNegativeIntegerSpecification::Literal(3))
        })
    );
    assert_eq!(keep.mode, Some(AstPathMode::Walk));
    assert_eq!(keep.target, Some(PathTarget::Paths));

    assert!(result.analysis.diagnostics.is_empty());
    let ir = result.analysis.ir.expect("canonical path-search IR");
    assert_eq!(ir.matches.len(), 1);
    assert_eq!(
        ir.matches[0].mode,
        Some(IrGraphMatchMode::RepeatableElements)
    );
    assert_eq!(ir.matches[0].paths.len(), 2);
    assert_eq!(ir.matches[0].paths[0].binding.as_deref(), Some("P"));
    assert_eq!(ir.matches[0].paths[1].binding.as_deref(), Some("Q"));
    let first = ir.matches[0].paths[0]
        .prefix
        .as_ref()
        .expect("first IR prefix");
    assert_eq!(first.search, Some(IrPathSearch::AllShortest));
    assert_eq!(first.mode, Some(IrPathMode::Trail));
    let second = ir.matches[0].paths[1]
        .prefix
        .as_ref()
        .expect("second IR prefix");
    assert_eq!(
        second.search,
        Some(IrPathSearch::Shortest {
            count: IrNonNegativeIntegerSpecification::Literal(2)
        })
    );
    assert_eq!(second.mode, Some(IrPathMode::Simple));
    let keep = ir.matches[0].keep.as_ref().expect("KEEP IR prefix");
    assert_eq!(
        keep.search,
        Some(IrPathSearch::Any {
            count: Some(IrNonNegativeIntegerSpecification::Literal(3))
        })
    );
    assert_eq!(keep.mode, Some(IrPathMode::Walk));
}

#[test]
fn remaining_iso_match_mode_and_shortest_group_forms_are_canonical() {
    let different = Compiler.compile(
        "different-edge-search.gql",
        "MATCH DIFFERENT EDGES ALL WALK PATHS (a)-[e]->(b), ANY SHORTEST ACYCLIC PATH (b)-[f]->(c) RETURN a, c",
        &empty_catalog(),
    );
    assert!(different.parse.diagnostics.is_empty());
    assert!(different.analysis.diagnostics.is_empty());
    let ir = different.analysis.ir.expect("different-edge IR");
    assert_eq!(ir.matches[0].mode, Some(IrGraphMatchMode::DifferentEdges));
    assert_eq!(
        ir.matches[0].paths[0]
            .prefix
            .as_ref()
            .and_then(|item| item.search.clone()),
        Some(IrPathSearch::All)
    );
    assert_eq!(
        ir.matches[0].paths[1]
            .prefix
            .as_ref()
            .and_then(|item| item.search.clone()),
        Some(IrPathSearch::AnyShortest)
    );

    let grouped = Compiler.compile(
        "shortest-group-search.gql",
        "MATCH SHORTEST 2 TRAIL PATHS GROUPS (a)-[e]->(b) RETURN a",
        &empty_catalog(),
    );
    assert!(grouped.parse.diagnostics.is_empty());
    assert!(grouped.analysis.diagnostics.is_empty());
    assert_eq!(
        grouped.analysis.ir.expect("shortest group IR").matches[0].paths[0]
            .prefix
            .as_ref()
            .and_then(|item| item.search.clone()),
        Some(IrPathSearch::ShortestGroups {
            count: Some(IrNonNegativeIntegerSpecification::Literal(2))
        })
    );
}

#[test]
fn official_match_mode_spellings_and_parameterized_non_negative_counts_are_complete() {
    for mode in [
        "REPEATABLE ELEMENT",
        "REPEATABLE ELEMENT BINDINGS",
        "REPEATABLE ELEMENTS",
        "DIFFERENT EDGE",
        "DIFFERENT EDGE BINDINGS",
        "DIFFERENT RELATIONSHIP",
        "DIFFERENT RELATIONSHIP BINDINGS",
        "DIFFERENT EDGES",
        "DIFFERENT RELATIONSHIPS",
    ] {
        let source = format!("MATCH {mode} (a)-[e]->(b) RETURN a");
        let result = Compiler.compile("match-mode-spelling.gql", &source, &empty_catalog());
        assert!(result.parse.diagnostics.is_empty(), "{mode}");
        assert!(result.analysis.ir.is_some(), "{mode}");
    }

    for prefix in [
        "ALL",
        "ANY",
        "ANY 0",
        "ALL SHORTEST",
        "ANY SHORTEST",
        "SHORTEST 0",
        "SHORTEST GROUPS",
        "SHORTEST 0 GROUP",
    ] {
        let source = format!("MATCH {prefix} (a)-[e]->(b) RETURN a");
        let result = Compiler.compile("path-search-spelling.gql", &source, &empty_catalog());
        assert!(result.parse.diagnostics.is_empty(), "{prefix}");
        assert!(result.analysis.ir.is_some(), "{prefix}");
    }

    let parameterized = Compiler.compile(
        "parameterized-path-count.gql",
        "MATCH p = SHORTEST $limit PATHS (a)-[e]->(b), q = ANY $42 TRAIL PATHS (b)-[f]->(c) RETURN p, q",
        &empty_catalog(),
    );
    assert!(parameterized.parse.diagnostics.is_empty());
    assert!(parameterized.analysis.diagnostics.is_empty());
    let ir = parameterized
        .analysis
        .ir
        .expect("parameterized path-count IR");
    assert_eq!(
        ir.matches[0].paths[0]
            .prefix
            .as_ref()
            .and_then(|item| item.search.clone()),
        Some(IrPathSearch::Shortest {
            count: IrNonNegativeIntegerSpecification::Parameter("limit".into())
        })
    );
    assert_eq!(
        ir.matches[0].paths[1]
            .prefix
            .as_ref()
            .and_then(|item| item.search.clone()),
        Some(IrPathSearch::Any {
            count: Some(IrNonNegativeIntegerSpecification::Parameter("42".into()))
        })
    );
}

#[test]
fn malformed_match_modes_and_path_prefixes_are_exactly_once_typed_and_emit_no_ir() {
    for (source, expected) in [
        (
            "MATCH REPEATABLE EDGES (a) RETURN a",
            "GQL-PARSE-GRAPH-MATCH-MODE-SYNTAX",
        ),
        (
            "MATCH DIFFERENT ELEMENTS (a) RETURN a",
            "GQL-PARSE-GRAPH-MATCH-MODE-SYNTAX",
        ),
        (
            "MATCH REPEATABLE ELEMENTS BINDINGS (a) RETURN a",
            "GQL-PARSE-GRAPH-MATCH-MODE-SYNTAX",
        ),
        (
            "MATCH DIFFERENT EDGE BINDING (a) RETURN a",
            "GQL-PARSE-GRAPH-MATCH-MODE-SYNTAX",
        ),
        (
            "MATCH SHORTEST (a) RETURN a",
            "GQL-PARSE-PATH-SEARCH-PREFIX-SYNTAX",
        ),
        (
            "MATCH ALL GROUPS (a) RETURN a",
            "GQL-PARSE-PATH-SEARCH-PREFIX-SYNTAX",
        ),
        (
            "MATCH SHORTEST 2.5 (a) RETURN a",
            "GQL-PARSE-PATH-SEARCH-PREFIX-SYNTAX",
        ),
        (
            "MATCH ANY -1 (a) RETURN a",
            "GQL-PARSE-PATH-SEARCH-PREFIX-SYNTAX",
        ),
        ("MATCH (a) KEEP RETURN a", "GQL-PARSE-KEEP-CLAUSE-SYNTAX"),
    ] {
        let result = Compiler.compile("invalid-path-prefix.gql", source, &empty_catalog());
        assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
        assert_eq!(
            result
                .parse
                .diagnostics
                .iter()
                .map(|item| item.code)
                .collect::<Vec<_>>(),
            [expected],
            "{source}"
        );
        assert!(result.statement.is_none(), "{source}");
        assert!(result.analysis.ir.is_none(), "{source}");
    }
}
