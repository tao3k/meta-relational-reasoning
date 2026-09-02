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
        self.parse_unsigned_clause(
            SyntaxKind::LimitClause,
            "GQL-PARSE-LIMIT-SYNTAX",
            "LIMIT requires a positive integer literal",
        )
    }

    pub(super) fn parse_offset_clause(&mut self) -> Vec<Event> {
        self.parse_unsigned_clause(
            SyntaxKind::OffsetClause,
            "GQL-PARSE-OFFSET-SYNTAX",
            "OFFSET requires a non-negative integer literal",
        )
    }

    fn parse_unsigned_clause(
        &mut self,
        kind: SyntaxKind,
        code: &'static str,
        message: &'static str,
    ) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        if self.matches_kind(TokenKind::Number) {
            children.push(self.bump_event());
        } else {
            self.emit_return_syntax(code, message, self.next_span_or(start));
        }
        node(kind, children)
    }

    pub(super) fn parse_order_by_clause(&mut self) -> Vec<Event> {
        self.parse_expression_key_clause(
            SyntaxKind::OrderByClause,
            "GQL-PARSE-ORDER-BY-SYNTAX",
            "ORDER",
        )
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
            if kind == SyntaxKind::OrderByClause
                && (self.matches_keyword(Keyword::Asc) || self.matches_keyword(Keyword::Desc))
            {
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
            }
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
