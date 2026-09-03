use crate::Compiler;
use crate::ast::{
    BinaryOperator as AstBinaryOperator, Expression as AstExpression, QueryClause, Statement,
};
use crate::catalog::{Catalog, CatalogName};
use crate::ir::{BinaryOperator as IrBinaryOperator, Expression as IrExpression};

fn empty_catalog() -> Catalog {
    Catalog::new(CatalogName("where-contract".into()), Vec::new(), Vec::new())
}

#[test]
fn boolean_where_is_preserved_from_source_ast_to_canonical_ir() {
    let source = "MATCH (n) WHERE n.active = TRUE AND n IS LABELED :Person RETURN n";
    let result = Compiler.compile("boolean-where.gql", source, &empty_catalog());

    assert!(
        result.parse.diagnostics.is_empty(),
        "{:?}",
        result.parse.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    let Some(Statement::Query(query)) = &result.statement else {
        panic!("statement is a query");
    };
    let Some(QueryClause::Where { expression, span }) = query.clauses.get(1) else {
        panic!("WHERE clause is preserved in source order");
    };
    assert_eq!(
        &source[span.start as usize..span.end as usize],
        "WHERE n.active = TRUE AND n IS LABELED :Person"
    );
    assert!(matches!(
        expression,
        AstExpression::Binary {
            operator: AstBinaryOperator::And,
            ..
        }
    ));

    assert!(
        result.analysis.diagnostics.is_empty(),
        "{:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("canonical query IR");
    assert!(matches!(
        ir.filters.as_slice(),
        [IrExpression::Binary {
            operator: IrBinaryOperator::And,
            ..
        }]
    ));
}

#[test]
fn statically_non_boolean_where_is_typed_and_emits_no_ir() {
    let source = "MATCH (n) WHERE 'truthy' RETURN n";
    let result = Compiler.compile("non-boolean-where.gql", source, &empty_catalog());

    assert!(result.parse.diagnostics.is_empty());
    assert!(result.analysis.ir.is_none());
    assert_eq!(
        result
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-WHERE-NOT-BOOLEAN"]
    );
}

#[test]
fn where_without_a_pattern_scope_is_typed_and_emits_no_ir() {
    let source = "WHERE TRUE RETURN TRUE";
    let result = Compiler.compile("orphan-where.gql", source, &empty_catalog());

    assert!(result.parse.diagnostics.is_empty());
    assert!(result.analysis.ir.is_none());
    assert_eq!(
        result
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-WHERE-WITHOUT-PATTERN-SCOPE"]
    );
}

#[test]
fn missing_where_expression_is_exactly_once_typed_and_emits_no_statement_or_ir() {
    let source = "MATCH (n) WHERE RETURN n";
    let result = Compiler.compile("missing-where-expression.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(result.statement.is_none());
    assert!(result.analysis.ir.is_none());
    assert_eq!(
        result
            .parse
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-PARSE-WHERE-SYNTAX"]
    );
}
