use crate::Compiler;
use crate::ast::{Expression as AstExpression, QueryClause, Statement};
use crate::catalog::{Catalog, CatalogName};
use crate::ir::{AggregateFunction, Expression as IrExpression, SetOperator, SortDirection};
use crate::syntax::{SyntaxElementKind, SyntaxKind, SyntaxNode};

fn node_receipt(node: &SyntaxNode, source: &str, output: &mut Vec<(SyntaxKind, String)>) {
    let span = node.span();
    output.push((
        node.kind(),
        source[span.start as usize..span.end as usize].to_owned(),
    ));
    for element in node.children() {
        if let SyntaxElementKind::Node(child) = element.kind {
            node_receipt(&child, source, output);
        }
    }
}

fn empty_catalog() -> Catalog {
    Catalog::new(
        CatalogName("query-syntax-mvp".into()),
        Vec::new(),
        Vec::new(),
    )
}

#[test]
fn complete_query_pipeline_reaches_one_canonical_branch() {
    let source = "MATCH (n) LET score = n.score, name = n.name RETURN name AS display_name, score AS rank ORDER BY rank DESC OFFSET 2 LIMIT 10";
    let result = Compiler.compile("complete-query-pipeline.gql", source, &empty_catalog());

    assert!(
        result.parse.diagnostics.is_empty(),
        "{:?}",
        result.parse.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.analysis.diagnostics.is_empty(),
        "{:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("canonical query branch");
    assert_eq!(ir.let_bindings.len(), 2);
    assert_eq!(ir.projection.len(), 2);
    assert_eq!(ir.projection[0].alias.as_deref(), Some("DISPLAY_NAME"));
    assert_eq!(ir.projection[1].alias.as_deref(), Some("RANK"));
    assert!(matches!(
        ir.order_by.as_slice(),
        [crate::ir::SortKey {
            expression: IrExpression::Binding(name),
            direction: SortDirection::Descending,
        }] if name == "RANK"
    ));
    assert_eq!(ir.offset, Some(2));
    assert_eq!(ir.limit, Some(10));

    let Some(Statement::Query(query)) = result.statement else {
        panic!("query AST must be admitted");
    };
    let Some(QueryClause::Let { bindings, .. }) = query
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::Let { .. }))
    else {
        panic!("LET clause must be typed");
    };
    assert_eq!(bindings.len(), 2);
    assert_eq!(bindings[0].binding.text, "score");
    assert_eq!(bindings[1].binding.text, "name");
    assert!(bindings[0].span.end <= bindings[1].span.start);
    assert_eq!(
        &source[bindings[0].binding.span.start as usize..bindings[0].binding.span.end as usize],
        "score"
    );
    let Some(QueryClause::Return { projections, .. }) = query
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::Return { .. }))
    else {
        panic!("RETURN clause must be typed");
    };
    assert_eq!(projections.len(), 2);
    assert!(matches!(projections[0].expression, AstExpression::Name(_)));
    assert_eq!(
        projections[0]
            .alias
            .as_ref()
            .map(|alias| alias.text.as_str()),
        Some("display_name")
    );
}

#[test]
fn grouping_aggregation_ordering_and_pagination_share_one_result_scope() {
    let source = "MATCH (n) LET team = n.team RETURN team AS team, COUNT(n) AS total GROUP BY team ORDER BY total DESC OFFSET 1 LIMIT 10";
    let result = Compiler.compile("grouped-query-pipeline.gql", source, &empty_catalog());

    assert!(
        result.parse.diagnostics.is_empty(),
        "{:?}",
        result.parse.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    let Some(Statement::Query(query)) = &result.statement else {
        panic!("grouped query AST must be admitted");
    };
    let Some(QueryClause::Return { projections, .. }) = query
        .clauses
        .iter()
        .find(|clause| matches!(clause, QueryClause::Return { .. }))
    else {
        panic!("grouped RETURN clause must be typed");
    };
    let mut cst_nodes = Vec::new();
    node_receipt(&result.parse.tree.root(), source, &mut cst_nodes);
    assert_eq!(
        projections
            .iter()
            .map(|projection| projection.alias.as_ref().map(|alias| alias.text.as_str()))
            .collect::<Vec<_>>(),
        [Some("team"), Some("total")],
        "{projections:?}; cst={cst_nodes:?}"
    );
    assert!(
        result.analysis.diagnostics.is_empty(),
        "{:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("grouped canonical query");
    assert_eq!(ir.group_by, [IrExpression::Binding("TEAM".into())]);
    assert!(matches!(
        ir.projection[1].expression,
        IrExpression::Aggregate {
            function: AggregateFunction::Count,
            ref arguments,
        } if arguments == &[IrExpression::Binding("N".into())]
    ));
    assert!(matches!(
        ir.order_by.as_slice(),
        [crate::ir::SortKey {
            expression: IrExpression::Binding(name),
            direction: SortDirection::Descending,
        }] if name == "TOTAL"
    ));
    assert_eq!((ir.offset, ir.limit), (Some(1), Some(10)));
}

#[test]
fn union_preserves_typed_operator_branch_order_and_output_identity() {
    let source = "RETURN 1 AS x UNION RETURN 2 AS x";
    let result = Compiler.compile("typed-union.gql", source, &empty_catalog());

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
    let ir = result.analysis.ir.expect("typed UNION IR");
    assert_eq!(ir.projection[0].alias.as_deref(), Some("X"));
    assert_eq!(ir.set_operations.len(), 1);
    assert_eq!(ir.set_operations[0].operator, SetOperator::UnionDistinct);
    assert_eq!(
        ir.set_operations[0].right.projection[0].alias.as_deref(),
        Some("X")
    );
}

#[test]
fn union_type_mismatch_is_one_typed_terminal_and_no_ir() {
    let source = "RETURN 1 AS x UNION RETURN 'x' AS x";
    let result = Compiler.compile("union-type-mismatch.gql", source, &empty_catalog());

    assert!(result.parse.diagnostics.is_empty());
    assert!(result.statement.is_some());
    assert!(result.analysis.ir.is_none());
    assert_eq!(
        result
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-UNION-PROJECTION-TYPE"]
    );
}

#[test]
fn union_output_alias_mismatch_is_one_typed_terminal_and_no_ir() {
    let result = Compiler.compile(
        "union-alias-mismatch.gql",
        "RETURN 1 AS x UNION RETURN 2 AS y",
        &empty_catalog(),
    );

    assert!(result.parse.diagnostics.is_empty());
    assert!(result.statement.is_some());
    assert!(result.analysis.ir.is_none());
    assert_eq!(
        result
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-UNION-PROJECTION-NAME"]
    );
}

#[test]
fn trailing_union_is_one_parse_terminal_with_zero_ast_or_ir_fallback() {
    let result = Compiler.compile("trailing-union.gql", "RETURN 1 UNION", &empty_catalog());

    assert_eq!(
        result
            .parse
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-PARSE-UNION-SYNTAX"]
    );
    assert!(result.statement.is_none());
    assert!(result.analysis.ir.is_none());
}

#[test]
fn return_projection_alias_is_visible_to_following_order_by() {
    let source = "MATCH (n) RETURN n.name AS display_name ORDER BY display_name";
    let result = Compiler.compile("return-alias-order.gql", source, &empty_catalog());

    assert!(result.parse.diagnostics.is_empty());
    assert!(
        result.analysis.diagnostics.is_empty(),
        "{:?}",
        result.analysis.diagnostics
    );
    assert!(result.analysis.ir.is_some());
}

#[test]
fn graph_clause_after_return_is_typed_and_emits_no_ir() {
    let source = "MATCH (a) RETURN a MATCH (b) RETURN b";
    let result = Compiler.compile("clause-after-return.gql", source, &empty_catalog());

    assert!(result.parse.diagnostics.is_empty());
    assert!(result.analysis.ir.is_none());
    assert_eq!(
        result
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-CLAUSE-AFTER-RETURN"]
    );
}

#[test]
fn every_union_branch_requires_a_return_projection() {
    let source = "MATCH (a) UNION MATCH (b) RETURN b";
    let result = Compiler.compile("union-missing-return.gql", source, &empty_catalog());

    assert!(result.parse.diagnostics.is_empty());
    assert!(result.analysis.ir.is_none());
    assert_eq!(
        result
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-QUERY-BRANCH-MISSING-RETURN"]
    );
}
