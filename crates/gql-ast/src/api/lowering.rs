//! Lowering helpers for converting parse trees into public GQL AST structures.
#![forbid(unsafe_code)]

use super::aggregate_lowering::lower_aggregate_call;
use super::data_management_lowering::{
    lower_catalog_statement, lower_data_clause, lower_procedure_statement, lower_session_statement,
    lower_transaction_statement,
};
use super::general_literal_lowering::lower_general_literal;
use super::identifier_lowering::identifier_from_token;
use super::label_lowering::lower_label_predicate;
use super::lowering_support::is_expression_kind;
use super::numeric_lowering::lower_numeric_literal;
use super::order_page_lowering::{lower_non_negative_integer_specification, lower_order_by_clause};
use super::pattern_graph_lowering::{lower_path_pattern, lower_path_prefix};
use super::predicate_lowering::{lower_graph_element_predicate, lower_predicate_test};
use super::primitive_query_lowering::lower_primitive_query_clause;
use super::value_type_predicate_lowering::lower_value_type_predicate;
use super::{
    BinaryOperator, CharacterStringForm, CharacterStringLiteral, DynamicParameterReference,
    Expression, GraphMatchMode, Identifier, LetBinding, MatchClause, ParameterNameForm, Query,
    QueryClause, ReturnProjection, Statement, SyntaxParseOutput, UnaryOperator,
};
use gql_source::{Diagnostic, Span};
use gql_syntax::{
    CharacterStringForm as SyntaxCharacterStringForm, Keyword,
    ParameterNameForm as SyntaxParameterNameForm, Parse as SyntaxParse, SyntaxElement,
    SyntaxElementKind, SyntaxKind, SyntaxNode, Token, TokenKind, decode_character_string,
    decode_parameter_reference, is_non_reserved_word,
};
/// Lower a parsed syntax tree into the AST result used by semantic analysis.
#[must_use]
pub fn lower_from_syntax(parse: &SyntaxParse) -> SyntaxParseOutput {
    if !parse.diagnostics.is_empty() {
        return SyntaxParseOutput {
            statement: None,
            diagnostics: parse.diagnostics.clone(),
        };
    }

    let mut diagnostics = Vec::new();
    let root = parse.tree.root();
    let mut statement =
        lower_statement_from_syntax_root(&root, parse.tree.source().text(), &mut diagnostics);
    if !diagnostics.is_empty() {
        statement = None;
    }
    SyntaxParseOutput {
        statement,
        diagnostics,
    }
}

fn lower_statement_from_syntax_root(
    root: &SyntaxNode,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Statement> {
    let mut clauses = Vec::new();
    let mut saw_query_node = false;
    for element in root.children() {
        let Some(node) = syntax_node(&element) else {
            continue;
        };
        if node.kind() == SyntaxKind::Query {
            saw_query_node = true;
            if let Some(catalog) = lower_catalog_statement(node, source) {
                return Some(Statement::Catalog(catalog));
            }
            if let Some(procedure) = lower_procedure_statement(node, source) {
                return Some(Statement::Procedure(procedure));
            }
            if let Some(transaction) = lower_transaction_statement(node) {
                return Some(Statement::Transaction(transaction));
            }
            if let Some(session) = lower_session_statement(node, source) {
                return Some(Statement::Session(session));
            }
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

    (!clauses.is_empty()).then_some(Statement::Query(Query {
        clauses,
        span: root.span(),
    }))
}

fn lower_query(node: &SyntaxNode, source: &str, diagnostics: &mut Vec<Diagnostic>) -> Query {
    let mut clauses = Vec::new();
    for element in node.children() {
        let Some(child) = syntax_node(&element) else {
            continue;
        };

        if let Some(clause) = lower_data_clause(child, source, diagnostics) {
            clauses.push(clause);
            continue;
        }

        match child.kind() {
            SyntaxKind::MatchClause => match lower_match_clause(child, source) {
                Some(match_clause) => clauses.push(QueryClause::Match(match_clause)),
                None => diagnostics.push(Diagnostic::error(
                    "GQL-AST-MATCH-MISSING-PATTERN",
                    "MATCH clause is missing a graph pattern",
                    child.span(),
                )),
            },
            SyntaxKind::OptionalMatchClause => match lower_optional_match_clause(child, source) {
                Some(match_clause) => clauses.push(QueryClause::OptionalMatch(match_clause)),
                None => diagnostics.push(Diagnostic::error(
                    "GQL-AST-OPTIONAL-MATCH-MISSING-PATTERN",
                    "OPTIONAL MATCH clause is missing a graph pattern",
                    child.span(),
                )),
            },
            SyntaxKind::WhereClause => {
                if let Some(expression) = lower_where_clause(child, source, diagnostics) {
                    clauses.push(QueryClause::Where {
                        expression,
                        span: significant_node_span(child),
                    });
                }
            }
            SyntaxKind::LetClause => {
                if let Some(clause) = lower_let_clause(child, source, diagnostics) {
                    clauses.push(clause);
                }
            }
            SyntaxKind::FilterStatement => {
                if let Some(clause) =
                    lower_primitive_query_clause(child, source, diagnostics, lower_expression)
                {
                    clauses.push(clause);
                }
            }
            SyntaxKind::ForStatement => {
                if let Some(clause) =
                    lower_primitive_query_clause(child, source, diagnostics, lower_expression)
                {
                    clauses.push(clause);
                }
            }
            SyntaxKind::ReturnClause => {
                let projections = lower_return_clause(child, source);
                clauses.push(QueryClause::Return {
                    quantifier: lower_result_set_quantifier(child),
                    all_bindings: syntax_tokens(child.children())
                        .any(|token| token.kind == TokenKind::Punctuation('*')),
                    projections,
                    span: significant_node_span(child),
                });
            }
            SyntaxKind::FinishStatement => clauses.push(QueryClause::Finish {
                span: significant_node_span(child),
            }),
            SyntaxKind::UnionClause => clauses.push(QueryClause::Union { span: child.span() }),
            SyntaxKind::LimitClause => {
                if let Some(value) = lower_non_negative_integer_specification(child) {
                    clauses.push(QueryClause::Limit {
                        value,
                        span: child.span(),
                    });
                }
            }
            SyntaxKind::OrderByClause => clauses.push(QueryClause::OrderBy {
                keys: lower_order_by_clause(child, source, lower_expression),
                span: child.span(),
            }),
            SyntaxKind::OffsetClause => {
                if let Some(value) = lower_non_negative_integer_specification(child) {
                    clauses.push(QueryClause::Offset {
                        value,
                        span: significant_node_span(child),
                    });
                }
            }
            SyntaxKind::GroupByClause => clauses.push(QueryClause::GroupBy {
                keys: lower_expression_list(child, source),
                span: significant_node_span(child),
            }),
            _ => {}
        }
    }

    if clauses.is_empty() && diagnostics.is_empty() {
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

pub(super) fn significant_node_span(node: &SyntaxNode) -> Span {
    let mut spans = node
        .children()
        .into_iter()
        .filter_map(|element| match element.kind {
            SyntaxElementKind::Node(child) => Some(child.span()),
            SyntaxElementKind::Token(token)
                if !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment) =>
            {
                Some(token.span)
            }
            SyntaxElementKind::Token(_) => None,
        });
    let Some(first) = spans.next() else {
        return node.span();
    };
    spans.fold(first, |span, next| Span::new(span.start, next.end))
}

fn lower_match_clause(node: &SyntaxNode, source: &str) -> Option<MatchClause> {
    let children = node.children();
    let patterns = children.iter().find_map(|element| {
        let list = syntax_node(element)?;
        (list.kind() == SyntaxKind::GraphPatternList).then(|| {
            list.children()
                .iter()
                .filter_map(|element| {
                    let child = syntax_node(element)?;
                    (child.kind() == SyntaxKind::PathPattern)
                        .then(|| lower_path_pattern(child, source))
                        .flatten()
                })
                .collect::<Vec<_>>()
        })
    })?;
    if patterns.is_empty() {
        return None;
    }

    Some(MatchClause {
        mode: children.iter().find_map(|element| {
            let mode = syntax_node(element)?;
            (mode.kind() == SyntaxKind::GraphMatchMode).then(|| {
                if syntax_tokens(mode.children())
                    .any(|token| token.text().eq_ignore_ascii_case("REPEATABLE"))
                {
                    GraphMatchMode::RepeatableElements
                } else {
                    GraphMatchMode::DifferentEdges
                }
            })
        }),
        patterns,
        keep: children.iter().find_map(|element| {
            let keep = syntax_node(element)?;
            (keep.kind() == SyntaxKind::KeepClause)
                .then(|| {
                    keep.children().iter().find_map(|element| {
                        let prefix = syntax_node(element)?;
                        (prefix.kind() == SyntaxKind::PathPrefix)
                            .then(|| lower_path_prefix(prefix))
                            .flatten()
                    })
                })
                .flatten()
        }),
        span: node.span(),
    })
}

fn lower_optional_match_clause(node: &SyntaxNode, source: &str) -> Option<MatchClause> {
    node.children().into_iter().find_map(|element| {
        let SyntaxElementKind::Node(child) = element.kind else {
            return None;
        };
        (child.kind() == SyntaxKind::MatchClause)
            .then(|| lower_match_clause(&child, source))
            .flatten()
    })
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

fn lower_result_set_quantifier(node: &SyntaxNode) -> Option<super::SetQuantifier> {
    node.children().iter().find_map(|element| {
        let child = syntax_node(element)?;
        (child.kind() == SyntaxKind::SetQuantifier).then(|| {
            if syntax_tokens(child.children())
                .any(|token| token.kind == TokenKind::Keyword(Keyword::Distinct))
            {
                super::SetQuantifier::Distinct
            } else {
                super::SetQuantifier::All
            }
        })
    })
}

fn lower_where_clause(
    node: &SyntaxNode,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Expression> {
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
        return expressions.into_iter().next();
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

    None
}

pub(super) fn lower_expression(node: &SyntaxNode, source: &str) -> Option<Expression> {
    match node.kind() {
        SyntaxKind::NameExpression
        | SyntaxKind::LiteralExpression
        | SyntaxKind::CharacterStringLiteralExpression
        | SyntaxKind::DynamicParameterExpression => {
            node.children().iter().find_map(|element| match element {
                SyntaxElement {
                    kind: SyntaxElementKind::Token(token),
                } => lower_expression_token(token, source),
                SyntaxElement {
                    kind: SyntaxElementKind::Node(_),
                } => None,
            })
        }
        SyntaxKind::ByteStringLiteralExpression
        | SyntaxKind::TemporalLiteralExpression
        | SyntaxKind::DurationLiteralExpression
        | SyntaxKind::RecordExpression => lower_general_literal(node, source),
        SyntaxKind::UnaryExpression => {
            let operator = node.children().iter().find_map(|element| match element {
                SyntaxElement {
                    kind: SyntaxElementKind::Token(token),
                } => match token.kind {
                    TokenKind::Keyword(Keyword::Not) => Some(UnaryOperator::Not),
                    TokenKind::Punctuation('+') => Some(UnaryOperator::Plus),
                    TokenKind::Punctuation('-') => Some(UnaryOperator::Negate),
                    _ => None,
                },
                _ => None,
            })?;
            let operand = node.children().iter().find_map(|element| match element {
                SyntaxElement {
                    kind: SyntaxElementKind::Node(child),
                } if is_expression_kind(child.kind()) => lower_expression(child, source),
                _ => None,
            })?;
            Some(Expression::Unary {
                operator,
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
        SyntaxKind::NullPredicateExpression | SyntaxKind::TruthPredicateExpression => {
            lower_predicate_test(node, |child| {
                is_expression_kind(child.kind())
                    .then(|| lower_expression(child, source))
                    .flatten()
            })
        }
        SyntaxKind::ValueTypePredicateExpression => lower_value_type_predicate(node, source),
        SyntaxKind::DirectedPredicateExpression
        | SyntaxKind::EndpointPredicateExpression
        | SyntaxKind::ElementIdentityPredicateExpression
        | SyntaxKind::PropertyExistsPredicateExpression => {
            lower_graph_element_predicate(node, source)
        }
        SyntaxKind::LabelPredicateExpression => lower_label_predicate(node, source),
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
        SyntaxKind::FunctionCallExpression => {
            let children = node.children();
            let mut expressions = children.iter().filter_map(|element| {
                let child = syntax_node(element)?;
                is_expression_kind(child.kind()).then_some(child)
            });
            let Expression::Name(name) = lower_expression(expressions.next()?, source)? else {
                return None;
            };
            let arguments = expressions
                .filter_map(|argument| lower_expression(argument, source))
                .collect();
            Some(Expression::FunctionCall {
                name,
                arguments,
                span: significant_node_span(node),
            })
        }
        SyntaxKind::AggregateFunctionExpression => lower_aggregate_call(node, source),
        SyntaxKind::ListExpression => {
            let items = node
                .children()
                .iter()
                .filter_map(|element| match element {
                    SyntaxElement {
                        kind: SyntaxElementKind::Node(child),
                    } if is_expression_kind(child.kind()) => Some(child),
                    _ => None,
                })
                .map(|child| lower_expression(child, source))
                .collect::<Option<Vec<_>>>()?;
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
        [TokenKind::Punctuation('|'), TokenKind::Punctuation('|')] => {
            Some(BinaryOperator::Concatenate)
        }
        [TokenKind::Keyword(Keyword::In)] => Some(BinaryOperator::In),
        [TokenKind::Keyword(Keyword::Or)] => Some(BinaryOperator::Or),
        [TokenKind::Keyword(Keyword::Xor)] => Some(BinaryOperator::Xor),
        [TokenKind::Keyword(Keyword::And)] => Some(BinaryOperator::And),
        [TokenKind::Punctuation('=')] => Some(BinaryOperator::Equals),
        [TokenKind::Punctuation('<'), TokenKind::Punctuation('>')] => {
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
    let operator = match token.kind {
        TokenKind::Keyword(Keyword::Not) => UnaryOperator::Not,
        TokenKind::Punctuation('+') => UnaryOperator::Plus,
        TokenKind::Punctuation('-') => UnaryOperator::Negate,
        _ => return None,
    };
    let operand_element = elements.get(1)?;
    let operand = parse_expression_atom(operand_element, source)?;
    Some(Expression::Unary {
        operator,
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
        TokenKind::Punctuation('|') => matches!(
            elements.get(index + 1),
            Some(SyntaxElement {
                kind: SyntaxElementKind::Token(Token {
                    kind: TokenKind::Punctuation('|'),
                    ..
                }),
            })
        )
        .then_some((BinaryOperator::Concatenate, 2)),
        TokenKind::Keyword(Keyword::Or) => Some((BinaryOperator::Or, 1)),
        TokenKind::Keyword(Keyword::Xor) => Some((BinaryOperator::Xor, 1)),
        TokenKind::Keyword(Keyword::And) => Some((BinaryOperator::And, 1)),
        TokenKind::Punctuation('=') => Some((BinaryOperator::Equals, 1)),
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
            let next_is_greater = matches!(
                elements.get(index + 1),
                Some(SyntaxElement {
                    kind: SyntaxElementKind::Token(Token {
                        kind: TokenKind::Punctuation('>'),
                        ..
                    }),
                })
            );
            if next_is_greater {
                Some((BinaryOperator::NotEquals, 2))
            } else if next_is_equals {
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
        TokenKind::Identifier
            if token.text().starts_with('"') || token.text().starts_with("@\"") =>
        {
            lower_character_string_literal(token)
        }
        TokenKind::Identifier => Some(Expression::Name(identifier_from_token(token, source))),
        TokenKind::DynamicParameter => {
            let decoded = decode_parameter_reference(token.text())?;
            Some(Expression::Parameter(DynamicParameterReference {
                name: decoded.name.into_owned(),
                form: match decoded.form {
                    SyntaxParameterNameForm::Extended => ParameterNameForm::Extended,
                    SyntaxParameterNameForm::Delimited => ParameterNameForm::Delimited,
                },
                span: token.span,
            }))
        }
        TokenKind::Keyword(_) if is_non_reserved_word(token.text()) => {
            Some(Expression::Name(identifier_from_token(token, source)))
        }
        TokenKind::Keyword(Keyword::True) => Some(Expression::Boolean(true, token.span)),
        TokenKind::Keyword(Keyword::False) => Some(Expression::Boolean(false, token.span)),
        TokenKind::Keyword(Keyword::Null) => Some(Expression::Null(token.span)),
        TokenKind::Keyword(_) => None,
        TokenKind::String => lower_character_string_literal(token),
        TokenKind::Number => lower_numeric_literal(token.text(), token.span),
        _ => None,
    }
}

fn lower_character_string_literal(token: &Token) -> Option<Expression> {
    let decoded = decode_character_string(token.text())?;
    let form = match decoded.form {
        SyntaxCharacterStringForm::SingleQuoted => CharacterStringForm::SingleQuoted,
        SyntaxCharacterStringForm::DoubleQuoted => CharacterStringForm::DoubleQuoted,
        SyntaxCharacterStringForm::GraveQuoted => return None,
    };
    Some(Expression::String(CharacterStringLiteral {
        value: decoded.value.into_owned(),
        form,
        no_escape: decoded.no_escape,
        span: token.span,
    }))
}

fn lower_let_clause(
    node: &SyntaxNode,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<QueryClause> {
    let mut bindings = Vec::new();
    for child in node.children().iter().filter_map(syntax_node) {
        if child.kind() != SyntaxKind::LetBinding {
            continue;
        }
        let binding = syntax_tokens(child.children()).find_map(|token| {
            (token.kind == TokenKind::Identifier).then(|| identifier_from_token(&token, source))
        });
        let value = child.children().iter().find_map(|element| {
            let expression = syntax_node(element)?;
            is_expression_kind(expression.kind())
                .then(|| lower_expression(expression, source))
                .flatten()
        });
        let Some(binding) = binding else {
            diagnostics.push(Diagnostic::error(
                if child.children().is_empty() {
                    "GQL-AST-LET-BINDING-MISSING"
                } else {
                    "GQL-AST-LET-BINDING-EXPECTED"
                },
                "LET binding requires an identifier",
                child.span(),
            ));
            continue;
        };
        let Some(value) = value else {
            diagnostics.push(Diagnostic::error(
                "GQL-AST-LET-VALUE-MISSING",
                "LET binding is missing a value",
                child.span(),
            ));
            continue;
        };
        bindings.push(LetBinding {
            binding,
            value,
            span: significant_node_span(child),
        });
    }
    (!bindings.is_empty()).then(|| QueryClause::Let {
        bindings,
        span: significant_node_span(node),
    })
}

fn lower_expression_list(node: &SyntaxNode, source: &str) -> Vec<Expression> {
    node.children()
        .iter()
        .filter_map(|element| {
            let child = syntax_node(element)?;
            is_expression_kind(child.kind())
                .then(|| lower_expression(child, source))
                .flatten()
        })
        .collect()
}

pub(super) fn syntax_node(element: &SyntaxElement) -> Option<&SyntaxNode> {
    if let SyntaxElementKind::Node(node) = &element.kind {
        Some(node)
    } else {
        None
    }
}

pub(super) fn syntax_tokens(elements: Vec<SyntaxElement>) -> impl Iterator<Item = Token> {
    elements
        .into_iter()
        .filter_map(|element| match element.kind {
            SyntaxElementKind::Token(token) => Some(token),
            SyntaxElementKind::Node(_) => None,
        })
}

fn first_identifier(node: &SyntaxNode, source: &str) -> Option<Identifier> {
    node.children()
        .into_iter()
        .find_map(|element| match element.kind {
            SyntaxElementKind::Token(token)
                if token.kind == TokenKind::Identifier || is_non_reserved_word(token.text()) =>
            {
                Some(identifier_from_token(&token, source))
            }
            _ => None,
        })
}

pub(super) fn descendant_tokens(node: &SyntaxNode) -> Vec<Token> {
    node.children()
        .into_iter()
        .flat_map(|element| match element.kind {
            SyntaxElementKind::Token(token) => vec![token],
            SyntaxElementKind::Node(child) => descendant_tokens(&child),
        })
        .collect()
}
