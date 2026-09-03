//! Pattern-local expression and property lowering.
#![forbid(unsafe_code)]

use super::identifier_lowering::identifier_from_token;
use super::lowering::lower_expression;
use super::lowering_support::is_expression_kind;
use super::{Expression, PropertyConstraint};
use gql_syntax::{SyntaxElement, SyntaxElementKind, SyntaxKind, SyntaxNode, TokenKind};

pub(super) fn lower_inline_where_predicate(node: &SyntaxNode, source: &str) -> Option<Expression> {
    node.children().iter().find_map(|element| {
        let SyntaxElementKind::Node(inline_where) = &element.kind else {
            return None;
        };
        if inline_where.kind() != SyntaxKind::InlineWhereClause {
            return None;
        }
        inline_where.children().iter().find_map(|child| {
            let SyntaxElementKind::Node(expression) = &child.kind else {
                return None;
            };
            is_expression_kind(expression.kind())
                .then(|| lower_expression(expression, source))
                .flatten()
        })
    })
}

pub(super) fn lower_pattern_properties(node: &SyntaxNode, source: &str) -> Vec<PropertyConstraint> {
    let mut properties = Vec::new();
    for element in node.children() {
        let SyntaxElementKind::Node(property_map) = &element.kind else {
            continue;
        };
        if property_map.kind() != SyntaxKind::PropertyMap {
            continue;
        }
        for map_element in property_map.children() {
            let SyntaxElementKind::Node(entry) = &map_element.kind else {
                continue;
            };
            if entry.kind() != SyntaxKind::PropertyEntry {
                continue;
            }
            let key = entry.children().iter().find_map(|child| match child {
                SyntaxElement {
                    kind: SyntaxElementKind::Token(token),
                } if matches!(token.kind, TokenKind::Identifier | TokenKind::Keyword(_)) => {
                    Some(identifier_from_token(token, source))
                }
                _ => None,
            });
            let value = entry.children().iter().find_map(|child| match child {
                SyntaxElement {
                    kind: SyntaxElementKind::Node(expression),
                } if is_expression_kind(expression.kind()) => lower_expression(expression, source),
                _ => None,
            });
            if let (Some(key), Some(value)) = (key, value) {
                properties.push(PropertyConstraint {
                    key,
                    value,
                    span: entry.span(),
                });
            }
        }
    }
    properties
}
