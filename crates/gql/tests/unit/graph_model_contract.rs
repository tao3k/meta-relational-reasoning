use crate::Compiler;
use crate::ast::{PatternElement as AstPatternElement, QueryClause, Statement};
use crate::catalog::{Catalog, CatalogName};
use crate::ir::{Expression as IrExpression, GraphPatternElement as IrPatternElement};

fn empty_catalog() -> Catalog {
    Catalog::new(
        CatalogName("graph-model-contract".into()),
        Vec::new(),
        Vec::new(),
    )
}

#[test]
fn unlabeled_edge_binding_survives_ast_and_canonical_ir() {
    let source = "MATCH (a)-[e]->(b) RETURN e";
    let result = Compiler.compile("unlabeled-edge-binding.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse diagnostics: {:?}",
        result.parse.diagnostics
    );
    let Some(Statement::Query(query)) = &result.statement else {
        panic!("edge source must remain a query");
    };
    let Some(QueryClause::Match(match_clause)) = query.clauses.first() else {
        panic!("MATCH clause exists");
    };
    let Some(AstPatternElement::Edge(edge)) = match_clause.patterns[0].elements.get(1) else {
        panic!("edge pattern exists");
    };
    assert_eq!(
        edge.binding.as_ref().map(|binding| binding.text.as_str()),
        Some("e")
    );
    assert!(edge.labels.is_empty());
    assert_eq!(
        &source[edge.span.start as usize..edge.span.end as usize],
        "-[e]->"
    );

    assert!(
        result.analysis.diagnostics.is_empty(),
        "semantic diagnostics: {:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("canonical unlabeled edge IR");
    let Some(IrPatternElement::Edge(edge)) = ir.graphs.first().expect("graph").elements.get(1)
    else {
        panic!("canonical edge exists");
    };
    assert_eq!(edge.binding.as_deref(), Some("E"));
    assert!(edge.labels.is_empty());
    assert!(matches!(
        ir.projection.as_slice(),
        [crate::ir::Projection {
            expression: IrExpression::Binding(binding),
            ..
        }] if binding == "E"
    ));
}

#[test]
fn edge_binding_and_label_without_separator_is_typed_and_emits_no_ir() {
    let source = "MATCH (a)-[e KNOWS]->(b) RETURN e";
    let result = Compiler.compile("edge-label-separator.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(result.parse.diagnostics.len(), 1);
    let diagnostic = &result.parse.diagnostics[0];
    assert_eq!(diagnostic.code, "GQL-PARSE-EDGE-LABEL-SEPARATOR");
    assert_eq!(
        &source[diagnostic.span.start as usize..diagnostic.span.end as usize],
        "KNOWS"
    );
    assert!(result.analysis.ir.is_none());
}
