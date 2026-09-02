use crate::Compiler;
use crate::ast::{
    BinaryOperator as AstBinaryOperator, Expression as AstExpression, QueryClause, Statement,
    UnaryOperator as AstUnaryOperator,
};
use crate::catalog::{Catalog, CatalogName};
use crate::ir::{
    BinaryOperator as IrBinaryOperator, Expression as IrExpression,
    UnaryOperator as IrUnaryOperator,
};
use crate::syntax::{SyntaxElementKind, SyntaxKind, SyntaxNode};

fn empty_catalog() -> Catalog {
    Catalog::new(
        CatalogName("expression-language-contract".into()),
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
fn iso_operator_precedence_survives_lossless_cst_ast_and_canonical_ir() {
    let source =
        "MATCH (n) RETURN TRUE OR FALSE XOR TRUE AND NOT FALSE, 'a' || 'b', -1 + +2 * 3, 1 <> 2";
    let result = Compiler.compile("iso-operator-precedence.gql", source, &empty_catalog());

    assert!(
        result.parse.diagnostics.is_empty(),
        "parse: {:?}",
        result.parse.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(
        count_kind(&result.parse.tree.root(), SyntaxKind::BinaryExpression),
        7
    );
    assert_eq!(
        count_kind(&result.parse.tree.root(), SyntaxKind::UnaryExpression),
        3
    );

    let Some(Statement::Query(query)) = &result.statement else {
        panic!("statement is query");
    };
    let Some(QueryClause::Return { projections, .. }) = query.clauses.last() else {
        panic!("RETURN clause exists");
    };
    assert_eq!(projections.len(), 4);
    assert!(matches!(
        &projections[0].expression,
        AstExpression::Binary {
            operator: AstBinaryOperator::Or,
            left,
            right,
        } if matches!(left.as_ref(), AstExpression::Boolean(true, _))
            && matches!(right.as_ref(), AstExpression::Binary {
                operator: AstBinaryOperator::Xor,
                left,
                right,
            } if matches!(left.as_ref(), AstExpression::Boolean(false, _))
                && matches!(right.as_ref(), AstExpression::Binary {
                    operator: AstBinaryOperator::And,
                    left,
                    right,
                } if matches!(left.as_ref(), AstExpression::Boolean(true, _))
                    && matches!(right.as_ref(), AstExpression::Unary {
                        operator: AstUnaryOperator::Not,
                        operand,
                    } if matches!(operand.as_ref(), AstExpression::Boolean(false, _)))))
    ));
    assert!(matches!(
        &projections[1].expression,
        AstExpression::Binary {
            operator: AstBinaryOperator::Concatenate,
            left,
            right,
        } if matches!(left.as_ref(), AstExpression::String(literal) if literal.value == "a")
            && matches!(right.as_ref(), AstExpression::String(literal) if literal.value == "b")
    ));
    assert!(matches!(
        &projections[2].expression,
        AstExpression::Binary {
            operator: AstBinaryOperator::Add,
            left,
            right,
        } if matches!(left.as_ref(), AstExpression::Unary {
                operator: AstUnaryOperator::Negate,
                operand,
            } if matches!(operand.as_ref(), AstExpression::Integer(1, _)))
            && matches!(right.as_ref(), AstExpression::Binary {
                operator: AstBinaryOperator::Multiply,
                left,
                right,
            } if matches!(left.as_ref(), AstExpression::Unary {
                    operator: AstUnaryOperator::Plus,
                    operand,
                } if matches!(operand.as_ref(), AstExpression::Integer(2, _)))
                && matches!(right.as_ref(), AstExpression::Integer(3, _)))
    ));
    assert!(matches!(
        &projections[3].expression,
        AstExpression::Binary {
            operator: AstBinaryOperator::NotEquals,
            left,
            right,
        } if matches!(left.as_ref(), AstExpression::Integer(1, _))
            && matches!(right.as_ref(), AstExpression::Integer(2, _))
    ));

    assert!(
        result.analysis.diagnostics.is_empty(),
        "sema: {:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("canonical expression IR");
    assert_eq!(ir.projection.len(), 4);
    assert!(matches!(
        &ir.projection[0].expression,
        IrExpression::Binary {
            operator: IrBinaryOperator::Or,
            right,
            ..
        } if matches!(right.as_ref(), IrExpression::Binary {
            operator: IrBinaryOperator::Xor,
            right,
            ..
        } if matches!(right.as_ref(), IrExpression::Binary {
            operator: IrBinaryOperator::And,
            right,
            ..
        } if matches!(right.as_ref(), IrExpression::Unary {
            operator: IrUnaryOperator::Not,
            ..
        })))
    ));
    assert!(matches!(
        &ir.projection[1].expression,
        IrExpression::Binary {
            operator: IrBinaryOperator::Concatenate,
            ..
        }
    ));
    assert!(matches!(
        &ir.projection[2].expression,
        IrExpression::Binary {
            operator: IrBinaryOperator::Add,
            left,
            right,
        } if matches!(left.as_ref(), IrExpression::Unary {
                operator: IrUnaryOperator::Negate,
                ..
            })
            && matches!(right.as_ref(), IrExpression::Binary {
                operator: IrBinaryOperator::Multiply,
                left,
                ..
            } if matches!(left.as_ref(), IrExpression::Unary {
                operator: IrUnaryOperator::Plus,
                ..
            }))
    ));
    assert!(matches!(
        &ir.projection[3].expression,
        IrExpression::Binary {
            operator: IrBinaryOperator::NotEquals,
            ..
        }
    ));
}

#[test]
fn non_boolean_xor_is_typed_and_emits_no_ir() {
    let result = Compiler.compile(
        "non-boolean-xor.gql",
        "MATCH (n) RETURN TRUE XOR 1",
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
        ["GQL-SEMA-NON-BOOLEAN-LOGIC"]
    );
}

#[test]
fn non_string_concatenation_is_typed_and_emits_no_ir() {
    let result = Compiler.compile(
        "non-string-concatenation.gql",
        "MATCH (n) RETURN 'Ada' || 42",
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
        ["GQL-SEMA-NON-STRING-CONCATENATION"]
    );
}

#[test]
fn non_iso_not_equals_spelling_is_typed_and_emits_no_statement_or_ir() {
    let source = "MATCH (n) RETURN 1 != 2";
    let result = Compiler.compile("non-iso-not-equals.gql", source, &empty_catalog());

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
        ["GQL-PARSE-NON-ISO-OPERATOR"]
    );
}
