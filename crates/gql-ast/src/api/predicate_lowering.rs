//! Lowering owner for ISO postfix predicate tests.

use super::lowering::{lower_expression, significant_node_span};
use super::lowering_support::is_expression_kind;
use super::{ElementIdentityKind, EndpointKind, Expression, TruthValue};
use gql_syntax::{
    Keyword, SyntaxElement, SyntaxElementKind, SyntaxKind, SyntaxNode, Token, TokenKind,
};

pub(super) fn lower_predicate_test<F>(node: &SyntaxNode, mut lower_operand: F) -> Option<Expression>
where
    F: FnMut(&SyntaxNode) -> Option<Expression>,
{
    let mut operand = None;
    let mut negated = false;
    let mut value = None;
    for element in node.children() {
        match element {
            SyntaxElement {
                kind: SyntaxElementKind::Node(child),
            } if operand.is_none() => operand = lower_operand(&child),
            SyntaxElement {
                kind:
                    SyntaxElementKind::Token(Token {
                        kind: TokenKind::Keyword(Keyword::Not),
                        ..
                    }),
            } => negated = true,
            SyntaxElement {
                kind: SyntaxElementKind::Token(token),
            } => {
                value = match token.kind {
                    TokenKind::Keyword(Keyword::True) => Some(TruthValue::True),
                    TokenKind::Keyword(Keyword::False) => Some(TruthValue::False),
                    TokenKind::Keyword(Keyword::UnknownTruth) => Some(TruthValue::Unknown),
                    _ => value,
                };
            }
            _ => {}
        }
    }
    let operand = operand?;
    let span = node.span();
    if node.kind() == SyntaxKind::NullPredicateExpression {
        return Some(Expression::NullPredicate {
            operand: Box::new(operand),
            negated,
            span,
        });
    }
    Some(Expression::TruthPredicate {
        operand: Box::new(operand),
        value: value?,
        negated,
        span,
    })
}

pub(super) fn lower_graph_element_predicate(node: &SyntaxNode, source: &str) -> Option<Expression> {
    let expressions = node
        .children()
        .iter()
        .filter_map(|element| match element {
            SyntaxElement {
                kind: SyntaxElementKind::Node(child),
            } if is_expression_kind(child.kind()) => lower_expression(child, source),
            _ => None,
        })
        .collect::<Vec<_>>();
    let negated = node.children().iter().any(|element| {
        matches!(
            element,
            SyntaxElement {
                kind: SyntaxElementKind::Token(Token {
                    kind: TokenKind::Keyword(Keyword::Not),
                    ..
                }),
            }
        )
    });
    let span = significant_node_span(node);
    match node.kind() {
        SyntaxKind::DirectedPredicateExpression => Some(Expression::DirectedPredicate {
            edge: Box::new(expressions.into_iter().next()?),
            negated,
            span,
        }),
        SyntaxKind::EndpointPredicateExpression => {
            let mut expressions = expressions.into_iter();
            let node_expression = expressions.next()?;
            let edge = expressions.next()?;
            let endpoint = node
                .children()
                .iter()
                .filter_map(|element| match element {
                    SyntaxElement {
                        kind: SyntaxElementKind::Token(token),
                    } => Some(token.text()),
                    _ => None,
                })
                .find_map(|text| {
                    if text.eq_ignore_ascii_case("SOURCE") {
                        Some(EndpointKind::Source)
                    } else if text.eq_ignore_ascii_case("DESTINATION") {
                        Some(EndpointKind::Destination)
                    } else {
                        None
                    }
                })?;
            Some(Expression::EndpointPredicate {
                node: Box::new(node_expression),
                edge: Box::new(edge),
                endpoint,
                negated,
                span,
            })
        }
        SyntaxKind::ElementIdentityPredicateExpression => {
            let kind = node.children().iter().find_map(|element| match element {
                SyntaxElement {
                    kind: SyntaxElementKind::Token(token),
                } if token.text().eq_ignore_ascii_case("ALL_DIFFERENT") => {
                    Some(ElementIdentityKind::AllDifferent)
                }
                SyntaxElement {
                    kind: SyntaxElementKind::Token(token),
                } if token.text().eq_ignore_ascii_case("SAME") => Some(ElementIdentityKind::Same),
                _ => None,
            })?;
            Some(Expression::ElementIdentityPredicate {
                kind,
                elements: expressions,
                span,
            })
        }
        SyntaxKind::PropertyExistsPredicateExpression => {
            let mut expressions = expressions.into_iter();
            let element = expressions.next()?;
            let Expression::Name(property) = expressions.next()? else {
                return None;
            };
            Some(Expression::PropertyExistsPredicate {
                element: Box::new(element),
                property,
                span,
            })
        }
        _ => None,
    }
}
