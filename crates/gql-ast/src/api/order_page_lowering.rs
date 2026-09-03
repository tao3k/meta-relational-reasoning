//! Typed CST-to-AST lowering for ISO ordering and pagination.
#![forbid(unsafe_code)]

use super::lowering_support::is_expression_kind;
use super::{
    DynamicParameterReference, Expression, NonNegativeIntegerSpecification, NullOrdering,
    ParameterNameForm, SortDirection, SortKey,
};
use gql_syntax::{
    ParameterNameForm as SyntaxParameterNameForm, SyntaxElementKind, SyntaxKind, SyntaxNode,
    TokenKind, decode_parameter_reference,
};

pub(super) fn lower_non_negative_integer_specification(
    node: &SyntaxNode,
) -> Option<NonNegativeIntegerSpecification> {
    let specification = node.children().into_iter().find_map(|element| {
        let SyntaxElementKind::Node(child) = element.kind else {
            return None;
        };
        (child.kind() == SyntaxKind::NonNegativeIntegerSpecification).then_some(child)
    })?;
    specification.children().into_iter().find_map(|element| {
        let SyntaxElementKind::Token(token) = element.kind else {
            return None;
        };
        match token.kind {
            TokenKind::Number => token
                .text()
                .parse()
                .ok()
                .map(NonNegativeIntegerSpecification::Literal),
            TokenKind::DynamicParameter => {
                let decoded = decode_parameter_reference(token.text())?;
                Some(NonNegativeIntegerSpecification::Parameter(
                    DynamicParameterReference {
                        name: decoded.name.into_owned(),
                        form: match decoded.form {
                            SyntaxParameterNameForm::Extended => ParameterNameForm::Extended,
                            SyntaxParameterNameForm::Delimited => ParameterNameForm::Delimited,
                        },
                        span: token.span,
                    },
                ))
            }
            _ => None,
        }
    })
}

pub(super) fn lower_order_by_clause(
    node: &SyntaxNode,
    source: &str,
    lower_expression: fn(&SyntaxNode, &str) -> Option<Expression>,
) -> Vec<SortKey> {
    node.children()
        .into_iter()
        .filter_map(|element| {
            let SyntaxElementKind::Node(specification) = element.kind else {
                return None;
            };
            if specification.kind() != SyntaxKind::SortSpecification {
                return None;
            }
            let expression = specification.children().into_iter().find_map(|element| {
                let SyntaxElementKind::Node(child) = element.kind else {
                    return None;
                };
                is_expression_kind(child.kind())
                    .then(|| lower_expression(&child, source))
                    .flatten()
            })?;
            Some(SortKey {
                expression,
                direction: lower_sort_direction(&specification),
                null_ordering: lower_null_ordering(&specification),
            })
        })
        .collect()
}

fn lower_sort_direction(specification: &SyntaxNode) -> Option<SortDirection> {
    specification.children().into_iter().find_map(|element| {
        let SyntaxElementKind::Node(child) = element.kind else {
            return None;
        };
        if child.kind() != SyntaxKind::OrderingSpecification {
            return None;
        }
        child.children().into_iter().find_map(|element| {
            let SyntaxElementKind::Token(token) = element.kind else {
                return None;
            };
            if token.text().eq_ignore_ascii_case("DESC")
                || token.text().eq_ignore_ascii_case("DESCENDING")
            {
                Some(SortDirection::Descending)
            } else if token.text().eq_ignore_ascii_case("ASC")
                || token.text().eq_ignore_ascii_case("ASCENDING")
            {
                Some(SortDirection::Ascending)
            } else {
                None
            }
        })
    })
}

fn lower_null_ordering(specification: &SyntaxNode) -> Option<NullOrdering> {
    specification.children().into_iter().find_map(|element| {
        let SyntaxElementKind::Node(child) = element.kind else {
            return None;
        };
        if child.kind() != SyntaxKind::NullOrdering {
            return None;
        }
        child.children().into_iter().find_map(|element| {
            let SyntaxElementKind::Token(token) = element.kind else {
                return None;
            };
            if token.text().eq_ignore_ascii_case("FIRST") {
                Some(NullOrdering::First)
            } else if token.text().eq_ignore_ascii_case("LAST") {
                Some(NullOrdering::Last)
            } else {
                None
            }
        })
    })
}
