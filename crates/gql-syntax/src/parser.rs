//! Parser module for the minimal ISO-like GQL grammar used in this slice.

#![forbid(unsafe_code)]

use gql_source::{Diagnostic, SourceText, Span};

use crate::{
    rowan_build,
    syntax::{Keyword, SyntaxElement, SyntaxElementKind, SyntaxKind, SyntaxNode, SyntaxTree, Token, TokenKind},
};

/// Parses `source` into a CST-like tree and diagnostics.
pub fn parse(name: &str, source: &str) -> crate::Parse {
    let source = SourceText::new(name, source);
    let (tokens, mut diagnostics) = lex(&source);
    let mut parser = Parser::new(&tokens, &source);
    let (children, has_match, saw_return) = parser.parse_top_level();
    parser.collect_diagnostics(&mut diagnostics);
        if !has_match && saw_return {
            diagnostics.push(Diagnostic::error(
                "GQL-PARSE-MISSING-KEYWORD",
                "expected MATCH before RETURN in this revision",
                Span::new(0, source.text().len() as u32),
        ));
    }

    let query = SyntaxNode::new(
        SyntaxKind::Query,
        Span::new(0, source.text().len() as u32),
        children,
    );
    let root = SyntaxNode::new(
        SyntaxKind::SourceFile,
        Span::new(0, source.text().len() as u32),
        vec![SyntaxElement {
            kind: SyntaxElementKind::Node(query),
        }],
    );
    let rowan = rowan_build::build_rowan_root(&root, &source);
        crate::Parse {
        tree: SyntaxTree::new(source, root, tokens.clone(), rowan),
        diagnostics,
    }
}

fn lex(source: &SourceText) -> (Vec<Token>, Vec<Diagnostic>) {
    crate::lexer::lex(source.text())
}

struct Parser<'a> {
    tokens: &'a [Token],
    index: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token], _source: &'a SourceText) -> Self {
        Self {
            tokens,
            index: 0,
            diagnostics: Vec::new(),
        }
    }

    fn collect_diagnostics(self, diagnostics: &mut Vec<Diagnostic>) {
        diagnostics.extend(self.diagnostics);
    }

    fn parse_top_level(&mut self) -> (Vec<SyntaxElement>, bool, bool) {
        let mut children = Vec::new();
        let mut saw_match = false;
        let mut saw_return = false;
        while !self.at_eof() {
            let Some(token) = self.current() else {
                break;
            };
            match token.kind {
                TokenKind::Keyword(Keyword::Match) => {
                    saw_match = true;
                    children.push(SyntaxElement {
                        kind: SyntaxElementKind::Node(self.parse_match_clause()),
                    });
                }
                TokenKind::Keyword(Keyword::Return) => {
                    saw_return = true;
                    children.push(SyntaxElement {
                        kind: SyntaxElementKind::Node(self.parse_return_clause()),
                    });
                }
                TokenKind::Keyword(Keyword::Where) => {
                    children.push(SyntaxElement {
                        kind: SyntaxElementKind::Node(self.parse_where_clause()),
                    });
                }
                TokenKind::Keyword(Keyword::Let) => {
                    children.push(SyntaxElement {
                        kind: SyntaxElementKind::Node(self.parse_let_clause()),
                    });
                }
                _ => {
                    children.push(SyntaxElement {
                        kind: SyntaxElementKind::Token(self.bump_token()),
                    });
                }
            }
        }
        (children, saw_match, saw_return)
    }

    fn parse_match_clause(&mut self) -> SyntaxNode {
        let start = self.span_start();
        let mut children = Vec::new();
        children.push(SyntaxElement {
            kind: SyntaxElementKind::Token(self.bump_token()),
        });
        children.extend(self.collect_trivia());
        if self.matches_kind(TokenKind::Punctuation('(')) {
            children.push(SyntaxElement {
                kind: SyntaxElementKind::Node(self.parse_graph_pattern()),
            });
        } else {
            self.emit_missing_keyword(
                "GQL-PARSE-MATCH-SYNTAX",
                "MATCH clause must start with a graph pattern",
                self.next_span_or(start),
            );
        }

        SyntaxNode::new(SyntaxKind::MatchClause, Span::new(start, self.span_end()), children)
    }

    fn parse_return_clause(&mut self) -> SyntaxNode {
        let start = self.span_start();
        let mut children = Vec::new();
        let mut expressions = 0usize;

        children.push(SyntaxElement {
            kind: SyntaxElementKind::Token(self.bump_token()),
        });
        loop {
            self.skip_trivia_and_include(&mut children);
            let Some(token) = self.current() else {
                break;
            };

            let end_clause = matches!(
                token.kind,
                TokenKind::Keyword(Keyword::Match)
                    | TokenKind::Keyword(Keyword::Where)
                    | TokenKind::Keyword(Keyword::Let)
                    | TokenKind::Keyword(Keyword::Return)
            );
            if end_clause {
                break;
            }

                match token.kind {
                    TokenKind::Identifier | TokenKind::String | TokenKind::Number | TokenKind::Keyword(_) => {
                        children.push(SyntaxElement {
                            kind: SyntaxElementKind::Node(self.parse_expression()),
                        });
                        expressions += 1;
                        self.skip_trivia_and_include(&mut children);
                        if self.matches_kind(TokenKind::Punctuation(',')) {
                            children.push(SyntaxElement {
                                kind: SyntaxElementKind::Token(self.bump_token()),
                            });
                        }
                    }
                TokenKind::Punctuation(_) => {
                    children.push(SyntaxElement {
                        kind: SyntaxElementKind::Token(self.bump_token()),
                    });
                }
                _ => {
                    self.emit_return_syntax(
                        "GQL-PARSE-RETURN-SYNTAX",
                        "invalid token in RETURN clause",
                        token.span,
                    );
                    children.push(SyntaxElement {
                        kind: SyntaxElementKind::Token(self.bump_token()),
                    });
                    break;
                }
            }
        }

        if expressions == 0 {
            self.emit_return_syntax(
                "GQL-PARSE-RETURN-SYNTAX",
                "RETURN requires at least one expression",
                Span::new(start, self.span_end()),
            );
        }

        SyntaxNode::new(SyntaxKind::ReturnClause, Span::new(start, self.span_end()), children)
    }

    fn parse_where_clause(&mut self) -> SyntaxNode {
        let start = self.span_start();
        let mut children = Vec::new();
        children.push(SyntaxElement {
            kind: SyntaxElementKind::Token(self.bump_token()),
        });

        let mut expressions = 0usize;
        loop {
            self.skip_trivia_and_include(&mut children);
            let Some(token) = self.current() else {
                break;
            };

            let end_clause = matches!(
                token.kind,
                TokenKind::Keyword(Keyword::Match)
                    | TokenKind::Keyword(Keyword::Where)
                    | TokenKind::Keyword(Keyword::Let)
                    | TokenKind::Keyword(Keyword::Return)
            );
            if end_clause {
                break;
            }

            match token.kind {
                TokenKind::Identifier | TokenKind::String | TokenKind::Number | TokenKind::Keyword(_) => {
                    children.push(SyntaxElement {
                        kind: SyntaxElementKind::Node(self.parse_expression()),
                    });
                    expressions += 1;
                    self.skip_trivia_and_include(&mut children);
                    if self.matches_kind(TokenKind::Punctuation(',')) {
                        children.push(SyntaxElement {
                            kind: SyntaxElementKind::Token(self.bump_token()),
                        });
                    }
                }
                TokenKind::Punctuation(_) => {
                    children.push(SyntaxElement {
                        kind: SyntaxElementKind::Token(self.bump_token()),
                    });
                }
                _ => {
                    self.emit_missing_keyword(
                        "GQL-PARSE-WHERE-SYNTAX",
                        "invalid token in WHERE clause",
                        token.span,
                    );
                    children.push(SyntaxElement {
                        kind: SyntaxElementKind::Token(self.bump_token()),
                    });
                    break;
                }
            }
        }

        if expressions == 0 {
            self.emit_missing_keyword(
                "GQL-PARSE-WHERE-SYNTAX",
                "WHERE requires at least one expression",
                Span::new(start, self.span_end()),
            );
        }

        SyntaxNode::new(SyntaxKind::WhereClause, Span::new(start, self.span_end()), children)
    }

    fn parse_let_clause(&mut self) -> SyntaxNode {
        let start = self.span_start();
        let mut children = Vec::new();
        children.push(SyntaxElement {
            kind: SyntaxElementKind::Token(self.bump_token()),
        });

        self.skip_trivia_and_include(&mut children);
        if let Some(token) = self.current() {
            if !self.is_clause_boundary_token(&token.kind) {
                children.push(SyntaxElement {
                    kind: SyntaxElementKind::Node(self.parse_let_binding_expression()),
                });
            }
        }

        self.skip_trivia_and_include(&mut children);
        if self.matches_kind(TokenKind::Punctuation('=')) {
            children.push(SyntaxElement {
                kind: SyntaxElementKind::Token(self.bump_token()),
            });
        }

        self.skip_trivia_and_include(&mut children);
        if let Some(token) = self.current() {
            if !self.is_clause_boundary_token(&token.kind) {
                children.push(SyntaxElement {
                    kind: SyntaxElementKind::Node(self.parse_expression()),
                });
            }
        }

        SyntaxNode::new(SyntaxKind::LetClause, Span::new(start, self.span_end()), children)
    }

    fn parse_graph_pattern(&mut self) -> SyntaxNode {
        let start = self.span_start();
        let mut children = Vec::new();
        children.push(SyntaxElement {
            kind: SyntaxElementKind::Node(self.parse_node_pattern()),
        });
        if self.matches_kind(TokenKind::Punctuation('-')) || self.matches_kind(TokenKind::Punctuation('<'))
        {
            children.extend(self.parse_graph_edge_sequence());
        }
        SyntaxNode::new(
            SyntaxKind::GraphPattern,
            Span::new(start, self.span_end()),
            children,
        )
    }

    fn parse_graph_edge_sequence(&mut self) -> Vec<SyntaxElement> {
        let mut children = Vec::new();
        let mut edge_children = Vec::new();
        self.skip_trivia_and_include(&mut children);
        let edge_start = self.span_start();

        let first_token = self.bump_token();
        let first_token_kind = first_token.kind;
        edge_children.push(SyntaxElement {
            kind: SyntaxElementKind::Token(first_token),
        });

        if matches!(first_token_kind, TokenKind::Punctuation('<'))
            && self.matches_kind(TokenKind::Punctuation('-'))
        {
            edge_children.push(SyntaxElement {
                kind: SyntaxElementKind::Token(self.bump_token()),
            });
        }

        self.skip_trivia_and_include(&mut children);

        if !self.matches_kind(TokenKind::Punctuation('[')) {
            self.emit_match_syntax(
                "GQL-PARSE-MATCH-SYNTAX",
                "edge delimiter missing '['",
                self.next_span_or(edge_start),
            );
            return children;
        }

        edge_children.push(SyntaxElement {
            kind: SyntaxElementKind::Token(self.bump_token()),
        });
        self.skip_trivia_and_include(&mut children);

        let label_list = self.parse_label_list(self.span_start());
        if !label_list.children().is_empty() {
            edge_children.push(SyntaxElement {
                kind: SyntaxElementKind::Node(label_list),
            });
        } else if self.matches_kind(TokenKind::Punctuation(']')) {
            edge_children.push(SyntaxElement {
                kind: SyntaxElementKind::Node(self.make_empty_label_list(self.span_start())),
            });
        } else {
            let span_start = edge_start;
            self.emit_match_syntax(
                "GQL-PARSE-MATCH-SYNTAX",
                "edge label list malformed",
                Span::new(span_start, self.span_end()),
            );
        }

        self.skip_trivia_and_include(&mut children);
        if self.matches_kind(TokenKind::Punctuation(']')) {
            edge_children.push(SyntaxElement {
                kind: SyntaxElementKind::Token(self.bump_token()),
            });
        } else {
            self.emit_match_syntax(
                "GQL-PARSE-MATCH-SYNTAX",
                "edge label list missing closing bracket",
                self.next_span_or(self.span_end()),
            );
        }

        self.skip_trivia_and_include(&mut children);
        while self.matches_kind(TokenKind::Punctuation('-'))
            || self.matches_kind(TokenKind::Punctuation('>'))
            || self.matches_kind(TokenKind::Punctuation('<'))
        {
            edge_children.push(SyntaxElement {
                kind: SyntaxElementKind::Token(self.bump_token()),
            });
            self.skip_trivia_and_include(&mut children);
            if !self.matches_kind(TokenKind::Punctuation('-'))
                && !self.matches_kind(TokenKind::Punctuation('>'))
                && !self.matches_kind(TokenKind::Punctuation('<'))
            {
                break;
            }
        }

        children.push(SyntaxElement {
            kind: SyntaxElementKind::Node(self.make_edge_pattern(
                edge_start,
                self.span_end(),
                edge_children,
            )),
        });

        if self.matches_kind(TokenKind::Punctuation('(')) {
            children.push(SyntaxElement {
                kind: SyntaxElementKind::Node(self.parse_node_pattern()),
            });
        }
        children
    }

    fn parse_label_list(&mut self, start: u32) -> SyntaxNode {
        let mut children = Vec::new();
        loop {
            self.skip_trivia_and_include(&mut children);
            match self.peek() {
                Some(TokenKind::Punctuation(']')) => break,
                Some(
                    TokenKind::Identifier
                        | TokenKind::Keyword(_)
                        | TokenKind::Number
                        | TokenKind::Punctuation(':'),
                ) => {
                    children.push(SyntaxElement {
                        kind: SyntaxElementKind::Token(self.bump_token()),
                    });
                }
                Some(_) => break,
                None => break,
            }
        }
        SyntaxNode::new(SyntaxKind::LabelList, Span::new(start, self.span_end()), children)
    }

    fn parse_node_pattern(&mut self) -> SyntaxNode {
        let start = self.span_start();
        let mut children = Vec::new();
        if !self.matches_kind(TokenKind::Punctuation('(')) {
            self.emit_match_syntax(
                "GQL-PARSE-MATCH-SYNTAX",
                "node pattern missing opening '('",
                self.next_span_or(start),
            );
            return self.make_node_pattern_with_children(start, Vec::new());
        }
        children.push(SyntaxElement {
            kind: SyntaxElementKind::Token(self.bump_token()),
        });
        let mut closed = false;
        while !self.at_eof() {
            if self.matches_kind(TokenKind::Punctuation(')')) {
                closed = true;
                children.push(SyntaxElement {
                    kind: SyntaxElementKind::Token(self.bump_token()),
                });
                break;
            }
            children.push(SyntaxElement {
                kind: SyntaxElementKind::Token(self.bump_token()),
            });
        }
        if !closed {
            self.emit_match_syntax(
                "GQL-PARSE-MATCH-SYNTAX",
                "node pattern missing closing ')'",
                Span::new(start, self.span_end()),
            );
        }
        self.make_node_pattern_with_children(start, children)
    }

    fn parse_expression(&mut self) -> SyntaxNode {
        let start = self.span_start();
        self.parse_or_expression(start)
    }

    fn parse_or_expression(&mut self, start: u32) -> SyntaxNode {
        let mut lhs = self.parse_and_expression();
        while self.matches_keyword(Keyword::Or) {
            let mut children = vec![SyntaxElement {
                kind: SyntaxElementKind::Node(lhs),
            }];
            children.push(SyntaxElement {
                kind: SyntaxElementKind::Token(self.bump_token()),
            });
            self.skip_trivia_and_include(&mut children);
            let rhs = self.parse_and_expression();
            children.push(SyntaxElement {
                kind: SyntaxElementKind::Node(rhs),
            });
            lhs = SyntaxNode::new(
                SyntaxKind::BinaryExpression,
                Span::new(start, self.span_end()),
                children,
            );
        }
        lhs
    }

    fn parse_and_expression(&mut self) -> SyntaxNode {
        let start = self.span_start();
        let mut lhs = self.parse_not_expression();
        while self.matches_keyword(Keyword::And) {
            let mut children = vec![SyntaxElement {
                kind: SyntaxElementKind::Node(lhs),
            }];
            children.push(SyntaxElement {
                kind: SyntaxElementKind::Token(self.bump_token()),
            });
            self.skip_trivia_and_include(&mut children);
            let rhs = self.parse_not_expression();
            children.push(SyntaxElement {
                kind: SyntaxElementKind::Node(rhs),
            });
            lhs = SyntaxNode::new(
                SyntaxKind::BinaryExpression,
                Span::new(start, self.span_end()),
                children,
            );
        }
        lhs
    }

    fn parse_not_expression(&mut self) -> SyntaxNode {
        if self.matches_keyword(Keyword::Not) {
            let start = self.span_start();
            let mut children = Vec::new();
            children.push(SyntaxElement {
                kind: SyntaxElementKind::Token(self.bump_token()),
            });
            self.skip_trivia_and_include(&mut children);
            let operand = self.parse_not_expression();
            children.push(SyntaxElement {
                kind: SyntaxElementKind::Node(operand),
            });
            return SyntaxNode::new(
                SyntaxKind::UnaryExpression,
                Span::new(start, self.span_end()),
                children,
            );
        }

        self.parse_comparison_expression()
    }

    fn parse_comparison_expression(&mut self) -> SyntaxNode {
        let start = self.span_start();
        let mut lhs = self.parse_primary_expression();
        while !self.at_eof() {
            if self.is_expression_boundary() {
                break;
            }

            let operator_start = self.index;
            let mut operator_tokens = Vec::new();
            self.skip_trivia_and_include(&mut operator_tokens);

            if self.at_eof() || self.is_expression_boundary() {
                self.index = operator_start;
                break;
            }

            let parsed_operator = self.consume_comparison_operator(&mut operator_tokens);
            if !parsed_operator {
                self.index = operator_start;
                break;
            }

            self.skip_trivia_and_include(&mut operator_tokens);
            let rhs = self.parse_primary_expression();
            let mut children = Vec::with_capacity(2 + operator_tokens.len());
            children.push(SyntaxElement {
                kind: SyntaxElementKind::Node(lhs),
            });
            children.extend(operator_tokens);
            children.push(SyntaxElement {
                kind: SyntaxElementKind::Node(rhs),
            });
            lhs = SyntaxNode::new(
                SyntaxKind::BinaryExpression,
                Span::new(start, self.span_end()),
                children,
            );
            if self.is_expression_boundary() || self.current_kind() == Some(TokenKind::Punctuation(',')) {
                break;
            }
        }
        lhs
    }

    fn parse_let_binding_expression(&mut self) -> SyntaxNode {
        self.parse_primary_expression()
    }

    fn parse_primary_expression(&mut self) -> SyntaxNode {
        let start = self.span_start();
        match self.current_kind() {
            None => {
                SyntaxNode::new(
                    SyntaxKind::Expression,
                    Span::new(start, self.span_end()),
                    Vec::new(),
                )
            }
            Some(TokenKind::Punctuation('(')) => {
                let mut children = Vec::new();
                children.push(SyntaxElement {
                    kind: SyntaxElementKind::Token(self.bump_token()),
                });
                self.skip_trivia_and_include(&mut children);
                if !self.at_eof() && !self.matches_kind(TokenKind::Punctuation(')')) {
                    children.push(SyntaxElement {
                        kind: SyntaxElementKind::Node(self.parse_or_expression(self.span_start())),
                    });
                    self.skip_trivia_and_include(&mut children);
                }
                if self.matches_kind(TokenKind::Punctuation(')')) {
                    children.push(SyntaxElement {
                        kind: SyntaxElementKind::Token(self.bump_token()),
                    });
                }
                SyntaxNode::new(
                    SyntaxKind::ParenthesizedExpression,
                    Span::new(start, self.span_end()),
                    children,
                )
            }
            Some(TokenKind::Identifier | TokenKind::String | TokenKind::Number | TokenKind::Keyword(_)) => {
                let token = self.bump_token();
                let kind = match token.kind {
                    TokenKind::Number | TokenKind::String => SyntaxKind::LiteralExpression,
                    _ => SyntaxKind::NameExpression,
                };
                SyntaxNode::new(kind, token.span, vec![SyntaxElement {
                    kind: SyntaxElementKind::Token(token),
                }])
            }
            Some(_) => SyntaxNode::new(
                SyntaxKind::NameExpression,
                {
                    let token = self.bump_token();
                    token.span
                },
                vec![SyntaxElement {
                    kind: SyntaxElementKind::Token(self.tokens[self.index - 1].clone()),
                }],
            ),
        }
    }

    fn consume_comparison_operator(&mut self, operator_tokens: &mut Vec<SyntaxElement>) -> bool {
        if self.matches_kind(TokenKind::Punctuation('=')) {
            operator_tokens.push(SyntaxElement {
                kind: SyntaxElementKind::Token(self.bump_token()),
            });
            return true;
        }
        if self.matches_kind(TokenKind::Punctuation('!'))
            && self.peek_kind(1) == Some(TokenKind::Punctuation('='))
        {
            operator_tokens.push(SyntaxElement {
                kind: SyntaxElementKind::Token(self.bump_token()),
            });
            operator_tokens.push(SyntaxElement {
                kind: SyntaxElementKind::Token(self.bump_token()),
            });
            return true;
        }
        if self.matches_kind(TokenKind::Punctuation('<')) {
            operator_tokens.push(SyntaxElement {
                kind: SyntaxElementKind::Token(self.bump_token()),
            });
            if self.matches_kind(TokenKind::Punctuation('=')) {
                operator_tokens.push(SyntaxElement {
                    kind: SyntaxElementKind::Token(self.bump_token()),
                });
            }
            return true;
        }
        if self.matches_kind(TokenKind::Punctuation('>')) {
            operator_tokens.push(SyntaxElement {
                kind: SyntaxElementKind::Token(self.bump_token()),
            });
            if self.matches_kind(TokenKind::Punctuation('=')) {
                operator_tokens.push(SyntaxElement {
                    kind: SyntaxElementKind::Token(self.bump_token()),
                });
            }
            return true;
        }
        false
    }

    fn is_expression_boundary(&self) -> bool {
        matches!(
            self.current_kind(),
            None
                | Some(TokenKind::Keyword(Keyword::Match))
                | Some(TokenKind::Keyword(Keyword::Where))
                | Some(TokenKind::Keyword(Keyword::Let))
                | Some(TokenKind::Keyword(Keyword::Return))
                | Some(TokenKind::Punctuation(','))
                | Some(TokenKind::Punctuation(')'))
        )
    }

    fn is_clause_boundary_token(&self, kind: &TokenKind) -> bool {
        self.is_clause_boundary(*kind)
    }

    fn is_clause_boundary(&self, kind: TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Keyword(Keyword::Match)
                | TokenKind::Keyword(Keyword::Where)
                | TokenKind::Keyword(Keyword::Let)
                | TokenKind::Keyword(Keyword::Return)
                | TokenKind::Punctuation(',')
        )
    }

    fn make_node_pattern_with_children(
        &mut self,
        start: u32,
        children: Vec<SyntaxElement>,
    ) -> SyntaxNode {
        SyntaxNode::new(
            SyntaxKind::NodePattern,
            Span::new(start, self.span_end()),
            children,
        )
    }

    fn make_edge_pattern(
        &mut self,
        start: u32,
        end: u32,
        children: Vec<SyntaxElement>,
    ) -> SyntaxNode {
        SyntaxNode::new(SyntaxKind::EdgePattern, Span::new(start, end), children)
    }

    fn make_empty_label_list(&mut self, start: u32) -> SyntaxNode {
        SyntaxNode::new(SyntaxKind::LabelList, Span::new(start, self.span_end()), Vec::new())
    }

    fn skip_trivia_and_include(&mut self, children: &mut Vec<SyntaxElement>) {
        while let Some(token) = self.current() {
            if !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment) {
                break;
            }
            children.push(SyntaxElement {
                kind: SyntaxElementKind::Token(self.bump_token()),
            });
        }
    }

    fn collect_trivia(&mut self) -> Vec<SyntaxElement> {
        let mut children = Vec::new();
        self.skip_trivia_and_include(&mut children);
        children
    }

    fn matches_kind(&self, kind: TokenKind) -> bool {
        self.current_kind() == Some(kind)
    }

    fn matches_keyword(&self, keyword: Keyword) -> bool {
        self.current_kind() == Some(TokenKind::Keyword(keyword))
    }

    fn peek_kind(&self, offset: usize) -> Option<TokenKind> {
        self.tokens.get(self.index + offset).map(|token| token.kind)
    }

    fn emit_missing_keyword(&mut self, code: &'static str, message: impl Into<String>, span: Span) {
        self.diagnostics
            .push(Diagnostic::error(code, message, span));
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

    fn peek(&self) -> Option<TokenKind> {
        self.current_kind()
    }

    fn bump_token(&mut self) -> Token {
        let token = self
            .tokens
            .get(self.index)
            .cloned()
            .unwrap_or_else(|| Token {
                kind: TokenKind::Unknown,
                span: Span::new(self.span_end(), self.span_end()),
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
        if self.index == 0 {
            0
        } else {
            self.tokens
                .get(self.index.saturating_sub(1))
                .map(|token| token.span.end)
                .unwrap_or_else(|| self.tokens.last().map(|token| token.span.end).unwrap_or(0))
        }
    }

    fn next_span_or(&self, fallback: u32) -> Span {
        Span::new(
            fallback,
            self.current().map(|token| token.span.end).unwrap_or(fallback),
        )
    }
}
