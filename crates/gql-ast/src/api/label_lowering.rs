//! Typed lowering for label predicates and label algebra.
#![forbid(unsafe_code)]

use super::identifier_lowering::identifier_from_token;
use super::lowering::lower_expression;
use super::lowering_support::is_expression_kind;
use super::{Expression, LabelExpression};
use gql_syntax::{Keyword, SyntaxElementKind, SyntaxKind, SyntaxNode, TokenKind};

pub(super) fn lower_label_predicate(node: &SyntaxNode, source: &str) -> Option<Expression> {
    let children = node.children();
    let operand = children.iter().find_map(|element| {
        let SyntaxElementKind::Node(child) = &element.kind else {
            return None;
        };
        is_expression_kind(child.kind())
            .then(|| lower_expression(child, source))
            .flatten()
    })?;
    let label = children.iter().find_map(|element| {
        let SyntaxElementKind::Node(child) = &element.kind else {
            return None;
        };
        is_label_expression_kind(child.kind())
            .then(|| lower_label_expression(child, source))
            .flatten()
    })?;
    let negated = children.iter().any(|element| {
        matches!(
            &element.kind,
            SyntaxElementKind::Token(token)
                if token.kind == TokenKind::Keyword(Keyword::Not)
        )
    });
    Some(Expression::IsLabeled {
        operand: Box::new(operand),
        label,
        negated,
        span: node.span(),
    })
}

fn lower_label_expression(node: &SyntaxNode, source: &str) -> Option<LabelExpression> {
    match node.kind() {
        SyntaxKind::LabelNameExpression => node.children().iter().find_map(|element| {
            let SyntaxElementKind::Token(token) = &element.kind else {
                return None;
            };
            (token.kind == TokenKind::Identifier)
                .then(|| LabelExpression::Name(identifier_from_token(token, source)))
        }),
        SyntaxKind::LabelWildcardExpression => Some(LabelExpression::Wildcard),
        SyntaxKind::LabelNotExpression => label_children(node, source)
            .into_iter()
            .next()
            .map(|operand| LabelExpression::Not(Box::new(operand))),
        SyntaxKind::LabelAndExpression => {
            let mut children = label_children(node, source).into_iter();
            Some(LabelExpression::And(
                Box::new(children.next()?),
                Box::new(children.next()?),
            ))
        }
        SyntaxKind::LabelOrExpression => {
            let mut children = label_children(node, source).into_iter();
            Some(LabelExpression::Or(
                Box::new(children.next()?),
                Box::new(children.next()?),
            ))
        }
        _ => None,
    }
}

fn label_children(node: &SyntaxNode, source: &str) -> Vec<LabelExpression> {
    node.children()
        .iter()
        .filter_map(|element| {
            let SyntaxElementKind::Node(child) = &element.kind else {
                return None;
            };
            is_label_expression_kind(child.kind())
                .then(|| lower_label_expression(child, source))
                .flatten()
        })
        .collect()
}

fn is_label_expression_kind(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::LabelNameExpression
            | SyntaxKind::LabelWildcardExpression
            | SyntaxKind::LabelNotExpression
            | SyntaxKind::LabelAndExpression
            | SyntaxKind::LabelOrExpression
    )
}
