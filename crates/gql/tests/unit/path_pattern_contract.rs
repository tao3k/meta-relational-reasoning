use crate::Compiler;
use crate::ast::{PathMode as AstPathMode, QueryClause, Statement};
use crate::catalog::{Catalog, CatalogName};
use crate::ir::PathMode as IrPathMode;
use crate::syntax::{SyntaxElementKind, SyntaxKind, SyntaxNode};

fn empty_catalog() -> Catalog {
    Catalog::new(
        CatalogName("path-pattern-contract".into()),
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
fn trail_path_mode_survives_lossless_cst_ast_and_canonical_ir() {
    let source = "MATCH TRAIL (a)-[:KNOWS]->{1,3}(b) RETURN a";
    let result = Compiler.compile("trail-path-mode.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse diagnostics: {:?}",
        result.parse.diagnostics
    );
    assert!(contains_node_kind(
        &result.parse.tree.root(),
        SyntaxKind::PathMode
    ));

    let Some(Statement::Query(query)) = &result.statement else {
        panic!("TRAIL source must remain a query");
    };
    let Some(QueryClause::Match(match_clause)) = query.clauses.first() else {
        panic!("MATCH clause exists");
    };
    assert_eq!(match_clause.mode, AstPathMode::Trail);
    assert_eq!(
        &source[match_clause.span.start as usize..match_clause.span.end as usize],
        "MATCH TRAIL (a)-[:KNOWS]->{1,3}(b)"
    );

    assert!(
        result.analysis.diagnostics.is_empty(),
        "semantic diagnostics: {:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("canonical TRAIL IR");
    assert_eq!(
        ir.graphs.into_iter().next().expect("graph").mode,
        IrPathMode::Trail
    );
}

#[test]
fn path_mode_without_pattern_is_typed_and_emits_no_ir() {
    let source = "MATCH TRAIL RETURN a";
    let result = Compiler.compile("missing-trail-pattern.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(
        result
            .parse
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-PARSE-PATH-MODE-SYNTAX"]
    );
    assert!(result.statement.is_none());
    assert!(result.analysis.ir.is_none());
}

#[test]
fn every_declared_path_mode_and_the_implicit_walk_are_canonical() {
    let cases = [
        ("WALK ", AstPathMode::Walk, IrPathMode::Walk),
        ("TRAIL ", AstPathMode::Trail, IrPathMode::Trail),
        ("ACYCLIC ", AstPathMode::Acyclic, IrPathMode::Acyclic),
        ("SIMPLE ", AstPathMode::Simple, IrPathMode::Simple),
        ("", AstPathMode::Walk, IrPathMode::Walk),
    ];

    for (spelling, expected_ast, expected_ir) in cases {
        let source = format!("MATCH {spelling}(a)-[:KNOWS]->(b) RETURN a");
        let result = Compiler.compile("path-mode.gql", &source, &empty_catalog());
        assert!(result.parse.diagnostics.is_empty(), "{spelling}");

        let Some(Statement::Query(query)) = &result.statement else {
            panic!("path mode source must remain a query");
        };
        let Some(QueryClause::Match(match_clause)) = query.clauses.first() else {
            panic!("MATCH clause exists");
        };
        assert_eq!(match_clause.mode, expected_ast, "{spelling}");
        assert_eq!(
            result
                .analysis
                .ir
                .expect("IR")
                .graphs
                .into_iter()
                .next()
                .expect("graph")
                .mode,
            expected_ir,
            "{spelling}"
        );
    }
}
