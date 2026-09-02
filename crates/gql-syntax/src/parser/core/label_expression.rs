//! Lossless parser for ISO label predicates and label algebra.
#![forbid(unsafe_code)]

use gql_source::Span;

use super::{Event, Parser, node};
use crate::syntax::{Keyword, SyntaxKind, TokenKind, recovery_diagnostic};

impl Parser<'_> {
    pub(super) fn is_label_predicate_start(&self) -> bool {
        if self.current_kind() != Some(TokenKind::Keyword(Keyword::Is)) {
            return false;
        }
        let mut index = self.index + 1;
        while self
            .tokens
            .get(index)
            .is_some_and(|token| matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))
        {
            index += 1;
        }
        if self.tokens.get(index).map(|token| token.kind) == Some(TokenKind::Keyword(Keyword::Not))
        {
            index += 1;
            while self.tokens.get(index).is_some_and(|token| {
                matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment)
            }) {
                index += 1;
            }
        }
        self.tokens.get(index).map(|token| token.kind) == Some(TokenKind::Keyword(Keyword::Labeled))
    }

    pub(super) fn parse_label_predicate_suffix(
        &mut self,
        lhs: Vec<Event>,
        trivia: Vec<Event>,
    ) -> Vec<Event> {
        let mut children = lhs;
        children.extend(trivia);
        children.push(self.bump_event());
        children.extend(self.skip_trivia());
        if self.matches_keyword(Keyword::Not) {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
        }
        children.push(self.bump_event());
        children.extend(self.skip_trivia());
        if self.matches_kind(TokenKind::Punctuation(':')) {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
        } else {
            self.emit_label_diagnostic("label predicate requires `:` before its label expression");
        }
        children.extend(self.parse_label_or());
        node(SyntaxKind::LabelPredicateExpression, children)
    }

    fn parse_label_or(&mut self) -> Vec<Event> {
        let mut lhs = self.parse_label_and();
        loop {
            let checkpoint = self.index;
            let trivia = self.skip_trivia();
            if !self.matches_kind(TokenKind::Punctuation('|')) {
                self.index = checkpoint;
                break;
            }
            let mut children = lhs;
            children.extend(trivia);
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
            children.extend(self.parse_label_and());
            lhs = node(SyntaxKind::LabelOrExpression, children);
        }
        lhs
    }

    fn parse_label_and(&mut self) -> Vec<Event> {
        let mut lhs = self.parse_label_not();
        loop {
            let checkpoint = self.index;
            let trivia = self.skip_trivia();
            if !self.matches_kind(TokenKind::Punctuation('&')) {
                self.index = checkpoint;
                break;
            }
            let mut children = lhs;
            children.extend(trivia);
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
            children.extend(self.parse_label_not());
            lhs = node(SyntaxKind::LabelAndExpression, children);
        }
        lhs
    }

    fn parse_label_not(&mut self) -> Vec<Event> {
        if self.matches_kind(TokenKind::Punctuation('!')) {
            let mut children = vec![self.bump_event()];
            children.extend(self.skip_trivia());
            children.extend(self.parse_label_not());
            return node(SyntaxKind::LabelNotExpression, children);
        }
        self.parse_label_atom()
    }

    fn parse_label_atom(&mut self) -> Vec<Event> {
        match self.current_kind() {
            Some(TokenKind::Identifier) => {
                node(SyntaxKind::LabelNameExpression, vec![self.bump_event()])
            }
            Some(TokenKind::Punctuation('%')) => {
                node(SyntaxKind::LabelWildcardExpression, vec![self.bump_event()])
            }
            _ => {
                self.emit_label_diagnostic("expected a label name, `%`, or negated label");
                node(SyntaxKind::LabelNameExpression, Vec::new())
            }
        }
    }

    fn emit_label_diagnostic(&mut self, message: &'static str) {
        let span = self
            .tokens
            .get(self.index)
            .map(|token| token.span)
            .unwrap_or_else(|| Span::new(self.source.len() as u32, self.source.len() as u32));
        self.emit_match_syntax(
            recovery_diagnostic("label-expression")
                .expect("Gerbil grammar owns label-expression recovery"),
            message,
            span,
        );
    }
}
