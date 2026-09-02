//! Postfix expression traversal for calls, property access, and subscripts.
#![forbid(unsafe_code)]

use super::{Event, Parser, node};
use crate::syntax::{SyntaxKind, TokenKind, recovery_diagnostic};

impl Parser<'_> {
    pub(super) fn parse_postfix_expression(&mut self, base: Vec<Event>) -> Vec<Event> {
        let mut expression = base;
        loop {
            let access_start = self.index;
            let trivia = self.skip_trivia();
            if self.matches_kind(TokenKind::Punctuation('('))
                && expression
                    .first()
                    .is_some_and(|event| matches!(event, Event::Start(SyntaxKind::NameExpression)))
            {
                expression = self.parse_function_call(expression, trivia);
                continue;
            }
            if self.matches_kind(TokenKind::Punctuation('.')) {
                let mut children = expression;
                children.extend(trivia);
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
                if self.matches_identifier() {
                    children.push(self.bump_event());
                    expression = node(SyntaxKind::PropertyAccessExpression, children);
                } else {
                    self.emit_match_syntax(
                        recovery_diagnostic("expression-syntax")
                            .expect("Gerbil grammar owns expression recovery"),
                        "property access requires an identifier after `.`",
                        self.next_span_or(self.span_end()),
                    );
                    expression = node(SyntaxKind::PropertyAccessExpression, children);
                    break;
                }
                continue;
            }
            if self.matches_kind(TokenKind::Punctuation('[')) {
                expression = self.parse_subscript(expression, trivia);
                continue;
            }
            self.index = access_start;
            break;
        }
        expression
    }

    fn parse_function_call(&mut self, expression: Vec<Event>, trivia: Vec<Event>) -> Vec<Event> {
        let mut children = expression;
        children.extend(trivia);
        children.push(self.bump_event());
        loop {
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation(')')) {
                children.push(self.bump_event());
                break;
            }
            if self.at_eof() {
                self.emit_return_syntax(
                    "GQL-PARSE-FUNCTION-CALL-SYNTAX",
                    "function call is missing `)`",
                    self.next_span_or(self.span_end()),
                );
                break;
            }
            children.extend(self.parse_expression());
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation(',')) {
                children.push(self.bump_event());
                continue;
            }
            if self.matches_kind(TokenKind::Punctuation(')')) {
                children.push(self.bump_event());
                break;
            }
            self.emit_return_syntax(
                "GQL-PARSE-FUNCTION-CALL-SYNTAX",
                "function arguments require `,` or `)`",
                self.next_span_or(self.span_end()),
            );
            break;
        }
        node(SyntaxKind::FunctionCallExpression, children)
    }

    fn parse_subscript(&mut self, expression: Vec<Event>, trivia: Vec<Event>) -> Vec<Event> {
        let mut children = expression;
        children.extend(trivia);
        children.push(self.bump_event());
        children.extend(self.skip_trivia());
        if !self.at_eof() && !self.matches_kind(TokenKind::Punctuation(']')) {
            children.extend(self.parse_expression());
            children.extend(self.skip_trivia());
        } else {
            self.emit_match_syntax(
                "GQL-PARSE-SUBSCRIPT-SYNTAX",
                "collection subscript requires an index expression",
                self.next_span_or(self.span_end()),
            );
        }
        if self.matches_kind(TokenKind::Punctuation(']')) {
            children.push(self.bump_event());
        } else {
            self.emit_match_syntax(
                "GQL-PARSE-SUBSCRIPT-SYNTAX",
                "collection subscript is missing `]`",
                self.next_span_or(self.span_end()),
            );
        }
        node(SyntaxKind::SubscriptExpression, children)
    }
}
