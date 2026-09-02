use crate::Compiler;
use crate::ast::{
    BinaryOperator as AstBinaryOperator, Expression as AstExpression,
    LabelExpression as AstLabelExpression, QueryClause, Statement,
};
use crate::catalog::{Catalog, CatalogName};
use crate::ir::{
    BinaryOperator as IrBinaryOperator, Expression as IrExpression,
    LabelExpression as IrLabelExpression,
};
use crate::syntax::{SyntaxElementKind, SyntaxKind, SyntaxNode};

fn empty_catalog() -> Catalog {
    Catalog::new(
        CatalogName("property-label-expression-contract".into()),
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
fn property_access_and_label_algebra_reach_the_same_canonical_ir() {
    let source = "MATCH (n:Person) WHERE n.name = 'Ada' AND n IS LABELED :Person&!Inactive|VIP RETURN n.name";
    let result = Compiler.compile("property-label-expression.gql", source, &empty_catalog());

    assert!(
        result.parse.diagnostics.is_empty(),
        "{:?}",
        result.parse.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    let root = result.parse.tree.root();
    assert_eq!(count_kind(&root, SyntaxKind::LabelPredicateExpression), 1);
    assert_eq!(count_kind(&root, SyntaxKind::LabelOrExpression), 1);
    assert_eq!(count_kind(&root, SyntaxKind::LabelAndExpression), 1);
    assert_eq!(count_kind(&root, SyntaxKind::LabelNotExpression), 1);

    let Some(Statement::Query(query)) = &result.statement else {
        panic!("statement is query");
    };
    let Some(QueryClause::Where { expression, .. }) = query.clauses.get(1) else {
        panic!("WHERE clause exists");
    };
    let AstExpression::Binary {
        operator: AstBinaryOperator::And,
        left,
        right,
    } = expression
    else {
        panic!("property comparison AND label predicate");
    };
    assert!(matches!(
        left.as_ref(),
        AstExpression::Binary {
            operator: AstBinaryOperator::Equals,
            left,
            ..
        } if matches!(left.as_ref(), AstExpression::PropertyAccess { property, .. } if property.text == "name")
    ));
    let AstExpression::IsLabeled {
        operand,
        label,
        negated,
        span,
    } = right.as_ref()
    else {
        panic!("typed label predicate exists");
    };
    assert!(!negated);
    assert!(matches!(operand.as_ref(), AstExpression::Name(name) if name.text == "n"));
    assert_eq!(
        &source[span.start as usize..span.end as usize],
        "n IS LABELED :Person&!Inactive|VIP"
    );
    assert!(matches!(
        label,
        AstLabelExpression::Or(left, right)
            if matches!(left.as_ref(), AstLabelExpression::And(left, right)
                if matches!(left.as_ref(), AstLabelExpression::Name(name) if name.text == "Person")
                    && matches!(right.as_ref(), AstLabelExpression::Not(inner)
                        if matches!(inner.as_ref(), AstLabelExpression::Name(name) if name.text == "Inactive")))
                && matches!(right.as_ref(), AstLabelExpression::Name(name) if name.text == "VIP")
    ));

    assert!(
        result.analysis.diagnostics.is_empty(),
        "{:?}",
        result.analysis.diagnostics
    );
    let filter = &result.analysis.ir.expect("canonical IR").filters[0];
    let IrExpression::Binary {
        operator: IrBinaryOperator::And,
        right,
        ..
    } = filter
    else {
        panic!("canonical boolean filter");
    };
    assert!(matches!(
        right.as_ref(),
        IrExpression::IsLabeled {
            operand,
            label: IrLabelExpression::Or(_, _),
            negated: false,
        } if matches!(operand.as_ref(), IrExpression::Binding(name) if name == "N")
    ));
}

#[test]
fn negated_wildcard_label_predicate_is_first_class() {
    let source = "MATCH (n) WHERE n IS NOT LABELED :% RETURN n";
    let result = Compiler.compile("negated-wildcard-label.gql", source, &empty_catalog());

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
    assert!(matches!(
        &result.analysis.ir.expect("IR").filters[0],
        IrExpression::IsLabeled {
            label: IrLabelExpression::Wildcard,
            negated: true,
            ..
        }
    ));
}

#[test]
fn malformed_label_algebra_is_exactly_once_typed_and_emits_no_ir() {
    let source = "MATCH (n) WHERE n IS LABELED :Person& RETURN n";
    let result = Compiler.compile("malformed-label.gql", source, &empty_catalog());

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
        ["GQL-PARSE-LABEL-EXPRESSION"]
    );
}

#[test]
fn label_predicate_rejects_non_graph_values_without_ir() {
    let result = Compiler.compile(
        "non-graph-label.gql",
        "MATCH (n) WHERE 1 IS LABELED :Person RETURN n",
        &empty_catalog(),
    );

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
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-LABEL-PREDICATE-NON-ELEMENT"]
    );
}
