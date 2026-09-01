//! Lossless event parser for the supported ISO GQL syntax slice.

#![forbid(unsafe_code)]

use gql_source::{Diagnostic, SourceText, Span};

use crate::syntax::{
    GrammarParserAction, Keyword, SyntaxKind, SyntaxTree, Token, TokenKind, binary_operator_spec,
    prefix_operator_precedence, recovery_diagnostic, top_level_parser_entrypoint,
};

/// Parser output consumed directly by the Rowan tree sink.
#[derive(Clone, Debug)]
pub(super) enum Event {
    Start(SyntaxKind),
    Finish,
    Token(Token),
}

/// Parses `source` into one lossless Rowan CST and diagnostics.
pub fn parse(name: &str, source: &str) -> crate::Parse {
    let source = SourceText::new(name, source);
    let (tokens, mut diagnostics) = crate::lexer::lex(source.text());
    let mut parser = Parser::new(&tokens, source.text());
    let (children, has_match, saw_return) = parser.parse_top_level();
    parser.collect_diagnostics(&mut diagnostics);

    if !has_match && saw_return {
        diagnostics.push(Diagnostic::error(
            "GQL-PARSE-MISSING-KEYWORD",
            "expected MATCH before RETURN in this revision",
            Span::new(0, source.text().len() as u32),
        ));
    }

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

pub(super) fn node(kind: SyntaxKind, mut children: Vec<Event>) -> Vec<Event> {
    let mut events = Vec::with_capacity(children.len() + 2);
    events.push(Event::Start(kind));
    events.append(&mut children);
    events.push(Event::Finish);
    events
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

pub(super) struct Parser<'a> {
    tokens: &'a [Token],
    source: &'a str,
    index: usize,
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

    fn parse_top_level(&mut self) -> (Vec<Event>, bool, bool) {
        let mut children = Vec::new();
        let mut saw_match = false;
        let mut saw_return = false;

        while !self.at_eof() {
            let Some(TokenKind::Keyword(keyword)) = self.current_kind() else {
                children.push(self.bump_event());
                continue;
            };
            let Some(entrypoint) = top_level_parser_entrypoint(keyword) else {
                children.push(self.bump_event());
                continue;
            };
            saw_match |= entrypoint.marks_match;
            saw_return |= entrypoint.marks_return;
            children.extend(match entrypoint.action {
                GrammarParserAction::MatchClause => self.parse_match_clause(),
                GrammarParserAction::OptionalMatchClause => self.parse_optional_match_clause(),
                GrammarParserAction::ReturnClause => self.parse_return_clause(),
                GrammarParserAction::WhereClause => self.parse_where_clause(),
                GrammarParserAction::LetClause => self.parse_let_clause(),
                GrammarParserAction::UnionClause => {
                    node(SyntaxKind::UnionClause, vec![self.bump_event()])
                }
                GrammarParserAction::LimitClause => self.parse_limit_clause(),
                GrammarParserAction::OrderByClause => self.parse_order_by_clause(),
                GrammarParserAction::OffsetClause => self.parse_offset_clause(),
                GrammarParserAction::UnsupportedStatement => {
                    self.parse_unsupported_statement(keyword)
                }
            });
        }

        (children, saw_match, saw_return)
    }

    fn parse_unsupported_statement(&mut self, keyword: Keyword) -> Vec<Event> {
        let span = self.current().map(|token| token.span).unwrap_or_else(|| {
            let end = self.span_end();
            Span::new(end, end)
        });
        self.emit_match_syntax(
            recovery_diagnostic("unsupported-statement")
                .expect("Gerbil grammar owns unsupported-statement recovery"),
            format!(
                "{} statements are reserved but unsupported by the active GQL frontend profile",
                keyword_name(keyword)
            ),
            span,
        );
        vec![self.bump_event()]
    }

    fn parse_match_clause(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());

        if self.looks_like_named_path_pattern() {
            children.extend(self.parse_named_path_pattern());
        } else if self.matches_kind(TokenKind::Punctuation('(')) {
            children.extend(self.parse_graph_pattern());
        } else {
            self.emit_match_syntax(
                "GQL-PARSE-MATCH-SYNTAX",
                "MATCH clause must start with a graph pattern",
                self.next_span_or(start),
            );
        }

        node(SyntaxKind::MatchClause, children)
    }

    fn parse_optional_match_clause(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());

        if self.matches_kind(TokenKind::Keyword(Keyword::Match)) {
            children.extend(self.parse_match_clause());
        } else {
            self.emit_match_syntax(
                "GQL-PARSE-OPTIONAL-MATCH-SYNTAX",
                "OPTIONAL must be followed by MATCH",
                self.next_span_or(start),
            );
        }

        node(SyntaxKind::OptionalMatchClause, children)
    }

    fn looks_like_named_path_pattern(&self) -> bool {
        if self.current_kind() != Some(TokenKind::Identifier) {
            return false;
        }

        let mut offset = 1;
        while matches!(
            self.tokens.get(self.index + offset).map(|token| token.kind),
            Some(TokenKind::Whitespace | TokenKind::Comment)
        ) {
            offset += 1;
        }

        matches!(
            self.tokens.get(self.index + offset).map(|token| token.kind),
            Some(TokenKind::Punctuation('='))
        )
    }

    fn parse_named_path_pattern(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());

        if self.matches_kind(TokenKind::Punctuation('=')) {
            children.push(self.bump_event());
        } else {
            self.emit_match_syntax(
                "GQL-PARSE-PATH-SYNTAX",
                "named path pattern must contain `=`",
                self.next_span_or(start),
            );
        }
        children.extend(self.skip_trivia());

        if self.matches_kind(TokenKind::Punctuation('(')) {
            children.extend(self.parse_graph_pattern());
        } else {
            self.emit_match_syntax(
                "GQL-PARSE-PATH-SYNTAX",
                "named path pattern must contain a graph pattern",
                self.next_span_or(start),
            );
        }

        node(SyntaxKind::PathPattern, children)
    }

    fn parse_return_clause(&mut self) -> Vec<Event> {
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
        if self.matches_kind(TokenKind::Identifier) {
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

    fn parse_limit_clause(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        if self.matches_kind(TokenKind::Number) {
            children.push(self.bump_event());
        } else {
            self.emit_return_syntax(
                "GQL-PARSE-LIMIT-SYNTAX",
                "LIMIT requires a positive integer literal",
                self.next_span_or(start),
            );
        }
        node(SyntaxKind::LimitClause, children)
    }

    fn parse_order_by_clause(&mut self) -> Vec<Event> {
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
                .is_some_and(|kind| self.is_expression_start(kind))
            {
                self.emit_return_syntax(
                    "GQL-PARSE-ORDER-BY-SYNTAX",
                    "ORDER BY requires at least one expression",
                    self.next_span_or(start),
                );
                break;
            }
            children.extend(self.parse_expression());
            keys += 1;
            children.extend(self.skip_trivia());
            if self.matches_keyword(Keyword::Asc) || self.matches_keyword(Keyword::Desc) {
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
                "GQL-PARSE-ORDER-BY-SYNTAX",
                "ORDER BY requires at least one expression",
                Span::new(start, self.span_end()),
            );
        }
        node(SyntaxKind::OrderByClause, children)
    }

    fn parse_offset_clause(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        if self.matches_kind(TokenKind::Number) {
            children.push(self.bump_event());
        } else {
            self.emit_return_syntax(
                "GQL-PARSE-OFFSET-SYNTAX",
                "OFFSET requires a non-negative integer literal",
                self.next_span_or(start),
            );
        }
        node(SyntaxKind::OffsetClause, children)
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
                    "GQL-PARSE-WHERE-SYNTAX",
                    "invalid token in WHERE clause",
                    token.span,
                );
                children.push(self.bump_event());
                break;
            }
        }

        if expressions == 0 {
            self.emit_match_syntax(
                "GQL-PARSE-WHERE-SYNTAX",
                "WHERE requires at least one expression",
                Span::new(start, self.span_end()),
            );
        }

        node(SyntaxKind::WhereClause, children)
    }

    fn parse_let_clause(&mut self) -> Vec<Event> {
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        if self
            .current()
            .is_some_and(|token| !self.is_clause_boundary(token.kind))
        {
            children.extend(self.parse_let_binding_expression());
        }

        children.extend(self.skip_trivia());
        if self.matches_kind(TokenKind::Punctuation('=')) {
            children.push(self.bump_event());
        }

        children.extend(self.skip_trivia());
        if self
            .current()
            .is_some_and(|token| !self.is_clause_boundary(token.kind))
        {
            children.extend(self.parse_expression());
        }

        node(SyntaxKind::LetClause, children)
    }

    fn parse_graph_pattern(&mut self) -> Vec<Event> {
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

        if self.matches_kind(TokenKind::Punctuation('{')) {
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
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());

        let min = self
            .current()
            .and_then(|token| (token.kind == TokenKind::Number).then(|| token.text().to_string()));
        if min.is_some() {
            children.push(self.bump_event());
        } else {
            self.emit_match_syntax(
                "GQL-PARSE-PATH-QUANTIFIER",
                "path quantifier requires a minimum number",
                self.next_span_or(start),
            );
        }
        children.extend(self.skip_trivia());

        let mut max = None;
        if self.matches_kind(TokenKind::Punctuation(',')) {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
            max = self.current().and_then(|token| {
                (token.kind == TokenKind::Number).then(|| token.text().to_string())
            });
            if max.is_some() {
                children.push(self.bump_event());
            } else {
                self.emit_match_syntax(
                    "GQL-PARSE-PATH-QUANTIFIER",
                    "path quantifier requires a maximum number after `,`",
                    self.next_span_or(start),
                );
            }
            children.extend(self.skip_trivia());
        }

        if self.matches_kind(TokenKind::Punctuation('}')) {
            children.push(self.bump_event());
        } else {
            self.emit_match_syntax(
                "GQL-PARSE-PATH-QUANTIFIER",
                "path quantifier is missing `}`",
                self.next_span_or(start),
            );
        }

        if min.as_deref().and_then(|value| value.parse::<u32>().ok()) == Some(0) {
            self.emit_match_syntax(
                "GQL-PARSE-PATH-QUANTIFIER",
                "path quantifier minimum must be greater than zero",
                Span::new(start, self.span_end()),
            );
        }
        if let (Some(min), Some(max)) = (
            min.as_deref().and_then(|value| value.parse::<u32>().ok()),
            max.as_deref().and_then(|value| value.parse::<u32>().ok()),
        ) && max < min
        {
            self.emit_match_syntax(
                "GQL-PARSE-PATH-QUANTIFIER",
                "path quantifier maximum must not be less than minimum",
                Span::new(start, self.span_end()),
            );
        }

        node(SyntaxKind::PathQuantifier, children)
    }

    fn parse_label_list(&mut self) -> Vec<Event> {
        let mut children = Vec::new();
        loop {
            children.extend(self.skip_trivia());
            match self.current_kind() {
                Some(TokenKind::Punctuation(']')) | None => break,
                Some(
                    TokenKind::Identifier
                    | TokenKind::Keyword(_)
                    | TokenKind::Number
                    | TokenKind::Punctuation(':'),
                ) => children.push(self.bump_event()),
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
            if matches!(
                self.current_kind(),
                Some(TokenKind::Identifier | TokenKind::Keyword(_))
            ) {
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

    pub(super) fn parse_expression(&mut self) -> Vec<Event> {
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

    fn parse_prefix_expression(&mut self) -> Vec<Event> {
        if let Some(TokenKind::Keyword(keyword)) = self.current_kind()
            && let Some(precedence) = prefix_operator_precedence(keyword)
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
        let base = match self.current_kind() {
            None => node(SyntaxKind::Expression, Vec::new()),
            Some(TokenKind::Keyword(Keyword::Case)) => self.parse_case_expression(),
            Some(TokenKind::Punctuation('[')) => self.parse_list_expression(),
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
            Some(TokenKind::Identifier | TokenKind::String | TokenKind::Number) => {
                let kind = match self.current_kind() {
                    Some(TokenKind::Number | TokenKind::String) => SyntaxKind::LiteralExpression,
                    _ => SyntaxKind::NameExpression,
                };
                node(kind, vec![self.bump_event()])
            }
            Some(TokenKind::Keyword(Keyword::True | Keyword::False | Keyword::Null)) => {
                node(SyntaxKind::LiteralExpression, vec![self.bump_event()])
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

        let mut expression = base;
        loop {
            let access_start = self.index;
            let trivia = self.skip_trivia();
            if self.matches_kind(TokenKind::Punctuation('.')) {
                let mut children = expression;
                children.extend(trivia);
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
                if self.matches_kind(TokenKind::Identifier) {
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
                expression = node(SyntaxKind::SubscriptExpression, children);
                continue;
            }

            self.index = access_start;
            break;
        }

        expression
    }

    fn parse_list_expression(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];

        loop {
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation(']')) {
                children.push(self.bump_event());
                break;
            }
            if self.at_eof() {
                self.emit_match_syntax(
                    "GQL-PARSE-LIST-SYNTAX",
                    "list value is missing `]`",
                    self.next_span_or(start),
                );
                break;
            }

            children.extend(self.parse_expression());
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation(',')) {
                children.push(self.bump_event());
                continue;
            }
            if self.matches_kind(TokenKind::Punctuation(']')) {
                children.push(self.bump_event());
                break;
            }

            self.emit_match_syntax(
                "GQL-PARSE-LIST-SYNTAX",
                "list value requires `,` or `]`",
                self.next_span_or(start),
            );
            if !self.at_eof() {
                children.push(self.bump_event());
            }
        }

        node(SyntaxKind::ListExpression, children)
    }

    pub(super) fn skip_trivia(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        while matches!(
            self.current_kind(),
            Some(TokenKind::Whitespace | TokenKind::Comment)
        ) {
            events.push(self.bump_event());
        }
        events
    }

    fn is_expression_start(&self, kind: TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Identifier
                | TokenKind::String
                | TokenKind::Number
                | TokenKind::Keyword(_)
                | TokenKind::Punctuation('(')
                | TokenKind::Punctuation('[')
        )
    }

    fn is_clause_keyword(&self, kind: TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Keyword(Keyword::Match)
                | TokenKind::Keyword(Keyword::Optional)
                | TokenKind::Keyword(Keyword::Where)
                | TokenKind::Keyword(Keyword::Let)
                | TokenKind::Keyword(Keyword::Return)
                | TokenKind::Keyword(Keyword::Union)
                | TokenKind::Keyword(Keyword::Limit)
                | TokenKind::Keyword(Keyword::Order)
                | TokenKind::Keyword(Keyword::Offset)
        )
    }

    fn is_clause_boundary(&self, kind: TokenKind) -> bool {
        self.is_clause_keyword(kind) || kind == TokenKind::Punctuation(',')
    }

    fn is_expression_boundary(&self) -> bool {
        self.current_kind().is_none_or(|kind| {
            self.is_clause_boundary(kind)
                || matches!(
                    kind,
                    TokenKind::Punctuation(')' | ']')
                        | TokenKind::Keyword(
                            Keyword::When | Keyword::Then | Keyword::Else | Keyword::End
                        )
                )
        })
    }

    fn matches_kind(&self, kind: TokenKind) -> bool {
        self.current_kind() == Some(kind)
    }

    pub(super) fn matches_keyword(&self, keyword: Keyword) -> bool {
        self.matches_kind(TokenKind::Keyword(keyword))
    }

    fn peek_kind(&self, offset: usize) -> Option<TokenKind> {
        self.tokens.get(self.index + offset).map(|token| token.kind)
    }

    fn previous_kind(&self) -> Option<TokenKind> {
        self.index
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
            .map(|token| token.kind)
    }

    pub(super) fn emit_match_syntax(
        &mut self,
        code: &'static str,
        message: impl Into<String>,
        span: Span,
    ) {
        self.diagnostics
            .push(Diagnostic::error(code, message, span));
    }

    fn emit_return_syntax(&mut self, code: &'static str, message: impl Into<String>, span: Span) {
        self.diagnostics
            .push(Diagnostic::error(code, message, span));
    }

    pub(super) fn at_eof(&self) -> bool {
        self.index >= self.tokens.len()
    }

    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    fn current_kind(&self) -> Option<TokenKind> {
        self.current().map(|token| token.kind)
    }

    pub(super) fn bump_event(&mut self) -> Event {
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

    pub(super) fn span_start(&self) -> u32 {
        self.current()
            .map(|token| token.span.start)
            .unwrap_or_else(|| self.span_end())
    }

    fn span_end(&self) -> u32 {
        self.index
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
            .map(|token| token.span.end)
            .or_else(|| self.tokens.last().map(|token| token.span.end))
            .unwrap_or(0)
    }

    pub(super) fn next_span_or(&self, fallback: u32) -> Span {
        Span::new(
            fallback,
            self.current()
                .map(|token| token.span.end)
                .unwrap_or(fallback),
        )
    }
}

fn keyword_name(keyword: Keyword) -> &'static str {
    match keyword {
        Keyword::Call => "CALL",
        Keyword::Create => "CREATE",
        Keyword::Drop => "DROP",
        Keyword::Insert => "INSERT",
        Keyword::Delete => "DELETE",
        Keyword::Set => "SET",
        Keyword::Remove => "REMOVE",
        Keyword::Case => "CASE",
        Keyword::When => "WHEN",
        Keyword::Then => "THEN",
        Keyword::Else => "ELSE",
        Keyword::End => "END",
        _ => "reserved",
    }
}
