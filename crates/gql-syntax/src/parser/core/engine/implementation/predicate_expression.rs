//! Lossless parser owner for ISO predicate expressions.
#![forbid(unsafe_code)]

use gql_source::Span;

use super::{Event, Parser, node};
use crate::syntax::{Keyword, SyntaxKind, TokenKind, recovery_diagnostic};

impl Parser<'_> {
    pub(super) fn parse_value_type_predicate_suffix(
        &mut self,
        mut children: Vec<Event>,
        operand_is_primary: bool,
        start: u32,
    ) -> Vec<Event> {
        if !operand_is_primary {
            self.emit_match_syntax(
                recovery_diagnostic("value-type-predicate-operand")
                    .expect("Gerbil grammar owns value-type predicate operand recovery"),
                "value-type predicates require a value expression primary; parenthesize a composite expression",
                Span::new(start, self.span_end()),
            );
        }
        if self.matches_kind(TokenKind::Keyword(Keyword::Typed)) {
            children.push(self.bump_event());
        } else {
            children.push(self.bump_event());
            if self.matches_kind(TokenKind::Punctuation(':')) {
                children.push(self.bump_event());
            } else {
                self.emit_match_syntax(
                    recovery_diagnostic("value-type-predicate")
                        .expect("Gerbil grammar owns value-type predicate recovery"),
                    "value-type predicate marker must be `TYPED` or `::`",
                    Span::new(start, self.span_end()),
                );
                return node(SyntaxKind::ValueTypePredicateExpression, children);
            }
        }
        children.extend(self.skip_trivia());
        if self.is_value_type_start() {
            children.extend(
                self.parse_property_value_type(
                    start,
                    recovery_diagnostic("value-type-predicate")
                        .expect("Gerbil grammar owns value-type predicate recovery"),
                ),
            );
        } else {
            self.emit_match_syntax(
                recovery_diagnostic("value-type-predicate")
                    .expect("Gerbil grammar owns value-type predicate recovery"),
                "value-type predicate requires an ISO GQL value type",
                Span::new(start, self.span_end()),
            );
        }
        node(SyntaxKind::ValueTypePredicateExpression, children)
    }

    pub(super) fn parse_graph_element_predicate_suffix(
        &mut self,
        mut children: Vec<Event>,
        operand_is_element_reference: bool,
        start: u32,
    ) -> Vec<Event> {
        let kind = if self.matches_word("DIRECTED") {
            SyntaxKind::DirectedPredicateExpression
        } else {
            SyntaxKind::EndpointPredicateExpression
        };
        if !operand_is_element_reference {
            self.emit_graph_element_predicate_syntax(
                "graph-element predicate requires an element variable reference",
                start,
            );
        }

        let endpoint = self.matches_word("SOURCE") || self.matches_word("DESTINATION");
        children.push(self.bump_event());
        if endpoint {
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Keyword(Keyword::Of)) {
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
            } else {
                self.emit_graph_element_predicate_syntax(
                    "SOURCE and DESTINATION predicates require `OF`",
                    start,
                );
                return node(kind, children);
            }
            if self.matches_regular_identifier() {
                children.extend(node(SyntaxKind::NameExpression, vec![self.bump_event()]));
            } else {
                self.emit_graph_element_predicate_syntax(
                    "SOURCE and DESTINATION predicates require an edge variable reference",
                    start,
                );
            }
        }
        node(kind, children)
    }

    pub(super) fn parse_graph_element_predicate_function(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let kind = match self.current_kind() {
            Some(TokenKind::Keyword(Keyword::AllDifferent | Keyword::Same)) => {
                SyntaxKind::ElementIdentityPredicateExpression
            }
            Some(TokenKind::Keyword(Keyword::PropertyExists)) => {
                SyntaxKind::PropertyExistsPredicateExpression
            }
            _ => unreachable!("caller admits only graph-element predicate functions"),
        };
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        if !self.matches_kind(TokenKind::Punctuation('(')) {
            self.emit_graph_element_predicate_syntax(
                "graph-element predicate function requires `(`",
                start,
            );
            return node(kind, children);
        }
        children.push(self.bump_event());
        children.extend(self.skip_trivia());

        let mut arguments = 0usize;
        let mut invalid_message = None;
        loop {
            if self.matches_regular_identifier() {
                children.extend(node(SyntaxKind::NameExpression, vec![self.bump_event()]));
                arguments += 1;
            } else {
                invalid_message =
                    Some("graph-element predicate arguments must be element or property names");
                break;
            }
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation(',')) {
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
                continue;
            }
            break;
        }

        if invalid_message.is_none() {
            invalid_message =
                if kind == SyntaxKind::PropertyExistsPredicateExpression && arguments != 2 {
                    Some("PROPERTY_EXISTS requires exactly two arguments")
                } else if kind == SyntaxKind::ElementIdentityPredicateExpression && arguments < 2 {
                    Some("graph-element identity predicate requires at least two arguments")
                } else {
                    None
                };
        }
        if invalid_message.is_some() {
            while self.current_kind().is_some() && !self.matches_kind(TokenKind::Punctuation(')')) {
                children.push(self.bump_event());
            }
        }
        if self.matches_kind(TokenKind::Punctuation(')')) {
            children.push(self.bump_event());
        } else if invalid_message.is_none() {
            invalid_message = Some("graph-element predicate function requires `)`");
        }
        if let Some(message) = invalid_message {
            self.emit_graph_element_predicate_syntax(message, start);
        }
        node(kind, children)
    }

    fn emit_graph_element_predicate_syntax(&mut self, message: &'static str, start: u32) {
        self.emit_match_syntax(
            recovery_diagnostic("graph-element-predicate")
                .expect("Gerbil grammar owns graph-element predicate recovery"),
            message,
            self.next_span_or(start),
        );
    }
}
