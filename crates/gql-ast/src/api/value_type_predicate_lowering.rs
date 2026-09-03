//! AST lowering owner for ISO GQL value-type predicates.
#![forbid(unsafe_code)]

use super::Expression;
use super::data_management_lowering::lower_property_value_type;
use super::lowering::{lower_expression, significant_node_span, syntax_node, syntax_tokens};
use super::lowering_support::is_expression_kind;
use gql_syntax::{Keyword, SyntaxKind, SyntaxNode, TokenKind};

pub(super) fn lower_value_type_predicate(node: &SyntaxNode, source: &str) -> Option<Expression> {
    let operand = node.children().iter().find_map(|element| {
        let child = syntax_node(element)?;
        is_expression_kind(child.kind())
            .then(|| lower_expression(child, source))
            .flatten()
    })?;
    let value_type = node.children().iter().find_map(|element| {
        let child = syntax_node(element)?;
        (child.kind() == SyntaxKind::PropertyValueType)
            .then(|| lower_property_value_type(child, source))
            .flatten()
    })?;
    let negated =
        syntax_tokens(node.children()).any(|token| token.kind == TokenKind::Keyword(Keyword::Not));
    Some(Expression::ValueTypePredicate {
        operand: Box::new(operand),
        value_type,
        negated,
        span: significant_node_span(node),
    })
}
