//! Public semantic analysis APIs for GQL statements.
#![forbid(unsafe_code)]

use std::collections::HashMap;

use gql_ast::{
    BinaryOperator, Expression, PatternElement, QueryClause, Statement,
    UnaryOperator,
};
use gql_catalog::GqlCatalog;
use gql_ir::{
    Binding as IrBinding, EdgeDirection as IrEdgeDirection, EdgePattern as IrEdgePattern,
    GraphPattern as IrGraphPattern, GraphPatternElement as IrGraphPatternElement,
    NodePattern as IrNodePattern, PathPattern as IrPathPattern, Predicate, QueryBlock,
};
use gql_types::{Value, ValueType};
use gql_source::{Diagnostic, Span};

/// Owned semantic analysis result for a parsed statement.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Analysis {
    pub ir: Option<QueryBlock>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Analyze a parsed statement and return semantic diagnostics plus optional IR.
///
/// The current implementation covers query and where/let constraints for
/// `MATCH`, `WHERE`, `LET`, and `RETURN` clauses.
#[must_use]
pub fn analyze(statement: &Statement, catalog: &dyn GqlCatalog) -> Analysis {
    analyze_statement(statement, catalog)
}

fn analyze_statement(statement: &Statement, _catalog: &dyn GqlCatalog) -> Analysis {
    let Statement::Query(query) = statement else {
        return Analysis {
            ir: None,
            diagnostics: vec![Diagnostic::error(
                "GQL-SEMA-NOT-YET-LOWERED",
                "catalog and data statements are not lowered by this foundation release",
                gql_source::Span::default(),
            )],
        };
    };
    let mut block = QueryBlock::default();
    let mut diagnostics = Vec::new();
    let mut bindings: HashMap<String, ValueType> = HashMap::new();

    for clause in &query.clauses {
        match clause {
            QueryClause::Match(match_clause) => {
                let pattern_bindings = collect_pattern_bindings(&match_clause.pattern);
                let pattern = build_graph_pattern(&match_clause.pattern);
                block.graph = Some(pattern);

                pattern_bindings
                    .iter()
                    .for_each(|binding| {
                        bindings.insert(binding.text.clone(), ValueType::Any);
                    });

            }
            QueryClause::Where { .. } => {
                if let QueryClause::Where { expression } = clause {
                    if let Some(predicate) = analyze_where_expression(expression, &bindings, &mut diagnostics)
                    {
                        block.predicates.push(predicate);
                    }
                }
            }
            QueryClause::Let { .. } => {
                if let QueryClause::Let { binding, .. } = clause {
                    if binding.text.is_empty() {
                        continue;
                    }
                    let inferred_type = let_clause_value_type(clause, &bindings, &mut diagnostics);
                    if bindings.contains_key(&binding.text) {
                        diagnostics.push(Diagnostic::error(
                            "GQL-SEMA-LET-DUPLICATE-BINDING",
                            format!("LET binding `{}` is already defined", binding.text),
                            binding.span,
                        ));
                    } else if let Some(inferred_type) = inferred_type {
                        bindings.insert(binding.text.clone(), inferred_type);
                    } else {
                        // Invalid LET value; keep binding absent to avoid propagating unresolved state.
                    }
                }
            }
            QueryClause::Return { .. } => {}
        }
    }
    Analysis {
        ir: diagnostics.is_empty().then_some(block),
        diagnostics,
    }
}

fn build_graph_pattern(pattern: &gql_ast::GraphPattern) -> IrGraphPattern {
    IrGraphPattern {
        elements: pattern
            .elements
            .iter()
            .map(build_graph_pattern_element)
            .collect(),
    }
}

fn build_graph_pattern_element(element: &PatternElement) -> IrGraphPatternElement {
    match element {
        PatternElement::Node(node) => {
            IrGraphPatternElement::Node(IrNodePattern {
                binding: node.binding.as_ref().map(|binding| binding.text.clone()),
                labels: node.labels.iter().map(|label| label.text.clone()).collect(),
            })
        }
        PatternElement::Edge(edge) => IrGraphPatternElement::Edge(IrEdgePattern {
            labels: edge.labels.iter().map(|label| label.text.clone()).collect(),
            direction: match edge.direction {
                gql_ast::EdgeDirection::Out => IrEdgeDirection::Out,
                gql_ast::EdgeDirection::In => IrEdgeDirection::In,
                gql_ast::EdgeDirection::Undirected => IrEdgeDirection::Undirected,
            },
        }),
        PatternElement::Path(path) => IrGraphPatternElement::Path(IrPathPattern {
            elements: path
                .elements
                .iter()
                .map(build_graph_pattern_element)
                .collect(),
        }),
    }
}

fn collect_pattern_bindings(clause: &gql_ast::GraphPattern) -> Vec<gql_ast::Identifier> {
    let mut bindings = Vec::new();
    collect_pattern_bindings_inner(&clause.elements, &mut bindings);
    bindings
}

fn collect_pattern_bindings_inner(
    elements: &[PatternElement],
    bindings: &mut Vec<gql_ast::Identifier>,
) {
    elements.iter().for_each(|element| match element {
        PatternElement::Node(node) => {
            if let Some(binding) = &node.binding {
                bindings.push(binding.clone());
            }
        }
        PatternElement::Path(path) => collect_pattern_bindings_inner(&path.elements, bindings),
        PatternElement::Edge(_) => {}
    });
}

fn analyze_where_expression(
    expression: &gql_ast::Expression,
    bindings: &HashMap<String, ValueType>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Predicate> {
    match expression {
        Expression::Name(identifier) => {
            if bindings.contains_key(&identifier.text) {
                None
            } else {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-WHERE-UNRESOLVED-BINDING",
                    format!("WHERE references unknown binding `{}`", identifier.text),
                    identifier.span,
                ));
                None
            }
        }
        Expression::Binary {
            operator: BinaryOperator::Equals,
            left,
            right,
        } => analyze_where_equality(left, right, bindings, diagnostics),
        Expression::Unary {
            operator: UnaryOperator::Not,
            ..
        } => {
            diagnostics.push(Diagnostic::error(
                "GQL-SEMA-WHERE-UNSUPPORTED-EXPRESSION",
                "WHERE does not yet support unary operators in this release",
                expression_span(expression),
            ));
            None
        }
        Expression::Binary { .. } => {
            diagnostics.push(Diagnostic::error(
                "GQL-SEMA-WHERE-UNSUPPORTED-EXPRESSION",
                "WHERE only supports simple binary equality in this release",
                expression_span(expression),
            ));
            None
        }
        Expression::String(_, span) | Expression::Integer(_, span) => {
            diagnostics.push(Diagnostic::error(
                "GQL-SEMA-WHERE-UNSUPPORTED-EXPRESSION",
                "WHERE only supports identifier predicates in this release",
                *span,
            ));
            None
        }
    }
}

fn analyze_where_equality(
    lhs: &gql_ast::Expression,
    rhs: &gql_ast::Expression,
    bindings: &HashMap<String, ValueType>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Predicate> {
    match (lhs, rhs) {
        (gql_ast::Expression::Name(lhs), gql_ast::Expression::Integer(rhs, _)) => {
            let Some(lhs_type) = bindings.get(&lhs.text).cloned() else {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-WHERE-UNRESOLVED-BINDING",
                    format!("WHERE references unknown binding `{}`", lhs.text),
                    lhs.span,
                ));
                return None;
            };
            Some(Predicate::Equals(
                IrBinding {
                    name: lhs.text.clone(),
                    value_type: lhs_type,
                },
                Value::Integer(*rhs),
            ))
        }
        (gql_ast::Expression::Integer(lhs, _), gql_ast::Expression::Name(rhs)) => {
            let Some(rhs_type) = bindings.get(&rhs.text).cloned() else {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-WHERE-UNRESOLVED-BINDING",
                    format!("WHERE references unknown binding `{}`", rhs.text),
                    rhs.span,
                ));
                return None;
            };
            Some(Predicate::Equals(
                IrBinding {
                    name: rhs.text.clone(),
                    value_type: rhs_type,
                },
                Value::Integer(*lhs),
            ))
        }
        (gql_ast::Expression::Name(lhs), gql_ast::Expression::String(rhs, _)) => {
            let Some(lhs_type) = bindings.get(&lhs.text).cloned() else {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-WHERE-UNRESOLVED-BINDING",
                    format!("WHERE references unknown binding `{}`", lhs.text),
                    lhs.span,
                ));
                return None;
            };
            Some(Predicate::Equals(
                IrBinding {
                    name: lhs.text.clone(),
                    value_type: lhs_type,
                },
                Value::String(rhs.clone()),
            ))
        }
        (gql_ast::Expression::String(lhs, _), gql_ast::Expression::Name(rhs)) => {
            let Some(rhs_type) = bindings.get(&rhs.text).cloned() else {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-WHERE-UNRESOLVED-BINDING",
                    format!("WHERE references unknown binding `{}`", rhs.text),
                    rhs.span,
                ));
                return None;
            };
            Some(Predicate::Equals(
                IrBinding {
                    name: rhs.text.clone(),
                    value_type: rhs_type,
                },
                Value::String(lhs.clone()),
            ))
        }
        (gql_ast::Expression::Name(lhs), gql_ast::Expression::Name(rhs)) => {
            let lhs_bound = bindings.contains_key(&lhs.text);
            let rhs_bound = bindings.contains_key(&rhs.text);
            if !lhs_bound {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-WHERE-UNRESOLVED-BINDING",
                    format!("WHERE references unknown binding `{}`", lhs.text),
                    lhs.span,
                ));
            }
            if !rhs_bound {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-WHERE-UNRESOLVED-BINDING",
                    format!("WHERE references unknown binding `{}`", rhs.text),
                    rhs.span,
                ));
            }
            if lhs_bound && rhs_bound {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-WHERE-UNSUPPORTED-EXPRESSION",
                    "WHERE only supports simple identifier/literal equality in this release",
                    lhs.span,
                ));
            }
            None
        }
        _ => {
            let span = first_span_in_expression_pair(lhs, rhs);
            diagnostics.push(Diagnostic::error(
                "GQL-SEMA-WHERE-UNSUPPORTED-EXPRESSION",
                "WHERE only supports simple identifier/literal equality in this release",
                span,
            ));
            None
        }
    }
}

fn first_span_in_expression_pair(
    lhs: &gql_ast::Expression,
    rhs: &gql_ast::Expression,
) -> Span {
    match (lhs, rhs) {
        (gql_ast::Expression::Name(name), _) => name.span,
        (gql_ast::Expression::String(_, span), _) => *span,
        (gql_ast::Expression::Integer(_, span), _) => *span,
        (gql_ast::Expression::Unary { operand, .. }, _) => expression_span(operand),
        (gql_ast::Expression::Binary { left, .. }, _) => expression_span(left),
    }
}

fn let_clause_value_type(
    clause: &gql_ast::QueryClause,
    bindings: &HashMap<String, ValueType>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ValueType> {
    match clause {
        gql_ast::QueryClause::Let { value, .. } => match value {
            Expression::Name(name) => {
                if let Some(ty) = bindings.get(&name.text).cloned() {
                    Some(ty)
                } else {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-LET-VALUE-UNRESOLVED-BINDING",
                        format!("LET value references unknown binding `{}`", name.text),
                        name.span,
                    ));
                    None
                }
            }
            Expression::Integer(_, _) => Some(ValueType::Integer),
            Expression::String(_, _) => Some(ValueType::String),
            Expression::Binary { .. } | Expression::Unary { .. } => {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-LET-UNSUPPORTED-EXPRESSION",
                    "LET value cannot be a binary expression in this release",
                    Span::default(),
                ));
                None
            }
        },
        _ => None,
    }
}

fn expression_span(expression: &Expression) -> Span {
    match expression {
        Expression::Name(identifier) => identifier.span,
        Expression::String(_, span) => *span,
        Expression::Integer(_, span) => *span,
        Expression::Unary { operand, .. } => expression_span(operand),
        Expression::Binary { left, .. } => expression_span(left),
    }
}
