//! Lossless event parser for the supported ISO GQL syntax slice.

#![forbid(unsafe_code)]

use gql_source::{Diagnostic, SourceText, Span};

use crate::syntax::{Keyword, SyntaxKind, SyntaxTree, Token, TokenKind};

/// Parser output consumed directly by the Rowan tree sink.
#[derive(Clone, Debug)]
pub(crate) enum Event {
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

    let query = node(SyntaxKind::Query, children);
    let events = node(SyntaxKind::SourceFile, query);
    let rowan = build_rowan_root(&events);

    crate::Parse {
        tree: SyntaxTree::new(source, tokens, rowan),
        diagnostics,
    }
}

fn node(kind: SyntaxKind, mut children: Vec<Event>) -> Vec<Event> {
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

struct Parser<'a> {
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
            match self.current_kind() {
                Some(TokenKind::Keyword(Keyword::Match)) => {
                    saw_match = true;
                    children.extend(self.parse_match_clause());
                }
                Some(TokenKind::Keyword(Keyword::Return)) => {
                    saw_return = true;
                    children.extend(self.parse_return_clause());
                }
                Some(TokenKind::Keyword(Keyword::Where)) => {
                    children.extend(self.parse_where_clause());
                }
                Some(TokenKind::Keyword(Keyword::Let)) => {
                    children.extend(self.parse_let_clause());
                }
                Some(_) => children.push(self.bump_event()),
                None => break,
            }
        }

        (children, saw_match, saw_return)
    }

    fn parse_match_clause(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());

        if self.matches_kind(TokenKind::Punctuation('(')) {
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

        let mut children = node(SyntaxKind::EdgePattern, edge_children);
        children.extend(self.skip_trivia());
        if self.matches_kind(TokenKind::Punctuation('(')) {
            children.extend(self.parse_node_pattern());
        }
        children
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

    fn parse_expression(&mut self) -> Vec<Event> {
        self.parse_or_expression()
    }

    fn parse_or_expression(&mut self) -> Vec<Event> {
        let mut lhs = self.parse_and_expression();
        loop {
            let operator_start = self.index;
            let trivia = self.skip_trivia();
            if !self.matches_keyword(Keyword::Or) {
                self.index = operator_start;
                break;
            }
            let mut children = lhs;
            children.extend(trivia);
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
            children.extend(self.parse_and_expression());
            lhs = node(SyntaxKind::BinaryExpression, children);
        }
        lhs
    }

    fn parse_and_expression(&mut self) -> Vec<Event> {
        let mut lhs = self.parse_not_expression();
        loop {
            let operator_start = self.index;
            let trivia = self.skip_trivia();
            if !self.matches_keyword(Keyword::And) {
                self.index = operator_start;
                break;
            }
            let mut children = lhs;
            children.extend(trivia);
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
            children.extend(self.parse_not_expression());
            lhs = node(SyntaxKind::BinaryExpression, children);
        }
        lhs
    }

    fn parse_not_expression(&mut self) -> Vec<Event> {
        if self.matches_keyword(Keyword::Not) {
            let mut children = vec![self.bump_event()];
            children.extend(self.skip_trivia());
            children.extend(self.parse_not_expression());
            return node(SyntaxKind::UnaryExpression, children);
        }
        self.parse_comparison_expression()
    }

    fn parse_comparison_expression(&mut self) -> Vec<Event> {
        let mut lhs = self.parse_primary_expression();
        loop {
            let operator_start = self.index;
            let mut operator = self.skip_trivia();
            if self.at_eof() || self.is_expression_boundary() {
                self.index = operator_start;
                break;
            }
            if !self.consume_comparison_operator(&mut operator) {
                self.index = operator_start;
                break;
            }
            operator.extend(self.skip_trivia());
            let mut children = lhs;
            children.extend(operator);
            children.extend(self.parse_primary_expression());
            lhs = node(SyntaxKind::BinaryExpression, children);
            if self.is_expression_boundary() {
                break;
            }
        }
        lhs
    }

    fn parse_let_binding_expression(&mut self) -> Vec<Event> {
        self.parse_primary_expression()
    }

    fn parse_primary_expression(&mut self) -> Vec<Event> {
        match self.current_kind() {
            None => node(SyntaxKind::Expression, Vec::new()),
            Some(TokenKind::Punctuation('(')) => {
                let mut children = vec![self.bump_event()];
                children.extend(self.skip_trivia());
                if !self.at_eof() && !self.matches_kind(TokenKind::Punctuation(')')) {
                    children.extend(self.parse_or_expression());
                    children.extend(self.skip_trivia());
                }
                if self.matches_kind(TokenKind::Punctuation(')')) {
                    children.push(self.bump_event());
                }
                node(SyntaxKind::ParenthesizedExpression, children)
            }
            Some(
                TokenKind::Identifier
                | TokenKind::String
                | TokenKind::Number
                | TokenKind::Keyword(_),
            ) => {
                let kind = match self.current_kind() {
                    Some(TokenKind::Number | TokenKind::String) => SyntaxKind::LiteralExpression,
                    _ => SyntaxKind::NameExpression,
                };
                node(kind, vec![self.bump_event()])
            }
            Some(_) => node(SyntaxKind::NameExpression, vec![self.bump_event()]),
        }
    }

    fn consume_comparison_operator(&mut self, events: &mut Vec<Event>) -> bool {
        if self.matches_kind(TokenKind::Punctuation('=')) {
            events.push(self.bump_event());
            return true;
        }
        if self.matches_kind(TokenKind::Punctuation('!'))
            && self.peek_kind(1) == Some(TokenKind::Punctuation('='))
        {
            events.push(self.bump_event());
            events.push(self.bump_event());
            return true;
        }
        if matches!(self.current_kind(), Some(TokenKind::Punctuation('<' | '>'))) {
            events.push(self.bump_event());
            if self.matches_kind(TokenKind::Punctuation('=')) {
                events.push(self.bump_event());
            }
            return true;
        }
        false
    }

    fn skip_trivia(&mut self) -> Vec<Event> {
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
        )
    }

    fn is_clause_keyword(&self, kind: TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Keyword(Keyword::Match)
                | TokenKind::Keyword(Keyword::Where)
                | TokenKind::Keyword(Keyword::Let)
                | TokenKind::Keyword(Keyword::Return)
        )
    }

    fn is_clause_boundary(&self, kind: TokenKind) -> bool {
        self.is_clause_keyword(kind) || kind == TokenKind::Punctuation(',')
    }

    fn is_expression_boundary(&self) -> bool {
        self.current_kind()
            .is_none_or(|kind| self.is_clause_boundary(kind) || kind == TokenKind::Punctuation(')'))
    }

    fn matches_kind(&self, kind: TokenKind) -> bool {
        self.current_kind() == Some(kind)
    }

    fn matches_keyword(&self, keyword: Keyword) -> bool {
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

    fn emit_match_syntax(&mut self, code: &'static str, message: impl Into<String>, span: Span) {
        self.diagnostics
            .push(Diagnostic::error(code, message, span));
    }

    fn emit_return_syntax(&mut self, code: &'static str, message: impl Into<String>, span: Span) {
        self.diagnostics
            .push(Diagnostic::error(code, message, span));
    }

    fn at_eof(&self) -> bool {
        self.index >= self.tokens.len()
    }

    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    fn current_kind(&self) -> Option<TokenKind> {
        self.current().map(|token| token.kind)
    }

    fn bump_event(&mut self) -> Event {
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

    fn span_start(&self) -> u32 {
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

    fn next_span_or(&self, fallback: u32) -> Span {
        Span::new(
            fallback,
            self.current()
                .map(|token| token.span.end)
                .unwrap_or(fallback),
        )
    }
}
