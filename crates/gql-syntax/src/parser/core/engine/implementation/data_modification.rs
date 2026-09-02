//! Graph data-modification statement parser owner.

use crate::syntax::{Keyword, SyntaxKind, TokenKind, recovery_diagnostic};

use super::{Event, Parser, node};

impl Parser<'_> {
    pub(super) fn parse_insert_statement(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        if self.matches_kind(TokenKind::Punctuation('(')) {
            children.extend(self.parse_graph_pattern_list());
        } else {
            self.emit_match_syntax(
                recovery_diagnostic("insert-statement")
                    .expect("Gerbil grammar owns INSERT recovery"),
                "INSERT requires an insert graph pattern",
                self.next_span_or(start),
            );
        }
        node(SyntaxKind::InsertStatement, children)
    }

    pub(super) fn parse_set_statement(&mut self) -> Vec<Event> {
        self.parse_expression_item_statement(
            SyntaxKind::SetStatement,
            SyntaxKind::SetItem,
            "set-statement",
            "SET requires at least one assignment",
        )
    }

    pub(super) fn parse_remove_statement(&mut self) -> Vec<Event> {
        self.parse_expression_item_statement(
            SyntaxKind::RemoveStatement,
            SyntaxKind::RemoveItem,
            "remove-statement",
            "REMOVE requires at least one property expression",
        )
    }

    pub(super) fn parse_delete_statement(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = Vec::new();
        if matches!(
            self.current_kind(),
            Some(TokenKind::Keyword(Keyword::Detach | Keyword::Nodetach))
        ) {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Keyword(Keyword::Delete)) {
                children.push(self.bump_event());
            } else {
                self.emit_match_syntax(
                    recovery_diagnostic("delete-statement")
                        .expect("Gerbil grammar owns DELETE recovery"),
                    "DETACH or NODETACH must be followed by DELETE",
                    self.next_span_or(start),
                );
                return node(SyntaxKind::DeleteStatement, children);
            }
        } else {
            children.push(self.bump_event());
        }

        let mut item_count = 0usize;
        loop {
            children.extend(self.skip_trivia());
            if self.at_eof()
                || self
                    .current_kind()
                    .is_some_and(|kind| self.is_clause_keyword(kind))
            {
                break;
            }
            let expression = self.parse_expression();
            children.extend(node(SyntaxKind::DeleteItem, expression));
            item_count += 1;
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation(',')) {
                children.push(self.bump_event());
            } else {
                break;
            }
        }
        if item_count == 0 {
            self.emit_match_syntax(
                recovery_diagnostic("delete-statement")
                    .expect("Gerbil grammar owns DELETE recovery"),
                "DELETE requires at least one value expression",
                self.next_span_or(start),
            );
        }
        node(SyntaxKind::DeleteStatement, children)
    }

    fn parse_expression_item_statement(
        &mut self,
        statement_kind: SyntaxKind,
        item_kind: SyntaxKind,
        recovery: &'static str,
        missing_message: &'static str,
    ) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        let mut item_count = 0usize;
        loop {
            children.extend(self.skip_trivia());
            if self.at_eof()
                || self
                    .current_kind()
                    .is_some_and(|kind| self.is_clause_keyword(kind))
            {
                break;
            }
            let expression = self.parse_expression();
            children.extend(node(item_kind, expression));
            item_count += 1;
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation(',')) {
                children.push(self.bump_event());
            } else {
                break;
            }
        }
        if item_count == 0 {
            self.emit_match_syntax(
                recovery_diagnostic(recovery).expect("Gerbil grammar owns statement recovery"),
                missing_message,
                self.next_span_or(start),
            );
        }
        node(statement_kind, children)
    }
}
