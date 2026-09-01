//! Lowering helpers for converting parse trees into public GQL AST structures.
#![forbid(unsafe_code)]

use super::{
    BinaryOperator, EdgeDirection, EdgePattern, Expression, GraphPattern, Identifier, MatchClause,
    NodePattern, PathPattern, PathQuantifier, PatternElement, Query, QueryClause, ReturnProjection,
    SortDirection, SortKey, Statement, SyntaxParseOutput, UnaryOperator,
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
    let root = parse.tree.root();
    let statement =
        lower_statement_from_syntax_root(&root, parse.tree.source().text(), &mut diagnostics);
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
        let Some(node) = syntax_node(&element) else {
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
        let Some(child) = syntax_node(&element) else {
            continue;
        };

        match child.kind() {
            SyntaxKind::MatchClause => {
                clauses.push(QueryClause::Match(lower_match_clause(child, source)))
            }
            SyntaxKind::OptionalMatchClause => clauses.push(QueryClause::OptionalMatch(
                lower_optional_match_clause(child, source),
            )),
            SyntaxKind::WhereClause => {
                clauses.push(QueryClause::Where {
                    expression: lower_where_clause(child, source, diagnostics),
                });
            }
            SyntaxKind::LetClause => {
                clauses.push(lower_let_clause(child, source, diagnostics));
            }
            SyntaxKind::ReturnClause => {
                let projections = lower_return_clause(child, source);
                if projections
                    .iter()
                    .any(|projection| projection.alias.is_some())
                {
                    clauses.push(QueryClause::ReturnAliased { projections });
                } else {
                    clauses.push(QueryClause::Return {
                        expressions: projections
                            .into_iter()
                            .map(|projection| projection.expression)
                            .collect(),
                    });
                }
            }
            SyntaxKind::UnionClause => clauses.push(QueryClause::Union { span: child.span() }),
            SyntaxKind::LimitClause => clauses.push(QueryClause::Limit {
                value: first_number(child),
                span: child.span(),
            }),
            SyntaxKind::OrderByClause => clauses.push(QueryClause::OrderBy {
                keys: lower_order_by_clause(child, source),
                span: child.span(),
            }),
            SyntaxKind::OffsetClause => clauses.push(QueryClause::Offset {
                value: first_number(child),
                span: child.span(),
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
        let Some(child) = syntax_node(&element) else {
            continue;
        };
        match child.kind() {
            SyntaxKind::GraphPattern => {
                pattern = lower_graph_pattern(child, source);
                break;
            }
            SyntaxKind::PathPattern => {
                pattern = GraphPattern {
                    elements: vec![PatternElement::Path(lower_path_pattern(child, source))],
                    span: child.span(),
                };
                break;
            }
            _ => {}
        }
    }

    MatchClause {
        pattern,
        span: node.span(),
    }
}

fn lower_optional_match_clause(node: &SyntaxNode, source: &str) -> MatchClause {
    node.children()
        .into_iter()
        .find_map(|element| {
            let SyntaxElementKind::Node(child) = element.kind else {
                return None;
            };
            (child.kind() == SyntaxKind::MatchClause).then(|| lower_match_clause(&child, source))
        })
        .unwrap_or(MatchClause {
            pattern: GraphPattern {
                elements: Vec::new(),
                span: node.span(),
            },
            span: node.span(),
        })
}

fn lower_graph_pattern(node: &SyntaxNode, source: &str) -> GraphPattern {
    let elements_in = node.children();
    let mut elements = Vec::new();

    for element in elements_in.iter() {
        match &element.kind {
            SyntaxElementKind::Node(child_node) => match child_node.kind() {
                SyntaxKind::NodePattern => {
                    elements.push(PatternElement::Node(lower_node_pattern(child_node, source)))
                }
                SyntaxKind::EdgePattern => {
                    elements.push(PatternElement::Edge(lower_edge_pattern(child_node, source)));
                }
                SyntaxKind::PathPattern => {
                    elements.push(PatternElement::Path(lower_path_pattern(child_node, source)))
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

fn lower_path_pattern(node: &SyntaxNode, source: &str) -> PathPattern {
    let binding = syntax_tokens(node.children()).find_map(|token| {
        (token.kind == TokenKind::Identifier).then(|| identifier_from_token(&token, source))
    });
    let elements = node
        .children()
        .into_iter()
        .find_map(|element| {
            let child = syntax_node(&element)?;
            (child.kind() == SyntaxKind::GraphPattern)
                .then(|| lower_graph_pattern(child, source).elements)
        })
        .unwrap_or_default();

    PathPattern {
        binding,
        elements,
        span: node.span(),
    }
}

fn lower_path_quantifier(node: &SyntaxNode, _source: &str) -> Option<PathQuantifier> {
    let quantifier = node.children().into_iter().find_map(|element| {
        let SyntaxElementKind::Node(child) = element.kind else {
            return None;
        };
        (child.kind() == SyntaxKind::PathQuantifier).then_some(child)
    })?;
    let mut numbers = syntax_tokens(quantifier.children())
        .filter(|token| token.kind == TokenKind::Number)
        .map(|token| token.text().parse::<u32>().ok());
    let min = numbers.next().flatten()?;
    let max = numbers.next().flatten();

    Some(PathQuantifier {
        min,
        max,
        span: quantifier.span(),
    })
}

fn edge_direction_from_pattern(edge: &SyntaxNode) -> EdgeDirection {
    let mut first_token = None;
    let mut last_token = None;

    for token in syntax_tokens(edge.children()) {
        if matches!(token.kind, TokenKind::Punctuation(_)) {
            if first_token.is_none() {
                first_token = Some(token.kind);
            }
            last_token = Some(token.kind);
        }
    }

    match (first_token, last_token) {
        (Some(TokenKind::Punctuation('<')), _) => EdgeDirection::In,
        (Some(TokenKind::Punctuation('-')), Some(TokenKind::Punctuation('>'))) => {
            EdgeDirection::Out
        }
        _ => EdgeDirection::Undirected,
    }
}

fn lower_edge_pattern(node: &SyntaxNode, source: &str) -> EdgePattern {
    let label_tokens = node
        .children()
        .into_iter()
        .find_map(|element| match element.kind {
            SyntaxElementKind::Node(child) if child.kind() == SyntaxKind::LabelList => {
                Some(syntax_tokens(child.children()).collect::<Vec<_>>())
            }
            _ => None,
        });
    let mut binding = None;
    let mut labels = Vec::new();
    if let Some(tokens) = label_tokens {
        let has_colon = tokens
            .iter()
            .any(|token| token.kind == TokenKind::Punctuation(':'));
        let mut in_labels = !has_colon;
        for token in tokens {
            match token.kind {
                TokenKind::Punctuation(':') => in_labels = true,
                TokenKind::Identifier | TokenKind::Keyword(_) => {
                    let identifier = identifier_from_token(&token, source);
                    if !in_labels && binding.is_none() {
                        binding = Some(identifier);
                    } else {
                        labels.push(identifier);
                    }
                }
                _ => {}
            }
        }
    }

    EdgePattern {
        binding,
        labels,
        properties: lower_pattern_properties(node, source),
        direction: edge_direction_from_pattern(node),
        quantifier: lower_path_quantifier(node, source),
        span: node.span(),
    }
}

fn lower_return_clause(node: &SyntaxNode, source: &str) -> Vec<ReturnProjection> {
    let mut projections = Vec::new();
    let mut pending_expression = None;
    for element in node.children() {
        let Some(child) = syntax_node(&element) else {
            continue;
        };
        if is_expression_kind(child.kind())
            && let Some(expression) = lower_expression(child, source)
            && let Some(expression) = pending_expression.replace(expression)
        {
            projections.push(ReturnProjection {
                expression,
                alias: None,
            });
        } else if child.kind() == SyntaxKind::ProjectionAlias
            && let Some(expression) = pending_expression.take()
        {
            projections.push(ReturnProjection {
                expression,
                alias: first_identifier(child, source),
            });
        }
    }
    if let Some(expression) = pending_expression {
        projections.push(ReturnProjection {
            expression,
            alias: None,
        });
    }
    projections
}

fn lower_where_clause(
    node: &SyntaxNode,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Expression {
    let expressions: Vec<_> = node
        .children()
        .iter()
        .filter_map(|element| match element {
            SyntaxElement {
                kind: SyntaxElementKind::Node(child_node),
            } if is_expression_kind(child_node.kind()) => lower_expression(child_node, source),
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
    match node.kind() {
        SyntaxKind::NameExpression | SyntaxKind::LiteralExpression => {
            node.children().iter().find_map(|element| match element {
                SyntaxElement {
                    kind: SyntaxElementKind::Token(token),
                } => lower_expression_token(token, source),
                SyntaxElement {
                    kind: SyntaxElementKind::Node(_),
                } => None,
            })
        }
        SyntaxKind::UnaryExpression => {
            let operand = node.children().iter().find_map(|element| match element {
                SyntaxElement {
                    kind: SyntaxElementKind::Node(child),
                } if is_expression_kind(child.kind()) => lower_expression(child, source),
                _ => None,
            })?;
            Some(Expression::Unary {
                operator: UnaryOperator::Not,
                operand: Box::new(operand),
            })
        }
        SyntaxKind::BinaryExpression => {
            let children = node.children();
            let mut operands = children.iter().filter_map(|element| match element {
                SyntaxElement {
                    kind: SyntaxElementKind::Node(child),
                } if is_expression_kind(child.kind()) => Some(child),
                _ => None,
            });
            let left = lower_expression(operands.next()?, source)?;
            let right = lower_expression(operands.next()?, source)?;
            Some(Expression::Binary {
                operator: binary_operator_from_node(node)?,
                left: Box::new(left),
                right: Box::new(right),
            })
        }
        SyntaxKind::PropertyAccessExpression => {
            let base = node.children().iter().find_map(|element| match element {
                SyntaxElement {
                    kind: SyntaxElementKind::Node(child),
                } if is_expression_kind(child.kind()) => lower_expression(child, source),
                _ => None,
            })?;
            let property = node.children().iter().find_map(|element| match element {
                SyntaxElement {
                    kind: SyntaxElementKind::Token(token),
                } if token.kind == TokenKind::Identifier => {
                    Some(identifier_from_token(token, source))
                }
                _ => None,
            })?;
            Some(Expression::PropertyAccess {
                base: Box::new(base),
                property,
            })
        }
        SyntaxKind::ListExpression => {
            let items = node
                .children()
                .iter()
                .filter_map(|element| match element {
                    SyntaxElement {
                        kind: SyntaxElementKind::Node(child),
                    } if is_expression_kind(child.kind()) => lower_expression(child, source),
                    _ => None,
                })
                .collect::<Vec<_>>();
            Some(Expression::List(items, node.span()))
        }
        SyntaxKind::SubscriptExpression => {
            let children = node.children();
            let mut expressions = children.iter().filter_map(|element| match element {
                SyntaxElement {
                    kind: SyntaxElementKind::Node(child),
                } if is_expression_kind(child.kind()) => lower_expression(child, source),
                _ => None,
            });
            let base = expressions.next()?;
            let index = expressions.next()?;
            Some(Expression::Subscript {
                base: Box::new(base),
                index: Box::new(index),
            })
        }
        SyntaxKind::CaseExpression => lower_case_expression(node, source),
        SyntaxKind::ParenthesizedExpression => {
            node.children().iter().find_map(|element| match element {
                SyntaxElement {
                    kind: SyntaxElementKind::Node(child),
                } if is_expression_kind(child.kind()) => lower_expression(child, source),
                _ => None,
            })
        }
        SyntaxKind::Expression => {
            let children = node.children();
            parse_expression_elements(&children, source)
        }
        _ => None,
    }
}

fn is_expression_kind(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Expression
            | SyntaxKind::NameExpression
            | SyntaxKind::LiteralExpression
            | SyntaxKind::UnaryExpression
            | SyntaxKind::BinaryExpression
            | SyntaxKind::PropertyAccessExpression
            | SyntaxKind::ListExpression
            | SyntaxKind::SubscriptExpression
            | SyntaxKind::ParenthesizedExpression
            | SyntaxKind::CaseExpression
    )
}

fn lower_case_expression(node: &SyntaxNode, source: &str) -> Option<Expression> {
    let children = node.children();
    let first_branch = children.iter().position(|element| {
        matches!(
            element,
            SyntaxElement {
                kind: SyntaxElementKind::Node(child),
            } if child.kind() == SyntaxKind::CaseWhenClause
        )
    })?;
    let operand = children[..first_branch]
        .iter()
        .find_map(|element| match element {
            SyntaxElement {
                kind: SyntaxElementKind::Node(child),
            } if is_expression_kind(child.kind()) => lower_expression(child, source).map(Box::new),
            _ => None,
        });
    let branches = children
        .iter()
        .filter_map(|element| match element {
            SyntaxElement {
                kind: SyntaxElementKind::Node(child),
            } if child.kind() == SyntaxKind::CaseWhenClause => {
                let expressions = child
                    .children()
                    .iter()
                    .filter_map(|element| match element {
                        SyntaxElement {
                            kind: SyntaxElementKind::Node(expression),
                        } if is_expression_kind(expression.kind()) => {
                            lower_expression(expression, source)
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                (expressions.len() == 2).then(|| super::types::CaseBranch {
                    condition: expressions[0].clone(),
                    result: expressions[1].clone(),
                    span: child.span(),
                })
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let else_result = children.iter().find_map(|element| match element {
        SyntaxElement {
            kind: SyntaxElementKind::Node(child),
        } if child.kind() == SyntaxKind::CaseElseClause => child
            .children()
            .iter()
            .find_map(|element| match element {
                SyntaxElement {
                    kind: SyntaxElementKind::Node(expression),
                } if is_expression_kind(expression.kind()) => lower_expression(expression, source),
                _ => None,
            })
            .map(Box::new),
        _ => None,
    });

    Some(Expression::Case {
        operand,
        branches,
        else_result,
        span: node.span(),
    })
}

fn binary_operator_from_node(node: &SyntaxNode) -> Option<BinaryOperator> {
    let tokens = syntax_tokens(node.children())
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))
        .map(|token| token.kind)
        .collect::<Vec<_>>();
    match tokens.as_slice() {
        [TokenKind::Punctuation('+')] => Some(BinaryOperator::Add),
        [TokenKind::Punctuation('-')] => Some(BinaryOperator::Subtract),
        [TokenKind::Punctuation('*')] => Some(BinaryOperator::Multiply),
        [TokenKind::Punctuation('/')] => Some(BinaryOperator::Divide),
        [TokenKind::Punctuation('%')] => Some(BinaryOperator::Modulo),
        [TokenKind::Keyword(Keyword::In)] => Some(BinaryOperator::In),
        [TokenKind::Keyword(Keyword::Or)] => Some(BinaryOperator::Or),
        [TokenKind::Keyword(Keyword::And)] => Some(BinaryOperator::And),
        [TokenKind::Punctuation('=')] => Some(BinaryOperator::Equals),
        [TokenKind::Punctuation('!'), TokenKind::Punctuation('=')] => {
            Some(BinaryOperator::NotEquals)
        }
        [TokenKind::Punctuation('<')] => Some(BinaryOperator::LessThan),
        [TokenKind::Punctuation('<'), TokenKind::Punctuation('=')] => {
            Some(BinaryOperator::LessThanOrEqual)
        }
        [TokenKind::Punctuation('>')] => Some(BinaryOperator::GreaterThan),
        [TokenKind::Punctuation('>'), TokenKind::Punctuation('=')] => {
            Some(BinaryOperator::GreaterThanOrEqual)
        }
        _ => None,
    }
}

fn parse_expression_elements(elements: &[SyntaxElement], source: &str) -> Option<Expression> {
    let compact = collect_non_trivia_elements(elements);
    let first = compact.first()?;

    if compact.len() == 1 {
        return parse_expression_atom(first, source);
    }

    if matches_parenthesized_expression(&compact) {
        return parse_expression_atom(compact[1], source);
    }

    if let Some(expression) = parse_expression_unary(&compact, source) {
        return Some(expression);
    }

    let mut index = 0usize;
    let mut lhs = parse_expression_atom(compact[index], source)?;
    index += 1;

    while index < compact.len() {
        let (operator, consumed) = parse_expression_binary_operator(&compact, index)?;
        index += consumed;
        let rhs_element = compact.get(index)?;
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
        } if is_expression_kind(child.kind()) => lower_expression(child, source),
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
    } = token_element
    else {
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
    } = elements.get(index)?
    else {
        return None;
    };
    match token.kind {
        TokenKind::Punctuation('+') => Some((BinaryOperator::Add, 1)),
        TokenKind::Punctuation('-') => Some((BinaryOperator::Subtract, 1)),
        TokenKind::Punctuation('*') => Some((BinaryOperator::Multiply, 1)),
        TokenKind::Punctuation('/') => Some((BinaryOperator::Divide, 1)),
        TokenKind::Punctuation('%') => Some((BinaryOperator::Modulo, 1)),
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
        } => is_expression_kind(node.kind()),
        _ => false,
    })
}

fn collect_non_trivia_elements(elements: &[SyntaxElement]) -> Vec<&SyntaxElement> {
    elements
        .iter()
        .filter(|element| match element {
            SyntaxElement {
                kind: SyntaxElementKind::Token(token),
            } => !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment),
            SyntaxElement {
                kind: SyntaxElementKind::Node(_),
            } => true,
        })
        .collect()
}

fn lower_expression_token(token: &Token, source: &str) -> Option<Expression> {
    match token.kind {
        TokenKind::Identifier => Some(Expression::Name(identifier_from_token(token, source))),
        TokenKind::Keyword(Keyword::True) => Some(Expression::Boolean(true, token.span)),
        TokenKind::Keyword(Keyword::False) => Some(Expression::Boolean(false, token.span)),
        TokenKind::Keyword(Keyword::Null) => Some(Expression::Null(token.span)),
        TokenKind::Keyword(_) => None,
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
        TokenKind::Number => {
            let value = token_text(token, source);
            if value.contains('.') {
                Some(Expression::Decimal(value, token.span))
            } else {
                value
                    .parse::<i64>()
                    .ok()
                    .map(|value| Expression::Integer(value, token.span))
            }
        }
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
            } if is_expression_kind(child.kind()) => {
                if binding.is_none() {
                    if let Some(expression) = lower_expression(&child, source) {
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
                        value = lower_expression(&child, source).or_else(|| {
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

    let pattern_children = node
        .children()
        .iter()
        .filter(|element| {
            !matches!(
                element,
                SyntaxElement {
                    kind: SyntaxElementKind::Node(child),
                } if child.kind() == SyntaxKind::PropertyMap
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    for token in syntax_tokens(pattern_children) {
        match token.kind {
            TokenKind::Whitespace => {}
            TokenKind::Punctuation(':') => in_labels = true,
            TokenKind::Identifier | TokenKind::Keyword(_) => {
                let identifier = identifier_from_token(&token, source);
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
        properties: lower_pattern_properties(node, source),
        span: node.span(),
    }
}

fn lower_pattern_properties(
    node: &SyntaxNode,
    source: &str,
) -> Vec<super::types::PropertyConstraint> {
    let mut properties = Vec::new();
    for element in node.children() {
        let SyntaxElementKind::Node(property_map) = &element.kind else {
            continue;
        };
        if property_map.kind() != SyntaxKind::PropertyMap {
            continue;
        }
        for map_element in property_map.children() {
            let SyntaxElementKind::Node(entry) = &map_element.kind else {
                continue;
            };
            if entry.kind() != SyntaxKind::PropertyEntry {
                continue;
            }
            let key = entry.children().iter().find_map(|child| match child {
                SyntaxElement {
                    kind: SyntaxElementKind::Token(token),
                } if matches!(token.kind, TokenKind::Identifier | TokenKind::Keyword(_)) => {
                    Some(identifier_from_token(token, source))
                }
                _ => None,
            });
            let value = entry.children().iter().find_map(|child| match child {
                SyntaxElement {
                    kind: SyntaxElementKind::Node(expression),
                } if is_expression_kind(expression.kind()) => lower_expression(expression, source),
                _ => None,
            });
            if let (Some(key), Some(value)) = (key, value) {
                properties.push(super::types::PropertyConstraint {
                    key,
                    value,
                    span: entry.span(),
                });
            }
        }
    }
    properties
}

fn syntax_node(element: &SyntaxElement) -> Option<&SyntaxNode> {
    if let SyntaxElementKind::Node(node) = &element.kind {
        Some(node)
    } else {
        None
    }
}

fn syntax_tokens(elements: Vec<SyntaxElement>) -> impl Iterator<Item = Token> {
    elements
        .into_iter()
        .filter_map(|element| match element.kind {
            SyntaxElementKind::Token(token) => Some(token),
            SyntaxElementKind::Node(_) => None,
        })
}

fn token_text(token: &Token, _source: &str) -> String {
    token.text().to_string()
}

fn identifier_from_token(token: &Token, source: &str) -> Identifier {
    Identifier {
        text: token_text(token, source),
        span: token.span,
    }
}

fn first_identifier(node: &SyntaxNode, source: &str) -> Option<Identifier> {
    node.children()
        .into_iter()
        .find_map(|element| match element.kind {
            SyntaxElementKind::Token(token) if token.kind == TokenKind::Identifier => {
                Some(identifier_from_token(&token, source))
            }
            _ => None,
        })
}

fn first_number(node: &SyntaxNode) -> Option<u64> {
    node.children()
        .into_iter()
        .find_map(|element| match element.kind {
            SyntaxElementKind::Token(token) if token.kind == TokenKind::Number => {
                token.text().parse().ok()
            }
            _ => None,
        })
}

fn lower_order_by_clause(node: &SyntaxNode, source: &str) -> Vec<SortKey> {
    let mut keys = Vec::new();
    let mut pending = None;
    for element in node.children() {
        match element.kind {
            SyntaxElementKind::Node(child)
                if is_expression_kind(child.kind())
                    && let Some(expression) = lower_expression(&child, source)
                    && let Some(expression) = pending.replace(expression) =>
            {
                keys.push(SortKey {
                    expression,
                    direction: SortDirection::Ascending,
                });
            }
            SyntaxElementKind::Token(token)
                if matches!(token.kind, TokenKind::Keyword(Keyword::Asc | Keyword::Desc)) =>
            {
                if let Some(expression) = pending.take() {
                    keys.push(SortKey {
                        expression,
                        direction: if token.kind == TokenKind::Keyword(Keyword::Desc) {
                            SortDirection::Descending
                        } else {
                            SortDirection::Ascending
                        },
                    });
                }
            }
            _ => {}
        }
    }
    if let Some(expression) = pending {
        keys.push(SortKey {
            expression,
            direction: SortDirection::Ascending,
        });
    }
    keys
}
