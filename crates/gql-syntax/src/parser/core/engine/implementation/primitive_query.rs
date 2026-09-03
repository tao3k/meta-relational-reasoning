//! ISO primitive-query statement parsing for FILTER and FOR.

#![forbid(unsafe_code)]

use gql_source::Span;

use crate::syntax::{Keyword, SyntaxKind, recovery_diagnostic};

use super::{Event, Parser, node};

impl Parser<'_> {
    pub(super) fn parse_filter_statement(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());

        if self
            .current_kind()
            .is_some_and(|kind| self.is_expression_start(kind) && !self.is_clause_keyword(kind))
        {
            children.extend(self.parse_expression());
        } else {
            self.emit_match_syntax(
                recovery_diagnostic("filter-statement")
                    .expect("Gerbil grammar owns FILTER recovery"),
                "FILTER requires a search condition",
                Span::new(start, self.span_end()),
            );
        }

        node(SyntaxKind::FilterStatement, children)
    }

    pub(super) fn parse_for_statement(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        let mut item = Vec::new();
        let mut invalid = false;

        item.extend(self.skip_trivia());
        if self.matches_regular_identifier() {
            item.push(self.bump_event());
        } else {
            invalid = true;
        }

        item.extend(self.skip_trivia());
        if self.matches_keyword(Keyword::In) {
            item.push(self.bump_event());
        } else {
            invalid = true;
        }

        item.extend(self.skip_trivia());
        if self
            .current_kind()
            .is_some_and(|kind| self.is_expression_start(kind) && !self.is_clause_keyword(kind))
        {
            item.extend(self.parse_expression());
        } else {
            invalid = true;
        }
        children.extend(node(SyntaxKind::ForItem, item));
        children.extend(self.skip_trivia());

        if self.matches_keyword(Keyword::With) {
            let mut ordinality = vec![self.bump_event()];
            ordinality.extend(self.skip_trivia());
            if self.matches_keyword(Keyword::Ordinality) || self.matches_keyword(Keyword::Offset) {
                ordinality.push(self.bump_event());
            } else {
                invalid = true;
            }
            ordinality.extend(self.skip_trivia());
            if self.matches_regular_identifier() {
                ordinality.push(self.bump_event());
            } else {
                invalid = true;
            }
            children.extend(node(SyntaxKind::ForOrdinalityOrOffset, ordinality));
        }

        if invalid {
            self.emit_match_syntax(
                recovery_diagnostic("for-statement").expect("Gerbil grammar owns FOR recovery"),
                "FOR requires a binding, IN source, and complete WITH binding when present",
                Span::new(start, self.span_end()),
            );
        }

        node(SyntaxKind::ForStatement, children)
    }
}
