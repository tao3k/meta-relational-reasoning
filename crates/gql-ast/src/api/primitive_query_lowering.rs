//! Lowering for ISO FILTER and FOR primitive-query statements.

#![forbid(unsafe_code)]

use gql_source::{Diagnostic, Span};
use gql_syntax::{
    SyntaxElement, SyntaxElementKind, SyntaxKind, SyntaxNode, Token, TokenKind,
    is_non_reserved_word,
};

use super::identifier_lowering::identifier_from_token;
use super::lowering_support::is_expression_kind;
use super::{
    Expression, ForItem, ForOrdinalityBinding, ForOrdinalityKind, Identifier, QueryClause,
};

pub(super) fn lower_primitive_query_clause(
    node: &SyntaxNode,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
    lower_expression: fn(&SyntaxNode, &str) -> Option<Expression>,
) -> Option<QueryClause> {
    match node.kind() {
        SyntaxKind::FilterStatement => {
            lower_filter_statement(node, source, diagnostics, lower_expression).map(|expression| {
                QueryClause::Filter {
                    expression,
                    span: significant_node_span(node),
                }
            })
        }
        SyntaxKind::ForStatement => {
            lower_for_statement(node, source, diagnostics, lower_expression).map(|item| {
                QueryClause::For {
                    item,
                    span: significant_node_span(node),
                }
            })
        }
        _ => None,
    }
}

fn lower_filter_statement(
    node: &SyntaxNode,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
    lower_expression: fn(&SyntaxNode, &str) -> Option<Expression>,
) -> Option<Expression> {
    let expression = node.children().iter().find_map(|element| {
        let child = syntax_node(element)?;
        is_expression_kind(child.kind())
            .then(|| lower_expression(child, source))
            .flatten()
    });
    if expression.is_none() {
        diagnostics.push(Diagnostic::error(
            "GQL-AST-FILTER-MISSING-EXPRESSION",
            "FILTER statement is missing a search condition",
            node.span(),
        ));
    }
    expression
}

fn lower_for_statement(
    node: &SyntaxNode,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
    lower_expression: fn(&SyntaxNode, &str) -> Option<Expression>,
) -> Option<ForItem> {
    let children = node.children();
    let item = children
        .iter()
        .filter_map(syntax_node)
        .find(|child| child.kind() == SyntaxKind::ForItem)?;
    let binding = first_identifier(item, source);
    let value = item.children().iter().find_map(|element| {
        let expression = syntax_node(element)?;
        is_expression_kind(expression.kind())
            .then(|| lower_expression(expression, source))
            .flatten()
    });
    let ordinality = children
        .iter()
        .filter_map(syntax_node)
        .find(|child| child.kind() == SyntaxKind::ForOrdinalityOrOffset)
        .and_then(|child| {
            let binding = last_identifier(child, source)?;
            let kind = if syntax_tokens(child.children())
                .any(|token| token.text().eq_ignore_ascii_case("OFFSET"))
            {
                ForOrdinalityKind::Offset
            } else {
                ForOrdinalityKind::Ordinality
            };
            Some(ForOrdinalityBinding {
                kind,
                binding,
                span: significant_node_span(child),
            })
        });

    let Some(binding) = binding else {
        diagnostics.push(Diagnostic::error(
            "GQL-AST-FOR-BINDING-MISSING",
            "FOR item requires a binding variable",
            item.span(),
        ));
        return None;
    };
    let Some(source_expression) = value else {
        diagnostics.push(Diagnostic::error(
            "GQL-AST-FOR-SOURCE-MISSING",
            "FOR item requires a source expression",
            item.span(),
        ));
        return None;
    };
    Some(ForItem {
        binding,
        source: source_expression,
        ordinality,
        span: significant_node_span(item),
    })
}

fn syntax_node(element: &SyntaxElement) -> Option<&SyntaxNode> {
    match &element.kind {
        SyntaxElementKind::Node(node) => Some(node),
        SyntaxElementKind::Token(_) => None,
    }
}

fn syntax_tokens(elements: Vec<SyntaxElement>) -> impl Iterator<Item = Token> {
    elements
        .into_iter()
        .filter_map(|element| match element.kind {
            SyntaxElementKind::Token(token) => Some(token),
            SyntaxElementKind::Node(_) => None,
        })
}

fn first_identifier(node: &SyntaxNode, source: &str) -> Option<Identifier> {
    syntax_tokens(node.children()).find_map(|token| {
        (token.kind == TokenKind::Identifier || is_non_reserved_word(token.text()))
            .then(|| identifier_from_token(&token, source))
    })
}

fn last_identifier(node: &SyntaxNode, source: &str) -> Option<Identifier> {
    syntax_tokens(node.children())
        .filter_map(|token| {
            (token.kind == TokenKind::Identifier || is_non_reserved_word(token.text()))
                .then(|| identifier_from_token(&token, source))
        })
        .last()
}

fn significant_node_span(node: &SyntaxNode) -> Span {
    let mut spans = node
        .children()
        .into_iter()
        .filter_map(|element| match element.kind {
            SyntaxElementKind::Node(child) => Some(child.span()),
            SyntaxElementKind::Token(token)
                if !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment) =>
            {
                Some(token.span)
            }
            SyntaxElementKind::Token(_) => None,
        });
    let Some(first) = spans.next() else {
        return node.span();
    };
    spans.fold(first, |span, next| Span::new(span.start, next.end))
}
