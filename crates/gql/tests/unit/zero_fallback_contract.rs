use crate::Compiler;
use crate::ast::{Query, Statement};
use crate::catalog::{Catalog, CatalogName};

fn empty_catalog() -> Catalog {
    Catalog::new(
        CatalogName("zero-fallback-contract".into()),
        Vec::new(),
        Vec::new(),
    )
}

#[test]
fn rejected_source_never_fabricates_a_statement_or_ir() {
    for (name, source, expected_code) in [
        (
            "unsupported-create-index.gql",
            "CREATE INDEX analytics",
            "GQL-PARSE-UNSUPPORTED-STATEMENT",
        ),
        (
            "missing-where-expression.gql",
            "MATCH (n) WHERE RETURN n",
            "GQL-PARSE-WHERE-SYNTAX",
        ),
        (
            "missing-let-binding.gql",
            "MATCH (n) LET = 1 RETURN n",
            "GQL-AST-LET-BINDING-EXPECTED",
        ),
        (
            "missing-optional-match.gql",
            "OPTIONAL RETURN n",
            "GQL-PARSE-OPTIONAL-MATCH-SYNTAX",
        ),
        (
            "missing-named-path-pattern.gql",
            "MATCH p = RETURN p",
            "GQL-PARSE-PATH-SYNTAX",
        ),
    ] {
        let lowered = Compiler.lower(name, source);
        assert!(
            lowered.statement.is_none(),
            "rejected source must not fabricate an AST statement: {source}"
        );
        assert_eq!(
            lowered
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect::<Vec<_>>(),
            [expected_code]
        );

        let compiled = Compiler.compile(name, source, &empty_catalog());
        assert!(compiled.statement.is_none());
        assert!(compiled.analysis.ir.is_none());
        assert!(compiled.analysis.catalog_command.is_none());
        assert!(compiled.analysis.procedure_command.is_none());
        assert!(compiled.analysis.transaction_command.is_none());
        assert!(compiled.analysis.session_command.is_none());
        assert_eq!(
            compiled
                .analysis
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect::<Vec<_>>(),
            [expected_code]
        );
    }
}

#[test]
fn semantic_analysis_rejects_an_empty_query_without_default_ir() {
    let analysis = Compiler.analyze(&Statement::Query(Query::default()), &empty_catalog());

    assert!(analysis.ir.is_none());
    assert!(analysis.catalog_command.is_none());
    assert!(analysis.procedure_command.is_none());
    assert!(analysis.transaction_command.is_none());
    assert!(analysis.session_command.is_none());
    assert_eq!(
        analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-EMPTY-QUERY"]
    );
}
