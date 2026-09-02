//! Collection membership semantic contracts.

use super::{BinaryOperator, Expression, IrExpression, QueryClause, Span, analyze, catalog, query};

#[test]
fn collection_membership_requires_list_rhs_and_lowers_to_ir() {
    let result = analyze(
        &query(vec![
            return_clause! { expressions: vec![Expression::Binary {
                    operator: BinaryOperator::In,
                    left: Box::new(Expression::Integer(1, Span::default())),
                    right: Box::new(Expression::List(
                        vec![Expression::Integer(1, Span::default())],
                        Span::default(),
                    )),
                }],
            },
        ]),
        &catalog(),
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert!(matches!(
        result.ir.expect("membership IR").projection.as_slice(),
        [gql_ir::Projection {
            expression: IrExpression::Binary {
                operator: gql_ir::BinaryOperator::In,
                ..
            },
            ..
        }]
    ));

    let invalid = analyze(
        &query(vec![
            return_clause! { expressions: vec![Expression::Binary {
                    operator: BinaryOperator::In,
                    left: Box::new(Expression::Integer(1, Span::default())),
                    right: Box::new(Expression::Integer(2, Span::default())),
                }],
            },
        ]),
        &catalog(),
    );
    assert!(invalid.ir.is_none());
    assert!(
        invalid
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-SEMA-NON-LIST-MEMBERSHIP")
    );
}
