use crate::Compiler;
use crate::ast::{
    AggregateFunction as AstAggregateFunction, Expression as AstExpression, QueryClause,
    SetQuantifier as AstSetQuantifier, Statement,
};
use crate::catalog::{Catalog, CatalogName};
use crate::ir::{
    AggregateFunction, Expression as IrExpression, SetOperator, SetQuantifier, SortDirection,
};
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
            null_ordering: None,
        }] if name == "RANK"
    ));
    assert_eq!(
        ir.offset,
        Some(crate::ir::NonNegativeIntegerSpecification::Literal(2))
    );
    assert_eq!(
        ir.limit,
        Some(crate::ir::NonNegativeIntegerSpecification::Literal(10))
    );

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
            ..
        } if arguments == &[IrExpression::Binding("N".into())]
    ));
    assert!(matches!(
        ir.order_by.as_slice(),
        [crate::ir::SortKey {
            expression: IrExpression::Binding(name),
            direction: SortDirection::Descending,
            null_ordering: None,
        }] if name == "TOTAL"
    ));
    assert_eq!(
        (ir.offset, ir.limit),
        (
            Some(crate::ir::NonNegativeIntegerSpecification::Literal(1)),
            Some(crate::ir::NonNegativeIntegerSpecification::Literal(10))
        )
    );
}

#[test]
fn iso_aggregate_function_family_crosses_lossless_cst_and_canonical_ir() {
    let source = concat!(
        "MATCH (n) RETURN COUNT(*) AS rows, COUNT(DISTINCT n) AS distinct_nodes, ",
        "AVG(ALL n.score) AS average_score, MAX(n.score) AS maximum_score, ",
        "MIN(n.score) AS minimum_score, SUM(n.score) AS total_score, ",
        "COLLECT_LIST(n.score) AS scores, STDDEV_SAMP(n.score) AS sample_deviation, ",
        "STDDEV_POP(n.score) AS population_deviation, ",
        "PERCENTILE_CONT(DISTINCT n.score, 0.5) AS continuous_median, ",
        "PERCENTILE_DISC(ALL n.score, 1) AS discrete_maximum"
    );
    let result = Compiler.compile("iso-aggregate-family.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "aggregate grammar must be admitted losslessly: {:?}",
        result.parse.diagnostics
    );
    assert!(
        result.analysis.diagnostics.is_empty(),
        "aggregate semantics must be admitted: {:?}",
        result.analysis.diagnostics
    );
    assert!(
        result.analysis.ir.is_some(),
        "aggregate IR must be complete"
    );

    let mut cst_nodes = Vec::new();
    node_receipt(&result.parse.tree.root(), source, &mut cst_nodes);
    assert_eq!(
        cst_nodes
            .iter()
            .filter(|(kind, _)| *kind == SyntaxKind::AggregateFunctionExpression)
            .count(),
        11
    );
    assert_eq!(
        cst_nodes
            .iter()
            .filter(|(kind, _)| *kind == SyntaxKind::SetQuantifier)
            .count(),
        4
    );

    let Some(Statement::Query(query)) = &result.statement else {
        panic!("aggregate query AST must be admitted");
    };
    let Some(QueryClause::Return { projections, .. }) = query.clauses.last() else {
        panic!("aggregate RETURN clause must be retained");
    };
    assert_eq!(projections.len(), 11);
    let ast_functions = projections
        .iter()
        .map(|projection| match &projection.expression {
            AstExpression::AggregateCall {
                function,
                quantifier,
                arguments,
                count_star,
                span,
            } => {
                assert!(source[span.start as usize..span.end as usize].contains('('));
                (*function, *quantifier, arguments.len(), *count_star)
            }
            expression => panic!("expected typed aggregate call, got {expression:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        ast_functions,
        vec![
            (AstAggregateFunction::Count, None, 0, true),
            (
                AstAggregateFunction::Count,
                Some(AstSetQuantifier::Distinct),
                1,
                false,
            ),
            (
                AstAggregateFunction::Average,
                Some(AstSetQuantifier::All),
                1,
                false,
            ),
            (AstAggregateFunction::Maximum, None, 1, false),
            (AstAggregateFunction::Minimum, None, 1, false),
            (AstAggregateFunction::Sum, None, 1, false),
            (AstAggregateFunction::CollectList, None, 1, false),
            (
                AstAggregateFunction::StandardDeviationSample,
                None,
                1,
                false,
            ),
            (
                AstAggregateFunction::StandardDeviationPopulation,
                None,
                1,
                false,
            ),
            (
                AstAggregateFunction::PercentileContinuous,
                Some(AstSetQuantifier::Distinct),
                2,
                false,
            ),
            (
                AstAggregateFunction::PercentileDiscrete,
                Some(AstSetQuantifier::All),
                2,
                false,
            ),
        ]
    );

    let ir = result.analysis.ir.expect("aggregate IR");
    let ir_functions = ir
        .projection
        .iter()
        .map(|projection| match &projection.expression {
            IrExpression::Aggregate {
                function,
                quantifier,
                arguments,
                count_star,
            } => (*function, *quantifier, arguments.len(), *count_star),
            expression => panic!("expected canonical aggregate IR, got {expression:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(ir_functions[0], (AggregateFunction::Count, None, 0, true));
    assert_eq!(
        ir_functions[1],
        (
            AggregateFunction::Count,
            Some(SetQuantifier::Distinct),
            1,
            false,
        )
    );
    assert_eq!(
        ir_functions[9],
        (
            AggregateFunction::PercentileContinuous,
            Some(SetQuantifier::Distinct),
            2,
            false,
        )
    );
    assert_eq!(
        ir_functions[10],
        (
            AggregateFunction::PercentileDiscrete,
            Some(SetQuantifier::All),
            2,
            false,
        )
    );
}

#[test]
fn malformed_and_non_numeric_aggregates_fail_closed_once() {
    let malformed = Compiler.compile(
        "malformed-percentile.gql",
        "MATCH (n) RETURN PERCENTILE_CONT(n.score) AS median",
        &empty_catalog(),
    );
    assert!(malformed.statement.is_none());
    assert!(malformed.analysis.ir.is_none());
    assert_eq!(
        malformed
            .parse
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-PARSE-AGGREGATE-FUNCTION-SYNTAX"]
    );

    let non_numeric = Compiler.compile(
        "non-numeric-aggregate.gql",
        "MATCH (n) RETURN SUM('Ada') AS invalid_total",
        &empty_catalog(),
    );
    assert!(non_numeric.statement.is_some());
    assert!(non_numeric.analysis.ir.is_none());
    assert_eq!(
        non_numeric
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-AGGREGATE-NUMERIC-OPERAND"]
    );
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
