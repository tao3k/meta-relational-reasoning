//! Shared ISO non-negative integer specification parser.
#![forbid(unsafe_code)]

use super::{Event, Parser, node};
use crate::syntax::{SyntaxKind, TokenKind};

impl Parser<'_> {
    pub(in crate::parser) fn is_non_negative_integer_specification(&self) -> bool {
        self.current().is_some_and(|token| match token.kind {
            TokenKind::Number => token.text().parse::<u64>().is_ok(),
            TokenKind::DynamicParameter => true,
            _ => false,
        })
    }

    pub(in crate::parser) fn parse_non_negative_integer_specification(&mut self) -> Vec<Event> {
        debug_assert!(self.is_non_negative_integer_specification());
        node(
            SyntaxKind::NonNegativeIntegerSpecification,
            vec![self.bump_event()],
        )
    }

    pub(in crate::parser) fn drain_invalid_non_negative_integer_specification(
        &mut self,
    ) -> Vec<Event> {
        let mut children = Vec::new();
        if self.matches_kind(TokenKind::Punctuation('-')) {
            children.push(self.bump_event());
        }
        if matches!(
            self.current_kind(),
            Some(
                TokenKind::Number
                    | TokenKind::DynamicParameter
                    | TokenKind::SubstitutedParameter
                    | TokenKind::Identifier
            )
        ) {
            children.push(self.bump_event());
        }
        node(SyntaxKind::NonNegativeIntegerSpecification, children)
    }
}
