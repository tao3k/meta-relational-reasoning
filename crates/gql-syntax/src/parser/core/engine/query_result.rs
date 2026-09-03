//! RETURN, grouping, ordering, and pagination parser owner.

use gql_source::Span;

use super::{Event, Parser, node};
use crate::syntax::{Keyword, SyntaxKind, TokenKind};

impl Parser<'_> {
    pub(super) fn parse_union_clause(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        if !matches!(
            self.current_kind(),
            Some(TokenKind::Keyword(
                Keyword::Match | Keyword::Optional | Keyword::Let | Keyword::Return
            ))
        ) {
            self.emit_return_syntax(
                crate::syntax::recovery_diagnostic("union-clause")
                    .expect("Gerbil grammar owns UNION recovery"),
                "UNION requires a complete query branch on its right side",
                self.next_span_or(start),
            );
        }
        node(SyntaxKind::UnionClause, children)
    }

    pub(super) fn parse_return_clause(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        if self.matches_keyword(Keyword::All) || self.matches_keyword(Keyword::Distinct) {
            children.extend(node(SyntaxKind::SetQuantifier, vec![self.bump_event()]));
        }
        children.extend(self.skip_trivia());
        if self.matches_kind(TokenKind::Punctuation('*')) {
            children.push(self.bump_event());
            let trailing_checkpoint = self.index;
            let trailing_trivia = self.skip_trivia();
            if !self.at_eof()
                && !self
                    .current_kind()
                    .is_some_and(|kind| self.is_clause_keyword(kind))
            {
                children.extend(trailing_trivia);
                self.emit_return_syntax(
                    "GQL-PARSE-RETURN-SYNTAX",
                    "RETURN * does not accept additional projection items",
                    self.next_span_or(start),
                );
                while !self.at_eof()
                    && !self
                        .current_kind()
                        .is_some_and(|kind| self.is_clause_keyword(kind))
                {
                    children.push(self.bump_event());
                }
            } else {
                self.index = trailing_checkpoint;
            }
            return node(SyntaxKind::ReturnClause, children);
        }
        let mut expressions = 0usize;
        loop {
            children.extend(self.skip_trivia());
            let Some(token) = self.current() else { break };
            if self.is_clause_keyword(token.kind) {
                break;
            }
            if self.is_expression_start(token.kind) {
                children.extend(self.parse_expression());
                expressions += 1;
                children.extend(self.skip_trivia());
                if self.matches_keyword(Keyword::As) {
                    children.extend(self.parse_projection_alias());
                }
                children.extend(self.skip_trivia());
                if self.matches_kind(TokenKind::Punctuation(',')) {
                    children.push(self.bump_event());
                }
            } else if matches!(token.kind, TokenKind::Punctuation(_)) {
                children.push(self.bump_event());
            } else {
                self.emit_return_syntax(
                    "GQL-PARSE-RETURN-SYNTAX",
                    "invalid token in RETURN clause",
                    token.span,
                );
                children.push(self.bump_event());
                break;
            }
        }
        if expressions == 0 {
            self.emit_return_syntax(
                "GQL-PARSE-RETURN-SYNTAX",
                "RETURN requires at least one expression",
                Span::new(start, self.span_end()),
            );
        }
        node(SyntaxKind::ReturnClause, children)
    }

    pub(super) fn parse_finish_statement(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        if !self.at_eof() && !self.matches_keyword(Keyword::Union) {
            self.emit_return_syntax(
                crate::syntax::recovery_diagnostic("finish-statement")
                    .expect("Gerbil grammar owns FINISH recovery"),
                "FINISH does not accept a result expression",
                self.next_span_or(start),
            );
            while !self.at_eof() && !self.matches_keyword(Keyword::Union) {
                children.push(self.bump_event());
            }
        }
        node(SyntaxKind::FinishStatement, children)
    }

    fn parse_projection_alias(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        if self.matches_identifier() {
            children.push(self.bump_event());
        } else {
            self.emit_return_syntax(
                "GQL-PARSE-RETURN-ALIAS-SYNTAX",
                "AS requires a projection alias identifier",
                self.next_span_or(start),
            );
        }
        node(SyntaxKind::ProjectionAlias, children)
    }

    pub(super) fn parse_limit_clause(&mut self) -> Vec<Event> {
        self.parse_page_clause(
            SyntaxKind::LimitClause,
            "GQL-PARSE-LIMIT-SYNTAX",
            "LIMIT requires a non-negative integer literal or dynamic parameter",
        )
    }

    pub(super) fn parse_offset_clause(&mut self) -> Vec<Event> {
        self.parse_page_clause(
            SyntaxKind::OffsetClause,
            "GQL-PARSE-OFFSET-SYNTAX",
            "OFFSET or SKIP requires a non-negative integer literal or dynamic parameter",
        )
    }

    fn parse_page_clause(
        &mut self,
        kind: SyntaxKind,
        code: &'static str,
        message: &'static str,
    ) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        if self.is_non_negative_integer_specification() {
            children.extend(self.parse_non_negative_integer_specification());
        } else {
            self.emit_return_syntax(code, message, self.next_span_or(start));
            children.extend(self.drain_invalid_non_negative_integer_specification());
        }
        node(kind, children)
    }

    pub(super) fn parse_order_by_clause(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        if self.matches_keyword(Keyword::By) {
            children.push(self.bump_event());
        } else {
            self.emit_return_syntax(
                "GQL-PARSE-ORDER-BY-SYNTAX",
                "ORDER must be followed by BY",
                self.next_span_or(start),
            );
            return node(SyntaxKind::OrderByClause, children);
        }

        let mut diagnosed = false;
        loop {
            children.extend(self.skip_trivia());
            if self.at_eof()
                || self.is_clause_keyword(self.current_kind().unwrap_or(TokenKind::Unknown))
            {
                if !diagnosed {
                    self.emit_return_syntax(
                        "GQL-PARSE-ORDER-BY-SYNTAX",
                        "ORDER BY requires a sort expression after BY or `,`",
                        self.next_span_or(start),
                    );
                }
                break;
            }
            if !self
                .current_kind()
                .is_some_and(|token| self.is_expression_start(token))
            {
                self.emit_return_syntax(
                    "GQL-PARSE-ORDER-BY-SYNTAX",
                    "ORDER BY requires a sort expression",
                    self.next_span_or(start),
                );
                break;
            }

            let mut specification = self.parse_expression();
            specification.extend(self.skip_trivia());
            if self.matches_word("ASC")
                || self.matches_word("ASCENDING")
                || self.matches_word("DESC")
                || self.matches_word("DESCENDING")
            {
                specification.extend(node(
                    SyntaxKind::OrderingSpecification,
                    vec![self.bump_event()],
                ));
                specification.extend(self.skip_trivia());
            }
            if self.matches_word("NULLS") {
                let mut null_ordering = vec![self.bump_event()];
                null_ordering.extend(self.skip_trivia());
                if self.matches_word("FIRST") || self.matches_word("LAST") {
                    null_ordering.push(self.bump_event());
                } else {
                    self.emit_return_syntax(
                        "GQL-PARSE-ORDER-BY-SYNTAX",
                        "NULLS requires FIRST or LAST",
                        self.next_span_or(start),
                    );
                    diagnosed = true;
                    if matches!(
                        self.current_kind(),
                        Some(TokenKind::Identifier | TokenKind::Keyword(_))
                    ) {
                        null_ordering.push(self.bump_event());
                    }
                }
                specification.extend(node(SyntaxKind::NullOrdering, null_ordering));
            }
            children.extend(node(SyntaxKind::SortSpecification, specification));

            let separator_checkpoint = self.index;
            let separator_trivia = self.skip_trivia();
            if self.matches_kind(TokenKind::Punctuation(',')) {
                children.extend(separator_trivia);
                children.push(self.bump_event());
                continue;
            }
            if self.at_eof()
                || self.is_clause_keyword(self.current_kind().unwrap_or(TokenKind::Unknown))
            {
                self.index = separator_checkpoint;
                break;
            }
            children.extend(separator_trivia);
            {
                self.emit_return_syntax(
                    "GQL-PARSE-ORDER-BY-SYNTAX",
                    "sort specifications require `,` or a following query clause",
                    self.next_span_or(start),
                );
                while !self.at_eof()
                    && !self.is_clause_keyword(self.current_kind().unwrap_or(TokenKind::Unknown))
                    && !self.matches_kind(TokenKind::Punctuation(','))
                {
                    children.push(self.bump_event());
                }
            }
            break;
        }
        node(SyntaxKind::OrderByClause, children)
    }

    pub(super) fn parse_group_by_clause(&mut self) -> Vec<Event> {
        self.parse_expression_key_clause(
            SyntaxKind::GroupByClause,
            "GQL-PARSE-GROUP-BY-SYNTAX",
            "GROUP",
        )
    }

    fn parse_expression_key_clause(
        &mut self,
        kind: SyntaxKind,
        code: &'static str,
        subject: &'static str,
    ) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        if self.matches_keyword(Keyword::By) {
            children.push(self.bump_event());
        } else {
            self.emit_return_syntax(
                code,
                format!("{subject} must be followed by BY"),
                self.next_span_or(start),
            );
            return node(kind, children);
        }
        let mut keys = 0usize;
        loop {
            children.extend(self.skip_trivia());
            if self.at_eof()
                || self.is_clause_keyword(self.current_kind().unwrap_or(TokenKind::Unknown))
            {
                break;
            }
            if !self
                .current_kind()
                .is_some_and(|token| self.is_expression_start(token))
            {
                self.emit_return_syntax(
                    code,
                    format!("{subject} BY requires at least one expression"),
                    self.next_span_or(start),
                );
                break;
            }
            children.extend(self.parse_expression());
            keys += 1;
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation(',')) {
                children.push(self.bump_event());
                continue;
            }
            break;
        }
        if keys == 0 {
            self.emit_return_syntax(
                code,
                format!("{subject} BY requires at least one expression"),
                Span::new(start, self.span_end()),
            );
        }
        node(kind, children)
    }
}
