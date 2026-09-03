use crate::Compiler;
use crate::ast::{Expression as AstExpression, QueryClause, Statement};
use crate::catalog::{Catalog, CatalogName};
use crate::ir::Expression as IrExpression;
use crate::syntax::{SyntaxElementKind, SyntaxKind, SyntaxNode};

fn empty_catalog() -> Catalog {
    Catalog::new(CatalogName("case-contract".into()), Vec::new(), Vec::new())
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
fn simple_case_survives_lossless_cst_ast_and_canonical_ir() {
    let source = "MATCH (n) RETURN CASE 1 WHEN 1 THEN 'one' ELSE 'other' END";
    let result = Compiler.compile("simple-case.gql", source, &empty_catalog());

    assert!(
        result.parse.diagnostics.is_empty(),
        "parse: {:?}",
        result.parse.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    let root = result.parse.tree.root();
    assert_eq!(count_kind(&root, SyntaxKind::CaseExpression), 1);
    assert_eq!(count_kind(&root, SyntaxKind::CaseWhenClause), 1);
    assert_eq!(count_kind(&root, SyntaxKind::CaseElseClause), 1);

    let Some(Statement::Query(query)) = &result.statement else {
        panic!("statement is query");
    };
    let Some(QueryClause::Return { projections, .. }) = query.clauses.last() else {
        panic!("RETURN clause exists");
    };
    let AstExpression::Case {
        operand,
        branches,
        else_result,
        span,
    } = &projections[0].expression
    else {
        panic!("typed CASE expression exists");
    };
    assert!(matches!(
        operand.as_deref(),
        Some(AstExpression::Integer(1, _))
    ));
    assert_eq!(branches.len(), 1);
    assert!(matches!(
        branches[0].condition,
        AstExpression::Integer(1, _)
    ));
    assert!(
        matches!(branches[0].result, AstExpression::String(ref literal) if literal.value == "one")
    );
    assert!(
        matches!(else_result.as_deref(), Some(AstExpression::String(literal)) if literal.value == "other")
    );
    assert_eq!(
        &source[span.start as usize..span.end as usize],
        "CASE 1 WHEN 1 THEN 'one' ELSE 'other' END"
    );

    assert!(
        result.analysis.diagnostics.is_empty(),
        "sema: {:?}",
        result.analysis.diagnostics
    );
    let expression = &result.analysis.ir.expect("CASE IR").projection[0].expression;
    let IrExpression::Case {
        operand,
        branches,
        else_result,
    } = expression
    else {
        panic!("canonical CASE expression exists");
    };
    assert!(matches!(operand.as_deref(), Some(IrExpression::Integer(1))));
    assert_eq!(branches.len(), 1);
    assert!(matches!(branches[0].condition, IrExpression::Integer(1)));
    assert!(matches!(branches[0].result, IrExpression::String(ref value) if value == "one"));
    assert!(
        matches!(else_result.as_deref(), Some(IrExpression::String(value)) if value == "other")
    );
}

#[test]
fn searched_and_nested_case_preserve_ordered_branch_structure() {
    let source = "MATCH (n) RETURN CASE WHEN TRUE THEN CASE WHEN FALSE THEN 0 ELSE 1 END WHEN FALSE THEN 2 ELSE 3 END";
    let result = Compiler.compile("nested-case.gql", source, &empty_catalog());

    assert!(
        result.parse.diagnostics.is_empty(),
        "parse: {:?}",
        result.parse.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(
        count_kind(&result.parse.tree.root(), SyntaxKind::CaseExpression),
        2
    );
    assert!(
        result.analysis.diagnostics.is_empty(),
        "sema: {:?}",
        result.analysis.diagnostics
    );

    let expression = &result.analysis.ir.expect("nested CASE IR").projection[0].expression;
    let IrExpression::Case {
        operand,
        branches,
        else_result,
    } = expression
    else {
        panic!("outer searched CASE exists");
    };
    assert!(operand.is_none());
    assert_eq!(branches.len(), 2);
    assert!(matches!(branches[0].condition, IrExpression::Boolean(true)));
    assert!(matches!(branches[0].result, IrExpression::Case { .. }));
    assert!(matches!(
        branches[1].condition,
        IrExpression::Boolean(false)
    ));
    assert!(matches!(branches[1].result, IrExpression::Integer(2)));
    assert!(matches!(
        else_result.as_deref(),
        Some(IrExpression::Integer(3))
    ));
}

#[test]
fn searched_case_rejects_non_boolean_when_and_emits_no_ir() {
    let result = Compiler.compile(
        "invalid-case-condition.gql",
        "MATCH (n) RETURN CASE WHEN 1 THEN 'invalid' END",
        &empty_catalog(),
    );

    assert!(
        result.parse.diagnostics.is_empty(),
        "parse: {:?}",
        result.parse.diagnostics
    );
    assert!(result.analysis.ir.is_none());
    assert_eq!(
        result
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-CASE-CONDITION-NOT-BOOLEAN"]
    );
}

#[test]
fn malformed_case_is_typed_parse_failure() {
    let source = "MATCH (n) RETURN CASE WHEN TRUE 'missing then' END";
    let result = Compiler.compile("malformed-case.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(result.analysis.ir.is_none());
    assert!(
        result
            .parse
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-PARSE-CASE-SYNTAX"),
        "parse diagnostics: {:?}",
        result.parse.diagnostics
    );
}
