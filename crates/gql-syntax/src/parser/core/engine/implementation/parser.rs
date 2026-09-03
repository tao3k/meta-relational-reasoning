//! Lossless event-parser engine for the supported ISO GQL syntax slice.

#![forbid(unsafe_code)]

use gql_source::{Diagnostic, SourceText, Span};

use crate::parser::core::keyword_name;
use crate::syntax::{
    GrammarParserAction, Keyword, SyntaxKind, SyntaxTree, Token, TokenKind,
    aggregate_function_spec, binary_operator_spec, prefix_operator_precedence, recovery_diagnostic,
    top_level_parser_entrypoint,
};

/// Parser output consumed directly by the Rowan tree sink.
#[derive(Clone, Debug)]
pub(in crate::parser) enum Event {
    Start(SyntaxKind),
    Finish,
    Token(Token),
}

/// Parses `source` into one lossless Rowan CST and diagnostics.
pub fn parse(name: &str, source: &str) -> crate::Parse {
    let source = SourceText::new(name, source);
    let (tokens, mut diagnostics) = crate::lexer::lex(source.text());
    let mut parser = Parser::new(&tokens, source.text());
    let children = parser.parse_top_level();
    parser.collect_diagnostics(&mut diagnostics);

    diagnostics.sort_by(|left, right| {
        (left.span.start, left.span.end, left.code).cmp(&(
            right.span.start,
            right.span.end,
            right.code,
        ))
    });

    let query = node(SyntaxKind::Query, children);
    let events = node(SyntaxKind::SourceFile, query);
    let rowan = build_rowan_root(&events);

    crate::Parse {
        tree: SyntaxTree::new(source, tokens, rowan),
        diagnostics,
    }
}

pub(in crate::parser) fn node(kind: SyntaxKind, mut children: Vec<Event>) -> Vec<Event> {
    let mut events = Vec::with_capacity(children.len() + 2);
    events.push(Event::Start(kind));
    events.append(&mut children);
    events.push(Event::Finish);
    events
}

fn is_value_expression_primary_kind(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::NameExpression
            | SyntaxKind::LiteralExpression
            | SyntaxKind::CharacterStringLiteralExpression
            | SyntaxKind::DynamicParameterExpression
            | SyntaxKind::ParenthesizedExpression
            | SyntaxKind::PropertyAccessExpression
            | SyntaxKind::FunctionCallExpression
            | SyntaxKind::ListExpression
            | SyntaxKind::ByteStringLiteralExpression
            | SyntaxKind::TemporalLiteralExpression
            | SyntaxKind::DurationLiteralExpression
            | SyntaxKind::RecordExpression
            | SyntaxKind::SubscriptExpression
            | SyntaxKind::CaseExpression
    )
}

fn build_rowan_root(events: &[Event]) -> rowan::GreenNode {
    let mut builder = rowan::GreenNodeBuilder::new();
    for event in events {
        match event {
            Event::Start(kind) => builder.start_node(kind.to_rowan()),
            Event::Finish => {
                builder.finish_node();
            }
            Event::Token(token) => builder.token(token.syntax_kind().to_rowan(), token.text()),
        }
    }
    builder.finish()
}

pub(in crate::parser) struct Parser<'a> {
    pub(in crate::parser) tokens: &'a [Token],
    pub(in crate::parser) source: &'a str,
    pub(in crate::parser) index: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token], source: &'a str) -> Self {
        Self {
            tokens,
            source,
            index: 0,
            diagnostics: Vec::new(),
        }
    }

    fn collect_diagnostics(self, diagnostics: &mut Vec<Diagnostic>) {
        diagnostics.extend(self.diagnostics);
    }

    fn parse_top_level(&mut self) -> Vec<Event> {
        let mut children = Vec::new();

        while !self.at_eof() {
            let Some(TokenKind::Keyword(keyword)) = self.current_kind() else {
                children.push(self.bump_event());
                continue;
            };
            let Some(entrypoint) = top_level_parser_entrypoint(keyword) else {
                children.push(self.bump_event());
                continue;
            };
            children.extend(match entrypoint.action {
                GrammarParserAction::MatchClause => self.parse_match_clause(),
                GrammarParserAction::OptionalMatchClause => self.parse_optional_match_clause(),
                GrammarParserAction::ReturnClause => self.parse_return_clause(),
                GrammarParserAction::FinishStatement => self.parse_finish_statement(),
                GrammarParserAction::WhereClause => self.parse_where_clause(),
                GrammarParserAction::LetClause => self.parse_let_clause(),
                GrammarParserAction::FilterStatement => self.parse_filter_statement(),
                GrammarParserAction::ForStatement => self.parse_for_statement(),
                GrammarParserAction::UnionClause => self.parse_union_clause(),
                GrammarParserAction::LimitClause => self.parse_limit_clause(),
                GrammarParserAction::OrderByClause => self.parse_order_by_clause(),
                GrammarParserAction::OffsetClause => self.parse_offset_clause(),
                GrammarParserAction::GroupByClause => self.parse_group_by_clause(),
                GrammarParserAction::CreateSchemaStatement => self.parse_create_schema_statement(),
                GrammarParserAction::DropSchemaStatement => self.parse_drop_schema_statement(),
                GrammarParserAction::InsertStatement => self.parse_insert_statement(),
                GrammarParserAction::SetStatement => self.parse_set_statement(),
                GrammarParserAction::RemoveStatement => self.parse_remove_statement(),
                GrammarParserAction::DeleteStatement => self.parse_delete_statement(),
                GrammarParserAction::CallStatement => self.parse_call_statement(),
                GrammarParserAction::StartTransactionStatement => {
                    self.parse_start_transaction_statement()
                }
                GrammarParserAction::CommitStatement => self.parse_commit_statement(),
                GrammarParserAction::RollbackStatement => self.parse_rollback_statement(),
                GrammarParserAction::SessionSetStatement => self.parse_session_statement(),
            });
        }

        children
    }

    fn parse_optional_match_clause(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());

        if self.matches_kind(TokenKind::Keyword(Keyword::Match)) {
            children.extend(self.parse_match_clause());
        } else {
            self.emit_match_syntax(
                recovery_diagnostic("optional-match")
                    .expect("Gerbil grammar owns OPTIONAL MATCH recovery"),
                "OPTIONAL must be followed by MATCH",
                self.next_span_or(start),
            );
        }

        node(SyntaxKind::OptionalMatchClause, children)
    }

    fn parse_where_clause(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        let mut expressions = 0usize;

        loop {
            children.extend(self.skip_trivia());
            let Some(token) = self.current() else {
                break;
            };
            if self.is_clause_keyword(token.kind) {
                break;
            }

            if self.is_expression_start(token.kind) {
                children.extend(self.parse_expression());
                expressions += 1;
                children.extend(self.skip_trivia());
                if self.matches_kind(TokenKind::Punctuation(',')) {
                    children.push(self.bump_event());
                }
            } else if matches!(token.kind, TokenKind::Punctuation(_)) {
                children.push(self.bump_event());
            } else {
                self.emit_match_syntax(
                    recovery_diagnostic("where-clause")
                        .expect("Gerbil grammar owns where-clause recovery"),
                    "invalid token in WHERE clause",
                    token.span,
                );
                children.push(self.bump_event());
                break;
            }
        }

        if expressions == 0 {
            self.emit_match_syntax(
                recovery_diagnostic("where-clause")
                    .expect("Gerbil grammar owns where-clause recovery"),
                "WHERE requires at least one expression",
                Span::new(start, self.span_end()),
            );
        }

        node(SyntaxKind::WhereClause, children)
    }

    fn parse_let_clause(&mut self) -> Vec<Event> {
        let mut children = vec![self.bump_event()];
        loop {
            let mut binding = Vec::new();
            binding.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Identifier) {
                binding.push(self.bump_event());
            } else {
                if !self.at_eof()
                    && !self.is_clause_boundary(self.current_kind().unwrap_or(TokenKind::Unknown))
                {
                    binding.extend(self.parse_let_binding_expression());
                }
                children.extend(node(SyntaxKind::LetBinding, binding));
                break;
            }

            binding.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation('=')) {
                binding.push(self.bump_event());
            }
            binding.extend(self.skip_trivia());
            if self
                .current()
                .is_some_and(|token| !self.is_clause_boundary(token.kind))
            {
                binding.extend(self.parse_expression());
            }
            children.extend(node(SyntaxKind::LetBinding, binding));
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation(',')) {
                children.push(self.bump_event());
                continue;
            }
            break;
        }

        node(SyntaxKind::LetClause, children)
    }

    pub(super) fn parse_graph_pattern(&mut self) -> Vec<Event> {
        let mut children = self.parse_node_pattern();
        while self.matches_kind(TokenKind::Punctuation('-'))
            || self.matches_kind(TokenKind::Punctuation('<'))
        {
            children.extend(self.parse_graph_edge_sequence());
        }
        node(SyntaxKind::GraphPattern, children)
    }

    fn parse_graph_edge_sequence(&mut self) -> Vec<Event> {
        let edge_start = self.span_start();
        let mut edge_children = Vec::new();
        edge_children.push(self.bump_event());

        if self.matches_kind(TokenKind::Punctuation('-'))
            && self.previous_kind() == Some(TokenKind::Punctuation('<'))
        {
            edge_children.push(self.bump_event());
        }
        edge_children.extend(self.skip_trivia());

        if !self.matches_kind(TokenKind::Punctuation('[')) {
            self.emit_match_syntax(
                "GQL-PARSE-MATCH-SYNTAX",
                "edge delimiter missing '['",
                self.next_span_or(edge_start),
            );
            let mut children = node(SyntaxKind::EdgePattern, edge_children);
            children.extend(self.skip_trivia());
            return children;
        }

        edge_children.push(self.bump_event());
        edge_children.extend(self.skip_trivia());
        edge_children.extend(self.parse_label_list());
        edge_children.extend(self.skip_trivia());

        if self.matches_kind(TokenKind::Punctuation('{')) {
            edge_children.extend(self.parse_property_map());
            edge_children.extend(self.skip_trivia());
        }

        if self.matches_keyword(Keyword::Where) {
            edge_children.extend(self.parse_inline_where_clause(
                "inline-edge-where",
                "inline edge WHERE",
                TokenKind::Punctuation(']'),
            ));
            edge_children.extend(self.skip_trivia());
        }

        if self.matches_kind(TokenKind::Punctuation(']')) {
            edge_children.push(self.bump_event());
        } else {
            self.emit_match_syntax(
                "GQL-PARSE-MATCH-SYNTAX",
                "edge label list missing closing bracket",
                self.next_span_or(self.span_end()),
            );
        }

        edge_children.extend(self.skip_trivia());
        while matches!(
            self.current_kind(),
            Some(TokenKind::Punctuation('-' | '>' | '<'))
        ) {
            edge_children.push(self.bump_event());
            edge_children.extend(self.skip_trivia());
        }

        if matches!(
            self.current_kind(),
            Some(TokenKind::Punctuation('{' | '*' | '+' | '?'))
        ) {
            edge_children.extend(self.parse_path_quantifier(edge_start));
        }

        let mut children = node(SyntaxKind::EdgePattern, edge_children);
        children.extend(self.skip_trivia());
        if self.matches_kind(TokenKind::Punctuation('(')) {
            children.extend(self.parse_node_pattern());
        }
        children
    }

    fn parse_path_quantifier(&mut self, start: u32) -> Vec<Event> {
        let diagnostic = recovery_diagnostic("path-quantifier")
            .expect("Gerbil grammar owns path quantifier recovery");
        if matches!(
            self.current_kind(),
            Some(TokenKind::Punctuation('*' | '+' | '?'))
        ) {
            return node(SyntaxKind::PathQuantifier, vec![self.bump_event()]);
        }

        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());

        let min = self
            .current()
            .and_then(|token| (token.kind == TokenKind::Number).then(|| token.text().to_string()));
        if min.is_some() {
            children.push(self.bump_event());
        }
        children.extend(self.skip_trivia());

        let mut max = None;
        let has_comma = self.matches_kind(TokenKind::Punctuation(','));
        if has_comma {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
            max = self.current().and_then(|token| {
                (token.kind == TokenKind::Number).then(|| token.text().to_string())
            });
            if max.is_some() {
                children.push(self.bump_event());
            }
            children.extend(self.skip_trivia());
        }

        if min.is_none() && max.is_none() {
            self.emit_match_syntax(
                diagnostic,
                "path quantifier requires a minimum or maximum bound",
                self.next_span_or(start),
            );
        }

        if self.matches_kind(TokenKind::Punctuation('}')) {
            children.push(self.bump_event());
        } else {
            self.emit_match_syntax(
                diagnostic,
                "path quantifier is missing `}`",
                self.next_span_or(start),
            );
        }

        if let (Some(min), Some(max)) = (
            min.as_deref().and_then(|value| value.parse::<u32>().ok()),
            max.as_deref().and_then(|value| value.parse::<u32>().ok()),
        ) && max < min
        {
            self.emit_match_syntax(
                diagnostic,
                "path quantifier maximum must not be less than minimum",
                Span::new(start, self.span_end()),
            );
        }

        node(SyntaxKind::PathQuantifier, children)
    }

    fn parse_label_list(&mut self) -> Vec<Event> {
        let mut children = Vec::new();
        let mut previous_was_name = false;
        let mut in_labels = false;
        loop {
            children.extend(self.skip_trivia());
            match self.current_kind() {
                Some(TokenKind::Punctuation(']')) | None => break,
                Some(TokenKind::Keyword(Keyword::Where)) => break,
                Some(TokenKind::Identifier | TokenKind::Keyword(_) | TokenKind::Number) => {
                    if previous_was_name {
                        self.emit_match_syntax(
                            recovery_diagnostic("edge-label-separator")
                                .expect("Gerbil grammar owns edge label separator recovery"),
                            "edge binding and labels require `:` separators",
                            self.current()
                                .map(|token| token.span)
                                .unwrap_or_else(|| self.next_span_or(self.span_end())),
                        );
                    }
                    if !in_labels && !previous_was_name && !self.matches_regular_identifier() {
                        let span = self
                            .current()
                            .map(|token| token.span)
                            .unwrap_or_else(|| self.next_span_or(self.span_end()));
                        self.emit_match_syntax(
                            recovery_diagnostic("binding-variable")
                                .expect("Gerbil grammar owns binding variable recovery"),
                            "edge binding variable requires a regular identifier",
                            span,
                        );
                    }
                    children.push(self.bump_event());
                    previous_was_name = true;
                }
                Some(TokenKind::Punctuation(':')) => {
                    children.push(self.bump_event());
                    previous_was_name = false;
                    in_labels = true;
                }
                Some(_) => break,
            }
        }
        node(SyntaxKind::LabelList, children)
    }

    fn parse_node_pattern(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = Vec::new();
        if !self.matches_kind(TokenKind::Punctuation('(')) {
            self.emit_match_syntax(
                "GQL-PARSE-MATCH-SYNTAX",
                "node pattern missing opening '('",
                self.next_span_or(start),
            );
            return node(SyntaxKind::NodePattern, children);
        }

        children.push(self.bump_event());
        children.extend(self.skip_trivia());
        if self.matches_regular_identifier() {
            children.push(self.bump_event());
        } else if self.matches_identifier()
            || matches!(self.current_kind(), Some(TokenKind::Keyword(keyword)) if keyword != Keyword::Where)
        {
            let span = self
                .current()
                .map(|token| token.span)
                .unwrap_or_else(|| self.next_span_or(start));
            self.emit_match_syntax(
                recovery_diagnostic("binding-variable")
                    .expect("Gerbil grammar owns binding variable recovery"),
                "node binding variable requires a regular identifier",
                span,
            );
            children.push(self.bump_event());
        }
        let mut closed = false;
        while !self.at_eof() {
            if self.matches_kind(TokenKind::Punctuation(')')) {
                closed = true;
                children.push(self.bump_event());
                break;
            }
            if self.matches_kind(TokenKind::Punctuation('{')) {
                children.extend(self.parse_property_map());
                continue;
            }
            if self.matches_keyword(Keyword::Where) {
                children.extend(self.parse_inline_where_clause(
                    "inline-node-where",
                    "inline node WHERE",
                    TokenKind::Punctuation(')'),
                ));
                continue;
            }
            children.push(self.bump_event());
        }
        if !closed {
            self.emit_match_syntax(
                "GQL-PARSE-MATCH-SYNTAX",
                "node pattern missing closing ')'",
                Span::new(start, self.span_end()),
            );
        }
        node(SyntaxKind::NodePattern, children)
    }

    fn parse_inline_where_clause(
        &mut self,
        recovery_site: &'static str,
        subject: &'static str,
        closing_delimiter: TokenKind,
    ) -> Vec<Event> {
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        if self.at_eof() || self.matches_kind(closing_delimiter) {
            let diagnostic_span = self.current().map_or_else(
                || {
                    let end = self.span_end();
                    Span::new(end, end)
                },
                |token| token.span,
            );
            self.emit_match_syntax(
                recovery_diagnostic(recovery_site)
                    .expect("Gerbil grammar owns inline pattern WHERE recovery"),
                format!("{subject} requires a predicate expression"),
                diagnostic_span,
            );
        } else {
            children.extend(self.parse_expression());
        }
        node(SyntaxKind::InlineWhereClause, children)
    }

    fn parse_property_map(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];

        loop {
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation('}')) {
                children.push(self.bump_event());
                break;
            }
            if self.at_eof()
                || self.matches_kind(TokenKind::Punctuation(')'))
                || self.matches_kind(TokenKind::Punctuation(']'))
            {
                self.emit_match_syntax(
                    "GQL-PARSE-PROPERTY-MAP-SYNTAX",
                    "property map is missing `}`",
                    self.next_span_or(start),
                );
                break;
            }

            let mut entry = Vec::new();
            if self.matches_identifier() {
                entry.push(self.bump_event());
            } else {
                self.emit_match_syntax(
                    "GQL-PARSE-PROPERTY-MAP-SYNTAX",
                    "property entry requires a property name",
                    self.next_span_or(start),
                );
                entry.push(self.bump_event());
            }
            entry.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation(':')) {
                entry.push(self.bump_event());
            } else {
                self.emit_match_syntax(
                    "GQL-PARSE-PROPERTY-MAP-SYNTAX",
                    "property entry requires `:`",
                    self.next_span_or(start),
                );
            }
            entry.extend(self.skip_trivia());
            if self.at_eof()
                || self.matches_kind(TokenKind::Punctuation(','))
                || self.matches_kind(TokenKind::Punctuation('}'))
                || self.matches_kind(TokenKind::Punctuation(')'))
            {
                self.emit_match_syntax(
                    "GQL-PARSE-PROPERTY-MAP-SYNTAX",
                    "property entry requires a value expression",
                    self.next_span_or(start),
                );
            } else {
                entry.extend(self.parse_expression());
            }
            children.extend(node(SyntaxKind::PropertyEntry, entry));
            children.extend(self.skip_trivia());

            if self.matches_kind(TokenKind::Punctuation(',')) {
                children.push(self.bump_event());
                continue;
            }
            if self.matches_kind(TokenKind::Punctuation('}')) {
                children.push(self.bump_event());
                break;
            }
            self.emit_match_syntax(
                "GQL-PARSE-PROPERTY-MAP-SYNTAX",
                "property map requires `,` or `}`",
                self.next_span_or(start),
            );
            if !self.at_eof() && !self.matches_kind(TokenKind::Punctuation(')')) {
                children.push(self.bump_event());
            }
        }

        node(SyntaxKind::PropertyMap, children)
    }

    pub(in crate::parser) fn parse_expression(&mut self) -> Vec<Event> {
        self.parse_binary_expression(0)
    }

    fn parse_binary_expression(&mut self, minimum_precedence: u8) -> Vec<Event> {
        let mut lhs = self.parse_prefix_expression();
        loop {
            let operator_start = self.index;
            let trivia = self.skip_trivia();
            if self.at_eof() || self.is_expression_boundary() {
                self.index = operator_start;
                break;
            }
            if self.matches_kind(TokenKind::Punctuation('!'))
                && self.peek_kind(1) == Some(TokenKind::Punctuation('='))
            {
                let span = Span::new(self.span_start(), self.tokens[self.index + 1].span.end);
                self.emit_match_syntax(
                    recovery_diagnostic("non-iso-operator")
                        .expect("Gerbil grammar owns non-ISO operator recovery"),
                    "ISO GQL uses `<>` for inequality; `!=` is not admitted",
                    span,
                );
                let mut children = lhs;
                children.extend(trivia);
                children.push(self.bump_event());
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
                children.extend(self.parse_binary_expression(31));
                lhs = node(SyntaxKind::BinaryExpression, children);
                continue;
            }
            if self.is_label_predicate_start() {
                if 30 < minimum_precedence {
                    self.index = operator_start;
                    break;
                }
                lhs = self.parse_label_predicate_suffix(lhs, trivia);
                continue;
            }
            if self.matches_kind(TokenKind::Keyword(Keyword::Is)) {
                if 30 < minimum_precedence {
                    self.index = operator_start;
                    break;
                }
                lhs = self.parse_predicate_test_suffix(lhs, trivia);
                continue;
            }
            let Some(specification) = self
                .current_kind()
                .and_then(|first| binary_operator_spec(first, self.peek_kind(1)))
            else {
                self.index = operator_start;
                break;
            };
            if specification.left_binding_power < minimum_precedence {
                self.index = operator_start;
                break;
            }
            let mut children = lhs;
            children.extend(trivia);
            for _ in 0..specification.width {
                children.push(self.bump_event());
            }
            children.extend(self.skip_trivia());
            children.extend(self.parse_binary_expression(specification.right_binding_power));
            lhs = node(SyntaxKind::BinaryExpression, children);
        }
        lhs
    }

    fn parse_predicate_test_suffix(
        &mut self,
        operand: Vec<Event>,
        trivia: Vec<Event>,
    ) -> Vec<Event> {
        let operand_is_primary = operand.first().is_some_and(
            |event| matches!(event, Event::Start(kind) if is_value_expression_primary_kind(*kind)),
        );
        let operand_is_element_reference = operand
            .first()
            .is_some_and(|event| matches!(event, Event::Start(SyntaxKind::NameExpression)));
        let start = self.span_start();
        let mut children = operand;
        children.extend(trivia);
        children.push(self.bump_event());
        children.extend(self.skip_trivia());

        if self.matches_kind(TokenKind::Keyword(Keyword::Not)) {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
        }

        if self.matches_kind(TokenKind::Keyword(Keyword::Typed))
            || self.matches_kind(TokenKind::Punctuation(':'))
        {
            return self.parse_value_type_predicate_suffix(children, operand_is_primary, start);
        }
        if self.matches_word("DIRECTED")
            || self.matches_word("SOURCE")
            || self.matches_word("DESTINATION")
        {
            return self.parse_graph_element_predicate_suffix(
                children,
                operand_is_element_reference,
                start,
            );
        }

        let kind = match self.current_kind() {
            Some(TokenKind::Keyword(Keyword::Null)) => {
                if !operand_is_primary {
                    self.emit_match_syntax(
                        recovery_diagnostic("null-predicate-operand")
                            .expect("Gerbil grammar owns NULL predicate operand recovery"),
                        "IS NULL requires a value expression primary; parenthesize a composite expression",
                        Span::new(start, self.current().map_or(self.span_end(), |token| token.span.end)),
                    );
                }
                SyntaxKind::NullPredicateExpression
            }
            Some(TokenKind::Keyword(Keyword::True | Keyword::False | Keyword::UnknownTruth)) => {
                SyntaxKind::TruthPredicateExpression
            }
            _ => {
                self.emit_match_syntax(
                    recovery_diagnostic("predicate-test")
                        .expect("Gerbil grammar owns predicate-test recovery"),
                    "IS predicate requires NULL, TRUE, FALSE, or UNKNOWN",
                    Span::new(start, self.span_end()),
                );
                return node(SyntaxKind::TruthPredicateExpression, children);
            }
        };
        children.push(self.bump_event());
        node(kind, children)
    }

    fn parse_prefix_expression(&mut self) -> Vec<Event> {
        if let Some(kind) = self.current_kind()
            && let Some(precedence) = prefix_operator_precedence(kind)
        {
            let mut children = vec![self.bump_event()];
            children.extend(self.skip_trivia());
            children.extend(self.parse_binary_expression(precedence));
            return node(SyntaxKind::UnaryExpression, children);
        }
        self.parse_primary_expression()
    }

    fn parse_let_binding_expression(&mut self) -> Vec<Event> {
        self.parse_primary_expression()
    }

    fn parse_primary_expression(&mut self) -> Vec<Event> {
        if self.matches_regular_identifier() {
            let identifier = self.bump_event();
            let expression = node(SyntaxKind::NameExpression, vec![identifier]);
            return self.parse_postfix_expression(expression);
        }
        let base = match self.current_kind() {
            None => node(SyntaxKind::Expression, Vec::new()),
            Some(TokenKind::Keyword(Keyword::Case)) => self.parse_case_expression(),
            Some(TokenKind::Keyword(keyword)) if aggregate_function_spec(keyword).is_some() => {
                return self.parse_aggregate_function();
            }
            Some(TokenKind::Keyword(
                Keyword::AllDifferent | Keyword::Same | Keyword::PropertyExists,
            )) => return self.parse_graph_element_predicate_function(),
            Some(TokenKind::Punctuation('[')) => self.parse_list_expression(),
            Some(TokenKind::Punctuation('{')) => self.parse_record_expression(false),
            Some(TokenKind::Keyword(Keyword::Record)) => self.parse_record_expression(true),
            Some(TokenKind::Keyword(
                Keyword::Date | Keyword::Time | Keyword::Timestamp | Keyword::Datetime,
            )) => self.parse_temporal_literal(),
            Some(TokenKind::Keyword(Keyword::Duration)) => self.parse_duration_literal(),
            Some(TokenKind::Punctuation('(')) => {
                let mut children = vec![self.bump_event()];
                children.extend(self.skip_trivia());
                if !self.at_eof() && !self.matches_kind(TokenKind::Punctuation(')')) {
                    children.extend(self.parse_binary_expression(0));
                    children.extend(self.skip_trivia());
                }
                if self.matches_kind(TokenKind::Punctuation(')')) {
                    children.push(self.bump_event());
                }
                node(SyntaxKind::ParenthesizedExpression, children)
            }
            Some(TokenKind::String | TokenKind::Number | TokenKind::ByteString) => {
                let kind = match self.current_kind() {
                    Some(TokenKind::ByteString) => SyntaxKind::ByteStringLiteralExpression,
                    Some(TokenKind::String) => SyntaxKind::CharacterStringLiteralExpression,
                    Some(TokenKind::Number) => SyntaxKind::LiteralExpression,
                    _ => SyntaxKind::NameExpression,
                };
                node(kind, vec![self.bump_event()])
            }
            Some(TokenKind::DynamicParameter) => node(
                SyntaxKind::DynamicParameterExpression,
                vec![self.bump_event()],
            ),
            Some(TokenKind::SubstitutedParameter) => {
                let span = self.current().map(|token| token.span).unwrap_or_else(|| {
                    let end = self.span_end();
                    Span::new(end, end)
                });
                self.emit_match_syntax(
                    recovery_diagnostic("substituted-parameter-context")
                        .expect("Gerbil grammar owns substituted parameter context recovery"),
                    "substituted parameter references are not value expressions",
                    span,
                );
                node(SyntaxKind::Expression, vec![self.bump_event()])
            }
            Some(TokenKind::Keyword(Keyword::True | Keyword::False | Keyword::Null)) => {
                node(SyntaxKind::LiteralExpression, vec![self.bump_event()])
            }
            Some(TokenKind::Identifier) => {
                if self.current().is_some_and(|token| {
                    token.text().starts_with('"') || token.text().starts_with("@\"")
                }) {
                    let token = self.bump_event();
                    let expression =
                        node(SyntaxKind::CharacterStringLiteralExpression, vec![token]);
                    return self.parse_postfix_expression(expression);
                }
                let span = self.current().map(|token| token.span).unwrap_or_else(|| {
                    let end = self.span_end();
                    Span::new(end, end)
                });
                self.emit_match_syntax(
                    recovery_diagnostic("binding-variable")
                        .expect("Gerbil grammar owns binding variable recovery"),
                    "binding variable requires a regular identifier",
                    span,
                );
                node(SyntaxKind::Expression, vec![self.bump_event()])
            }
            Some(TokenKind::Keyword(keyword)) => {
                let span = self.current().map(|token| token.span).unwrap_or_else(|| {
                    let end = self.span_end();
                    Span::new(end, end)
                });
                self.emit_match_syntax(
                    recovery_diagnostic("unsupported-keyword-expression")
                        .expect("Gerbil grammar owns unsupported keyword recovery"),
                    format!(
                        "{} is reserved but unsupported in an expression by the active GQL frontend profile",
                        keyword_name(keyword)
                    ),
                    span,
                );
                node(SyntaxKind::Expression, vec![self.bump_event()])
            }
            Some(_) => node(SyntaxKind::NameExpression, vec![self.bump_event()]),
        };

        self.parse_postfix_expression(base)
    }

    pub(in crate::parser) fn skip_trivia(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        while matches!(
            self.current_kind(),
            Some(TokenKind::Whitespace | TokenKind::Comment)
        ) {
            events.push(self.bump_event());
        }
        events
    }

    pub(in crate::parser) fn emit_match_syntax(
        &mut self,
        code: &'static str,
        message: impl Into<String>,
        span: Span,
    ) {
        self.diagnostics
            .push(Diagnostic::error(code, message, span));
    }

    pub(in crate::parser) fn emit_return_syntax(
        &mut self,
        code: &'static str,
        message: impl Into<String>,
        span: Span,
    ) {
        self.diagnostics
            .push(Diagnostic::error(code, message, span));
    }

    pub(in crate::parser) fn at_eof(&self) -> bool {
        self.index >= self.tokens.len()
    }

    pub(in crate::parser) fn current(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    pub(in crate::parser) fn current_kind(&self) -> Option<TokenKind> {
        self.current().map(|token| token.kind)
    }

    pub(in crate::parser) fn bump_event(&mut self) -> Event {
        Event::Token(self.bump_token())
    }

    fn bump_token(&mut self) -> Token {
        let token = self.tokens.get(self.index).cloned().unwrap_or_else(|| {
            let end = self.source.len() as u32;
            Token::new(TokenKind::Unknown, Span::new(end, end), "")
        });
        self.index += 1;
        token
    }

    pub(in crate::parser) fn span_start(&self) -> u32 {
        self.current()
            .map(|token| token.span.start)
            .unwrap_or_else(|| self.span_end())
    }

    pub(in crate::parser) fn span_end(&self) -> u32 {
        self.index
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
            .map(|token| token.span.end)
            .or_else(|| self.tokens.last().map(|token| token.span.end))
            .unwrap_or(0)
    }

    pub(in crate::parser) fn next_span_or(&self, default_start: u32) -> Span {
        Span::new(
            default_start,
            self.current()
                .map(|token| token.span.end)
                .unwrap_or(default_start),
        )
    }
}
