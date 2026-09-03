//! Typed lowering for the Gerbil-owned ISO aggregate function family.
#![forbid(unsafe_code)]

use super::lowering::{lower_expression, significant_node_span};
use super::lowering_support::is_expression_kind;
use super::{AggregateFunction, Expression, SetQuantifier};
use gql_syntax::{Keyword, SyntaxElementKind, SyntaxKind, SyntaxNode, TokenKind};

pub(super) fn lower_aggregate_call(node: &SyntaxNode, source: &str) -> Option<Expression> {
    let mut function = None;
    let mut quantifier = None;
    let mut arguments = Vec::new();
    let mut count_star = false;

    for element in node.children() {
        match element.kind {
            SyntaxElementKind::Token(token) => match token.kind {
                TokenKind::Keyword(keyword) if function.is_none() => {
                    function = aggregate_function(keyword);
                }
                TokenKind::Punctuation('*') => count_star = true,
                _ => {}
            },
            SyntaxElementKind::Node(child) if child.kind() == SyntaxKind::SetQuantifier => {
                quantifier = child
                    .children()
                    .into_iter()
                    .find_map(|element| match element.kind {
                        SyntaxElementKind::Token(token) => match token.kind {
                            TokenKind::Keyword(Keyword::All) => Some(SetQuantifier::All),
                            TokenKind::Keyword(Keyword::Distinct) => Some(SetQuantifier::Distinct),
                            _ => None,
                        },
                        SyntaxElementKind::Node(_) => None,
                    });
            }
            SyntaxElementKind::Node(child) if is_expression_kind(child.kind()) => {
                arguments.push(lower_expression(&child, source)?);
            }
            SyntaxElementKind::Node(_) => {}
        }
    }

    Some(Expression::AggregateCall {
        function: function?,
        quantifier,
        arguments,
        count_star,
        span: significant_node_span(node),
    })
}

fn aggregate_function(keyword: Keyword) -> Option<AggregateFunction> {
    Some(match keyword {
        Keyword::Avg => AggregateFunction::Average,
        Keyword::Count => AggregateFunction::Count,
        Keyword::Max => AggregateFunction::Maximum,
        Keyword::Min => AggregateFunction::Minimum,
        Keyword::Sum => AggregateFunction::Sum,
        Keyword::CollectList => AggregateFunction::CollectList,
        Keyword::StddevSamp => AggregateFunction::StandardDeviationSample,
        Keyword::StddevPop => AggregateFunction::StandardDeviationPopulation,
        Keyword::PercentileCont => AggregateFunction::PercentileContinuous,
        Keyword::PercentileDisc => AggregateFunction::PercentileDiscrete,
        _ => return None,
    })
}
