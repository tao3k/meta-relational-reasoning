//! Fail-closed lowering for byte, temporal, duration, and record literals.
#![forbid(unsafe_code)]

use super::identifier_lowering::identifier_from_token;
use super::lowering::{lower_expression, significant_node_span};
use super::lowering_support::is_expression_kind;
use super::{Expression, RecordField};
use gql_syntax::{
    Keyword, SyntaxElement, SyntaxElementKind, SyntaxKind, SyntaxNode, TokenKind,
    decode_character_string, is_non_reserved_word,
};

fn is_character_string_token(kind: TokenKind, text: &str) -> bool {
    kind == TokenKind::String
        || kind == TokenKind::Identifier && (text.starts_with('"') || text.starts_with("@\""))
}

pub(super) fn lower_general_literal(node: &SyntaxNode, source: &str) -> Option<Expression> {
    match node.kind() {
        SyntaxKind::ByteStringLiteralExpression => {
            let children = node.children();
            let token = children.iter().find_map(|element| match element {
                SyntaxElement {
                    kind: SyntaxElementKind::Token(token),
                } if token.kind == TokenKind::ByteString => Some(token),
                _ => None,
            })?;
            Some(Expression::ByteString(
                lower_byte_string(token.text())?,
                token.span,
            ))
        }
        SyntaxKind::TemporalLiteralExpression => lower_temporal_literal(node),
        SyntaxKind::DurationLiteralExpression => {
            let children = node.children();
            let token = children.iter().find_map(|element| match element {
                SyntaxElement {
                    kind: SyntaxElementKind::Token(token),
                } if is_character_string_token(token.kind, token.text()) => Some(token),
                _ => None,
            })?;
            Some(Expression::Duration(
                lower_character_sequence(token.text())?,
                significant_node_span(node),
            ))
        }
        SyntaxKind::RecordExpression => {
            let fields = node
                .children()
                .iter()
                .filter_map(|element| match element {
                    SyntaxElement {
                        kind: SyntaxElementKind::Node(child),
                    } if child.kind() == SyntaxKind::RecordEntry => Some(child),
                    _ => None,
                })
                .map(|entry| lower_record_field(entry, source))
                .collect::<Option<Vec<_>>>()?;
            Some(Expression::Record(fields, node.span()))
        }
        _ => None,
    }
}

pub(super) fn lower_character_sequence(text: &str) -> Option<String> {
    decode_character_string(text).map(|decoded| decoded.value.into_owned())
}

fn lower_byte_string(text: &str) -> Option<Vec<u8>> {
    let body = text
        .get(2..text.len().checked_sub(1)?)?
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace());
    let mut result = Vec::with_capacity(body.size_hint().0 / 2);
    let mut high = None;
    for byte in body {
        let nibble = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => return None,
        };
        if let Some(high) = high.take() {
            result.push((high << 4) | nibble);
        } else {
            high = Some(nibble);
        }
    }
    high.is_none().then_some(result)
}

fn lower_temporal_literal(node: &SyntaxNode) -> Option<Expression> {
    let children = node.children();
    let qualifier = children.iter().find_map(|element| match element {
        SyntaxElement {
            kind: SyntaxElementKind::Token(token),
        } => match token.kind {
            TokenKind::Keyword(
                keyword @ (Keyword::Date | Keyword::Time | Keyword::Timestamp | Keyword::Datetime),
            ) => Some(keyword),
            _ => None,
        },
        _ => None,
    })?;
    let value = children.iter().find_map(|element| match element {
        SyntaxElement {
            kind: SyntaxElementKind::Token(token),
        } if is_character_string_token(token.kind, token.text()) => {
            lower_character_sequence(token.text())
        }
        _ => None,
    })?;
    let span = significant_node_span(node);
    Some(match qualifier {
        Keyword::Date => Expression::Date(value, span),
        Keyword::Time => Expression::Time(value, span),
        Keyword::Timestamp | Keyword::Datetime => Expression::Timestamp(value, span),
        _ => unreachable!("qualifier restricted above"),
    })
}

fn lower_record_field(node: &SyntaxNode, source: &str) -> Option<RecordField> {
    let name = node.children().iter().find_map(|element| match element {
        SyntaxElement {
            kind: SyntaxElementKind::Token(token),
        } if token.kind == TokenKind::Identifier
            || matches!(token.kind, TokenKind::Keyword(_))
                && is_non_reserved_word(token.text()) =>
        {
            Some(identifier_from_token(token, source))
        }
        _ => None,
    })?;
    let value = node.children().iter().find_map(|element| match element {
        SyntaxElement {
            kind: SyntaxElementKind::Node(child),
        } if is_expression_kind(child.kind()) => lower_expression(child, source),
        _ => None,
    })?;
    Some(RecordField {
        name,
        value,
        span: significant_node_span(node),
    })
}
