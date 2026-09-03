use super::core::{Event, Parser, node};
use crate::syntax::{Keyword, SyntaxKind};

impl Parser<'_> {
    pub(super) fn parse_case_expression(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());

        if !self.matches_keyword(Keyword::When) {
            children.extend(self.parse_expression());
            children.extend(self.skip_trivia());
        }

        let mut branch_count = 0;
        while self.matches_keyword(Keyword::When) {
            children.extend(self.parse_case_when_clause());
            children.extend(self.skip_trivia());
            branch_count += 1;
        }
        if branch_count == 0 {
            self.emit_match_syntax(
                "GQL-PARSE-CASE-SYNTAX",
                "CASE requires at least one WHEN branch",
                self.next_span_or(start),
            );
        }

        if self.matches_keyword(Keyword::Else) {
            children.extend(self.parse_case_else_clause());
            children.extend(self.skip_trivia());
        }

        if self.matches_keyword(Keyword::End) {
            children.push(self.bump_event());
        } else {
            self.emit_match_syntax(
                "GQL-PARSE-CASE-SYNTAX",
                "CASE expression is missing END",
                self.next_span_or(start),
            );
        }

        node(SyntaxKind::CaseExpression, children)
    }

    fn parse_case_when_clause(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());

        if self.matches_keyword(Keyword::Then) || self.at_eof() {
            self.emit_match_syntax(
                "GQL-PARSE-CASE-SYNTAX",
                "WHEN requires a condition expression",
                self.next_span_or(start),
            );
        } else {
            children.extend(self.parse_expression());
            children.extend(self.skip_trivia());
        }

        if self.matches_keyword(Keyword::Then) {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
        } else {
            self.emit_match_syntax(
                "GQL-PARSE-CASE-SYNTAX",
                "WHEN condition must be followed by THEN",
                self.next_span_or(start),
            );
        }

        if self.at_eof()
            || self.matches_keyword(Keyword::When)
            || self.matches_keyword(Keyword::Else)
            || self.matches_keyword(Keyword::End)
        {
            self.emit_match_syntax(
                "GQL-PARSE-CASE-SYNTAX",
                "THEN requires a result expression",
                self.next_span_or(start),
            );
        } else {
            children.extend(self.parse_expression());
        }

        node(SyntaxKind::CaseWhenClause, children)
    }

    fn parse_case_else_clause(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        if self.at_eof() || self.matches_keyword(Keyword::End) {
            self.emit_match_syntax(
                "GQL-PARSE-CASE-SYNTAX",
                "ELSE requires a result expression",
                self.next_span_or(start),
            );
        } else {
            children.extend(self.parse_expression());
        }
        node(SyntaxKind::CaseElseClause, children)
    }
}
