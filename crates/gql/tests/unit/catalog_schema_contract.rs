use crate::Compiler;
use crate::ast::{CatalogStatement, Statement};
use crate::catalog::{Catalog, CatalogName, Schema, SchemaName};
use crate::ir::CatalogCommand;
use crate::syntax::{SyntaxElementKind, SyntaxKind, SyntaxNode};

fn catalog(schemas: Vec<Schema>) -> Catalog {
    Catalog::new(CatalogName("catalog-contract".into()), Vec::new(), schemas)
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
fn create_schema_reaches_one_backend_neutral_catalog_command() {
    let source = "CREATE SCHEMA analytics";
    let result = Compiler.compile("create-schema.gql", source, &catalog(Vec::new()));

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse diagnostics: {:?}",
        result.parse.diagnostics
    );
    assert!(contains_node_kind(
        &result.parse.tree.root(),
        SyntaxKind::CreateSchemaStatement
    ));
    let Some(Statement::Catalog(CatalogStatement::CreateSchema { name })) = &result.statement
    else {
        panic!("CREATE SCHEMA must lower to a catalog statement");
    };
    assert_eq!(name.parts[0].text, "analytics");
    assert_eq!(
        &source[name.span.start as usize..name.span.end as usize],
        "analytics"
    );
    assert!(result.analysis.diagnostics.is_empty());
    assert!(result.analysis.ir.is_none());
    assert_eq!(
        result.analysis.catalog_command,
        Some(CatalogCommand::CreateSchema {
            name: crate::ir::CatalogObjectName {
                parts: vec!["ANALYTICS".into()],
            },
        })
    );
}

#[test]
fn create_existing_schema_is_typed_and_emits_no_command() {
    let source = "CREATE SCHEMA analytics";
    let existing = Schema::new(
        SchemaName("ANALYTICS".into()),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    let result = Compiler.compile("duplicate-schema.gql", source, &catalog(vec![existing]));

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(
        result
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-SCHEMA-ALREADY-EXISTS"]
    );
    assert!(result.analysis.ir.is_none());
    assert!(result.analysis.catalog_command.is_none());
}
