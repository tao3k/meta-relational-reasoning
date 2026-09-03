use crate::Compiler;
use crate::ast::{
    BinaryOperator as AstBinaryOperator, Expression as AstExpression,
    PropertyValueTypeForm as AstPropertyValueTypeForm, QueryClause, Statement,
    TruthValue as AstTruthValue, UnaryOperator as AstUnaryOperator,
};
use crate::catalog::{Catalog, CatalogName};
use crate::ir::{
    BinaryOperator as IrBinaryOperator, DeclaredValueTypeForm, Expression as IrExpression,
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

#[test]
fn truth_and_null_predicates_cross_lossless_cst_ast_and_canonical_ir() {
    let source = "MATCH (n) WHERE n.deleted IS NULL OR n.active IS NOT FALSE RETURN n.deleted IS NOT NULL, n.active IS TRUE, NULL IS UNKNOWN";
    let result = Compiler.compile("truth-null-predicates.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "{:?}",
        result.parse.diagnostics
    );
    assert_eq!(
        count_kind(
            &result.parse.tree.root(),
            SyntaxKind::NullPredicateExpression
        ),
        2
    );
    assert_eq!(
        count_kind(
            &result.parse.tree.root(),
            SyntaxKind::TruthPredicateExpression
        ),
        3
    );
    let Some(Statement::Query(query)) = &result.statement else {
        panic!("statement is query");
    };
    let Some(QueryClause::Return { projections, .. }) = query.clauses.last() else {
        panic!("RETURN clause exists");
    };
    assert_eq!(projections.len(), 3);
    assert!(matches!(
        &projections[0].expression,
        AstExpression::NullPredicate {
            operand,
            negated: true,
            span,
        } if matches!(operand.as_ref(), AstExpression::PropertyAccess { .. })
            && &source[span.start as usize..span.end as usize] == "n.deleted IS NOT NULL"
    ));
    assert!(matches!(
        &projections[1].expression,
        AstExpression::TruthPredicate {
            operand,
            value: AstTruthValue::True,
            negated: false,
            span,
        } if matches!(operand.as_ref(), AstExpression::PropertyAccess { .. })
            && &source[span.start as usize..span.end as usize] == "n.active IS TRUE"
    ));
    assert!(matches!(
        &projections[2].expression,
        AstExpression::TruthPredicate {
            operand,
            value: AstTruthValue::Unknown,
            negated: false,
            span,
        } if matches!(operand.as_ref(), AstExpression::Null(_))
            && &source[span.start as usize..span.end as usize] == "NULL IS UNKNOWN"
    ));
    assert!(result.analysis.diagnostics.is_empty());
    let ir = result.analysis.ir.expect("canonical predicate IR");
    assert!(matches!(
        ir.filters.as_slice(),
        [IrExpression::Binary {
            operator: IrBinaryOperator::Or,
            left,
            right,
        }] if matches!(left.as_ref(), IrExpression::Unary {
            operator: IrUnaryOperator::IsNull,
            ..
        }) && matches!(right.as_ref(), IrExpression::Unary {
            operator: IrUnaryOperator::IsNotFalse,
            ..
        })
    ));
    assert!(matches!(
        ir.projection.as_slice(),
        [
            crate::ir::Projection {
                expression: IrExpression::Unary {
                    operator: IrUnaryOperator::IsNotNull,
                    ..
                },
                ..
            },
            crate::ir::Projection {
                expression: IrExpression::Unary {
                    operator: IrUnaryOperator::IsTrue,
                    ..
                },
                ..
            },
            crate::ir::Projection {
                expression: IrExpression::Unary {
                    operator: IrUnaryOperator::IsUnknown,
                    ..
                },
                ..
            }
        ]
    ));
}

#[test]
fn malformed_and_non_boolean_truth_predicates_fail_closed_once() {
    let malformed = Compiler.compile(
        "malformed-truth-predicate.gql",
        "RETURN TRUE IS NOT",
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
        ["GQL-PARSE-PREDICATE-TEST-SYNTAX"]
    );

    let non_boolean = Compiler.compile(
        "non-boolean-truth-predicate.gql",
        "RETURN 42 IS TRUE",
        &empty_catalog(),
    );
    assert!(non_boolean.parse.diagnostics.is_empty());
    assert!(non_boolean.analysis.ir.is_none());
    assert_eq!(
        non_boolean
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-TRUTH-PREDICATE-OPERAND"]
    );
}

#[test]
fn null_predicate_requires_a_value_expression_primary() {
    let valid = Compiler.compile(
        "parenthesized-null-predicate.gql",
        "RETURN (1 + 2) IS NULL",
        &empty_catalog(),
    );
    assert!(valid.parse.diagnostics.is_empty());
    assert!(valid.analysis.ir.is_some());

    let invalid = Compiler.compile(
        "composite-null-predicate.gql",
        "RETURN 1 + 2 IS NULL",
        &empty_catalog(),
    );
    assert!(invalid.statement.is_none());
    assert!(invalid.analysis.ir.is_none());
    assert_eq!(
        invalid
            .parse
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-PARSE-NULL-PREDICATE-OPERAND"]
    );
}

#[test]
fn iso_value_type_predicate_family_crosses_lossless_cst_ast_and_canonical_ir() {
    let source = "MATCH (n) RETURN n.score IS TYPED INT64, n.tags IS NOT :: LIST<STRING>, (1 + 2) IS TYPED INT64";
    let result = Compiler.compile("iso-value-type-predicates.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse: {:?}",
        result.parse.diagnostics
    );
    assert_eq!(
        count_kind(
            &result.parse.tree.root(),
            SyntaxKind::ValueTypePredicateExpression,
        ),
        3,
    );
    assert_eq!(
        count_kind(&result.parse.tree.root(), SyntaxKind::PropertyValueType),
        4,
    );
    let Some(Statement::Query(query)) = &result.statement else {
        panic!("statement is query");
    };
    let Some(QueryClause::Return { projections, .. }) = query.clauses.last() else {
        panic!("RETURN clause exists");
    };
    assert_eq!(projections.len(), 3);
    assert!(matches!(
        &projections[0].expression,
        AstExpression::ValueTypePredicate {
            operand,
            value_type,
            negated: false,
            span,
        } if matches!(operand.as_ref(), AstExpression::PropertyAccess { .. })
            && matches!(&value_type.form, AstPropertyValueTypeForm::Named { name, .. } if name == "INT64")
            && &source[span.start as usize..span.end as usize] == "n.score IS TYPED INT64"
    ));
    assert!(matches!(
        &projections[1].expression,
        AstExpression::ValueTypePredicate {
            value_type,
            negated: true,
            span,
            ..
        } if matches!(&value_type.form, AstPropertyValueTypeForm::List { element: Some(element), .. }
            if matches!(&element.form, AstPropertyValueTypeForm::Named { name, .. } if name == "STRING"))
            && &source[span.start as usize..span.end as usize] == "n.tags IS NOT :: LIST<STRING>"
    ));
    assert!(matches!(
        &projections[2].expression,
        AstExpression::ValueTypePredicate { operand, negated: false, .. }
            if matches!(operand.as_ref(), AstExpression::Binary { .. })
    ));
    assert!(
        result.analysis.diagnostics.is_empty(),
        "sema: {:?}",
        result.analysis.diagnostics
    );
    let ir = result
        .analysis
        .ir
        .expect("canonical value-type predicate IR");
    assert!(matches!(
        ir.projection.as_slice(),
        [
            crate::ir::Projection {
                expression: IrExpression::IsTyped {
                    value_type: crate::ir::DeclaredValueType {
                        form: DeclaredValueTypeForm::Named { name, .. },
                        ..
                    },
                    negated: false,
                    ..
                },
                ..
            },
            crate::ir::Projection {
                expression: IrExpression::IsTyped {
                    value_type: crate::ir::DeclaredValueType {
                        form: DeclaredValueTypeForm::List { element: Some(element), .. },
                        ..
                    },
                    negated: true,
                    ..
                },
                ..
            },
            crate::ir::Projection {
                expression: IrExpression::IsTyped { negated: false, .. },
                ..
            },
        ] if name == "INT64"
            && matches!(&element.form, DeclaredValueTypeForm::Named { name, .. } if name == "STRING")
    ));
}

#[test]
fn malformed_value_type_predicates_fail_closed_once() {
    let malformed_marker = Compiler.compile(
        "malformed-value-type-marker.gql",
        "RETURN 42 IS : INT64",
        &empty_catalog(),
    );
    assert!(malformed_marker.statement.is_none());
    assert!(malformed_marker.analysis.ir.is_none());
    assert_eq!(
        malformed_marker
            .parse
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-PARSE-VALUE-TYPE-PREDICATE-SYNTAX"],
    );

    let missing_type = Compiler.compile(
        "missing-value-type-predicate.gql",
        "RETURN 42 IS TYPED",
        &empty_catalog(),
    );
    assert!(missing_type.statement.is_none());
    assert!(missing_type.analysis.ir.is_none());
    assert_eq!(
        missing_type
            .parse
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-PARSE-VALUE-TYPE-PREDICATE-SYNTAX"],
    );

    let composite_operand = Compiler.compile(
        "composite-value-type-predicate.gql",
        "RETURN 1 + 2 IS TYPED INT64",
        &empty_catalog(),
    );
    assert!(composite_operand.statement.is_none());
    assert!(composite_operand.analysis.ir.is_none());
    assert_eq!(
        composite_operand
            .parse
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-PARSE-VALUE-TYPE-PREDICATE-OPERAND"],
    );

    let invalid_type = Compiler.compile(
        "invalid-value-type-predicate.gql",
        "RETURN 'Ada' IS TYPED STRING(5, 2)",
        &empty_catalog(),
    );
    assert!(invalid_type.parse.diagnostics.is_empty());
    assert!(invalid_type.analysis.ir.is_none());
    assert_eq!(
        invalid_type
            .analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SEMA-VALUE-TYPE-LENGTH-RANGE"],
    );
}
