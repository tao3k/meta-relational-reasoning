//! Lowering helpers for converting parse trees into public GQL AST structures.
#![forbid(unsafe_code)]

use super::{
    BinaryOperator,
    UnaryOperator,
    EdgeDirection,
    EdgePattern,
    Expression,
    Identifier,
    MatchClause,
    NodePattern,
    PatternElement,
    Query,
    QueryClause,
    GraphPattern,
    Statement,
    SyntaxParseOutput,
};
use gql_source::{Diagnostic, Span};
use gql_syntax::{
    Keyword, Parse as SyntaxParse, SyntaxElement, SyntaxElementKind, SyntaxKind, SyntaxNode, Token,
    TokenKind,
};
/// Lower a parsed syntax tree into the AST result used by semantic analysis.
#[must_use]
pub fn lower_from_syntax(parse: &SyntaxParse) -> SyntaxParseOutput {
    let mut diagnostics = parse.diagnostics.clone();
    let statement = lower_statement_from_syntax_root(
        parse.tree.root(),
        parse.tree.source().text(),
        &mut diagnostics,
    );
    SyntaxParseOutput {
        statement,
        diagnostics,
    }
}

fn lower_statement_from_syntax_root(
    root: &SyntaxNode,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Statement {
    let mut clauses = Vec::new();
    let mut saw_query_node = false;
    for element in root.children() {
        let Some(node) = syntax_node(element) else {
            continue;
        };
        if node.kind() == SyntaxKind::Query {
            saw_query_node = true;
            clauses.extend(lower_query(node, source, diagnostics).clauses);
        }
    }

    if !saw_query_node && clauses.is_empty() {
        diagnostics.push(Diagnostic::error(
            "GQL-PARSE-MISSING-MATCH",
            "expected MATCH clause",
            Span::new(0, source.len() as u32),
        ));
    }

    Statement::Query(Query {
        clauses,
        span: root.span(),
    })
}

fn lower_query(node: &SyntaxNode, source: &str, diagnostics: &mut Vec<Diagnostic>) -> Query {
    let mut clauses = Vec::new();
    for element in node.children() {
        let Some(child) = syntax_node(element) else {
            continue;
        };

        match child.kind() {
            SyntaxKind::MatchClause => {
                clauses.push(QueryClause::Match(lower_match_clause(child, source)))
            }
            SyntaxKind::WhereClause => {
                clauses.push(QueryClause::Where {
                    expression: lower_where_clause(child, source, diagnostics),
                });
            }
            SyntaxKind::LetClause => {
                clauses.push(lower_let_clause(child, source, diagnostics));
            }
            SyntaxKind::ReturnClause => clauses.push(QueryClause::Return {
                expressions: lower_return_clause(child, source),
            }),
            _ => {}
        }
    }

    if clauses.is_empty() {
        diagnostics.push(Diagnostic::error(
            "GQL-PARSE-MISSING-MATCH",
            "expected MATCH clause",
            node.span(),
        ));
    }

    Query {
        clauses,
        span: node.span(),
    }
}

fn lower_match_clause(node: &SyntaxNode, source: &str) -> MatchClause {
    let mut pattern = GraphPattern {
        elements: Vec::new(),
        span: node.span(),
    };

    for element in node.children() {
        let Some(child) = syntax_node(element) else {
            continue;
        };
        if child.kind() == SyntaxKind::GraphPattern {
            pattern = lower_graph_pattern(child, source);
            break;
        }
    }

    MatchClause {
        pattern,
        span: node.span(),
    }
}

fn lower_graph_pattern(node: &SyntaxNode, source: &str) -> GraphPattern {
    let elements_in = node.children();
    let mut pending_edge_labels: Option<Vec<Identifier>> = None;
    let mut elements = Vec::new();

    for element in elements_in.iter() {
        match &element.kind {
            SyntaxElementKind::Node(child_node) => match child_node.kind() {
                SyntaxKind::NodePattern => {
                    elements.push(PatternElement::Node(lower_node_pattern(child_node, source)))
                }
                SyntaxKind::LabelList => {
                    pending_edge_labels = Some(lower_label_list(child_node, source));
                }
                SyntaxKind::EdgePattern => {
                    let labels = pending_edge_labels.take().unwrap_or_default();
                    let direction = edge_direction_from_span(child_node.span(), source);
                    elements.push(PatternElement::Edge(EdgePattern {
                        labels,
                        direction,
                        span: child_node.span(),
                    }));
                }
                _ => {}
            },
            SyntaxElementKind::Token(_) => {}
        }
    }

    GraphPattern {
        elements,
        span: node.span(),
    }
}

fn edge_direction_from_span(edge_span: Span, source: &str) -> EdgeDirection {
    let raw = source
        .get(edge_span.start as usize..edge_span.end as usize)
        .unwrap_or("");
    let edge_text = raw.trim();

    if edge_text.starts_with("<-") {
        return EdgeDirection::In;
    }
    if edge_text.ends_with("->") {
        return EdgeDirection::Out;
    }
    EdgeDirection::Undirected
}

fn lower_return_clause(node: &SyntaxNode, source: &str) -> Vec<Expression> {
    let mut expressions = Vec::new();
    for element in node.children() {
        let Some(child) = syntax_node(element) else {
            continue;
        };
        if child.kind() == SyntaxKind::Expression {
            if let Some(expression) = lower_expression(child, source) {
                expressions.push(expression);
            }
        }
    }
    expressions
}

fn lower_where_clause(node: &SyntaxNode, source: &str, diagnostics: &mut Vec<Diagnostic>) -> Expression {
    let expressions: Vec<_> = node
        .children()
        .iter()
        .filter_map(|element| match element {
            SyntaxElement {
                kind: SyntaxElementKind::Node(child_node),
            } if child_node.kind() == SyntaxKind::Expression => lower_expression(child_node, source),
            _ => None,
        })
        .collect();

    if expressions.len() == 1 {
        return expressions[0].clone();
    }

    if expressions.len() > 1 {
        diagnostics.push(Diagnostic::error(
            "GQL-AST-WHERE-UNSUPPORTED-EXPRESSION",
            "WHERE clause expression shape is not supported by this release",
            node.span(),
        ));
    } else {
        diagnostics.push(Diagnostic::error(
            "GQL-AST-WHERE-MISSING-EXPRESSION",
            "WHERE clause is missing an expression",
            node.span(),
        ));
    }

    Expression::Name(Identifier {
        text: String::new(),
        span: node.span(),
    })
}

fn lower_expression(node: &SyntaxNode, source: &str) -> Option<Expression> {
    let elements: Vec<&SyntaxElement> = node
        .children()
        .iter()
        .filter(|element| match element {
            SyntaxElement {
                kind: SyntaxElementKind::Token(token),
            } => !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment),
            SyntaxElement {
                kind: SyntaxElementKind::Node(_),
            } => true,
        })
        .collect();
    parse_expression_elements(&elements, source)
}

fn parse_expression_elements(
    elements: &[&SyntaxElement],
    source: &str,
) -> Option<Expression> {
    let first = elements.first()?;

    if elements.len() == 1 {
        return parse_expression_atom(first, source);
    }

    if matches_parenthesized_expression(elements) {
        return parse_expression_atom(elements[1], source);
    }

    if let Some(expression) = parse_expression_unary(elements, source) {
        return Some(expression);
    }

    let mut index = 0usize;
    let mut lhs = parse_expression_atom(elements[index], source)?;
    index += 1;

    while index < elements.len() {
        let (operator, consumed) = parse_expression_binary_operator(elements, index)?;
        index += consumed;
        let rhs_element = elements.get(index)?;
        let rhs = parse_expression_atom(rhs_element, source)?;
        index += 1;
        lhs = Expression::Binary {
            operator,
            left: Box::new(lhs),
            right: Box::new(rhs),
        };
    }

    Some(lhs)
}

fn parse_expression_atom(element: &SyntaxElement, source: &str) -> Option<Expression> {
    match element {
        SyntaxElement {
            kind: SyntaxElementKind::Node(child),
        } if child.kind() == SyntaxKind::Expression => lower_expression(child, source),
        SyntaxElement {
            kind: SyntaxElementKind::Token(token),
        } => lower_expression_token(token, source),
        _ => None,
    }
}

fn parse_expression_unary(elements: &[&SyntaxElement], source: &str) -> Option<Expression> {
    let token_element = elements.first()?;
    let SyntaxElement {
        kind: SyntaxElementKind::Token(token),
    } = token_element else {
        return None;
    };
    if token.kind != TokenKind::Keyword(Keyword::Not) {
        return None;
    }
    let operand_element = elements.get(1)?;
    let operand = parse_expression_atom(operand_element, source)?;
    Some(Expression::Unary {
        operator: UnaryOperator::Not,
        operand: Box::new(operand),
    })
}

fn parse_expression_binary_operator(
    elements: &[&SyntaxElement],
    index: usize,
) -> Option<(BinaryOperator, usize)> {
    let SyntaxElement {
        kind: SyntaxElementKind::Token(token),
    } = elements.get(index)? else {
        return None;
    };
    match token.kind {
        TokenKind::Keyword(Keyword::Or) => Some((BinaryOperator::Or, 1)),
        TokenKind::Keyword(Keyword::And) => Some((BinaryOperator::And, 1)),
        TokenKind::Punctuation('=') => Some((BinaryOperator::Equals, 1)),
        TokenKind::Punctuation('!') => {
            let next_is_equals = matches!(
                elements.get(index + 1),
                Some(SyntaxElement {
                    kind: SyntaxElementKind::Token(Token {
                        kind: TokenKind::Punctuation('='),
                        ..
                    }),
                })
            );
            if next_is_equals {
                Some((BinaryOperator::NotEquals, 2))
            } else {
                None
            }
        }
        TokenKind::Punctuation('<') => {
            let next_is_equals = matches!(
                elements.get(index + 1),
                Some(SyntaxElement {
                    kind: SyntaxElementKind::Token(Token {
                        kind: TokenKind::Punctuation('='),
                        ..
                    }),
                })
            );
            if next_is_equals {
                Some((BinaryOperator::LessThanOrEqual, 2))
            } else {
                Some((BinaryOperator::LessThan, 1))
            }
        }
        TokenKind::Punctuation('>') => {
            let next_is_equals = matches!(
                elements.get(index + 1),
                Some(SyntaxElement {
                    kind: SyntaxElementKind::Token(Token {
                        kind: TokenKind::Punctuation('='),
                        ..
                    }),
                })
            );
            if next_is_equals {
                Some((BinaryOperator::GreaterThanOrEqual, 2))
            } else {
                Some((BinaryOperator::GreaterThan, 1))
            }
        }
        _ => None,
    }
}

fn matches_parenthesized_expression(elements: &[&SyntaxElement]) -> bool {
    if elements.len() < 2 {
        return false;
    }
    let Some(open) = elements.first() else {
        return false;
    };
    let Some(close) = elements.last() else {
        return false;
    };

    matches!(
        (open, close),
        (
            SyntaxElement {
                kind: SyntaxElementKind::Token(open_token),
            },
            SyntaxElement {
                kind: SyntaxElementKind::Token(close_token),
            },
        ) if open_token.kind == TokenKind::Punctuation('(')
            && close_token.kind == TokenKind::Punctuation(')')
    ) && elements.get(1).is_some_and(|middle| match middle {
        SyntaxElement {
            kind: SyntaxElementKind::Node(node),
        } => node.kind() == SyntaxKind::Expression,
        _ => false,
    })
}

fn lower_expression_token(token: &Token, source: &str) -> Option<Expression> {
    match token.kind {
        TokenKind::Identifier => Some(Expression::Name(identifier_from_token(token, source))),
        TokenKind::Keyword(_) => Some(Expression::Name(Identifier {
            text: token_text(token, source),
            span: token.span,
        })),
        TokenKind::String => {
            let mut value = token_text(token, source);
            if value.len() >= 2
                && ((value.starts_with('\'') && value.ends_with('\''))
                    || (value.starts_with('\"') && value.ends_with('\"')))
            {
                value = value[1..value.len() - 1].to_string();
            }
            Some(Expression::String(value, token.span))
        }
        TokenKind::Number => token_text(token, source)
            .parse::<i64>()
            .ok()
            .map(|value| Expression::Integer(value, token.span)),
        _ => None,
    }
}

fn lower_let_clause(
    node: &SyntaxNode,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> QueryClause {
    let mut binding: Option<Identifier> = None;
    let mut value: Option<Expression> = None;
    let mut seen_equals = false;

    for element in node.children() {
        match element {
            SyntaxElement {
                kind: SyntaxElementKind::Node(child),
            } if child.kind() == SyntaxKind::Expression => {
                if binding.is_none() {
                    if let Some(expression) = lower_expression(child, source) {
                        match expression {
                            Expression::Name(name) => binding = Some(name),
                            _ => {
                                if seen_equals && value.is_none() {
                                    value = Some(expression);
                                }
                                diagnostics.push(Diagnostic::error(
                                    "GQL-AST-LET-BINDING-EXPECTED",
                                    "LET clause expects an identifier binding",
                                    child.span(),
                                ));
                            }
                        }
                    }
                } else {
                    if value.is_none() {
                        value = lower_expression(child, source).or_else(|| {
                            diagnostics.push(Diagnostic::error(
                                "GQL-AST-LET-VALUE-INVALID",
                                "LET clause value could not be lowered",
                                child.span(),
                            ));
                            Some(Expression::Name(Identifier {
                                text: String::new(),
                                span: node.span(),
                            }))
                        })
                    }
                }
            }
            SyntaxElement {
                kind: SyntaxElementKind::Token(token),
            } if matches!(token.kind, TokenKind::Punctuation('=')) => {
                seen_equals = true;
            }
            _ => {}
        }
    }

    let binding = match binding {
        Some(binding) => binding,
        None => {
            diagnostics.push(Diagnostic::error(
                "GQL-AST-LET-BINDING-MISSING",
                "LET clause is missing a binding",
                node.span(),
            ));
            Identifier {
                text: String::new(),
                span: node.span(),
            }
        }
    };

    let value = match value {
        Some(value) => value,
        None => {
            diagnostics.push(Diagnostic::error(
                "GQL-AST-LET-VALUE-MISSING",
                "LET clause is missing a value",
                node.span(),
            ));
            Expression::Name(Identifier {
                text: String::new(),
                span: node.span(),
            })
        }
    };

    QueryClause::Let { binding, value }
}

fn lower_node_pattern(node: &SyntaxNode, source: &str) -> NodePattern {
    let mut binding = None;
    let mut labels = Vec::new();
    let mut consumed_binding = false;
    let mut in_labels = false;

    for token in syntax_tokens(node.children()) {
        match token.kind {
            TokenKind::Whitespace => {}
            TokenKind::Punctuation(':') => in_labels = true,
            TokenKind::Identifier | TokenKind::Keyword(_) => {
                let identifier = identifier_from_token(token, source);
                if !consumed_binding && !in_labels {
                    binding = Some(identifier);
                    consumed_binding = true;
                } else if in_labels || consumed_binding {
                    labels.push(identifier);
                }
            }
            _ => {}
        }
    }

    NodePattern {
        binding,
        labels,
        span: node.span(),
    }
}

fn lower_label_list(node: &SyntaxNode, source: &str) -> Vec<Identifier> {
    syntax_tokens(node.children())
        .filter_map(|token| match token.kind {
            TokenKind::Identifier | TokenKind::Keyword(_) => {
                Some(identifier_from_token(token, source))
            }
            _ => None,
        })
        .collect()
}

fn syntax_node(element: &SyntaxElement) -> Option<&SyntaxNode> {
    if let SyntaxElementKind::Node(node) = &element.kind {
        Some(node)
    } else {
        None
    }
}

fn syntax_token(element: &SyntaxElement) -> Option<&Token> {
    if let SyntaxElementKind::Token(token) = &element.kind {
        Some(token)
    } else {
        None
    }
}

fn syntax_tokens(elements: &[SyntaxElement]) -> impl Iterator<Item = &Token> {
    elements.iter().filter_map(syntax_token)
}

fn token_text(token: &Token, source: &str) -> String {
    source
        .get(token.span.start as usize..token.span.end as usize)
        .unwrap_or("")
        .to_string()
}

fn identifier_from_token(token: &Token, source: &str) -> Identifier {
    Identifier {
        text: token_text(token, source),
        span: token.span,
    }
}
