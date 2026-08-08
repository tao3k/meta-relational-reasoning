//! Backend-neutral semantic analysis for the supported ISO GQL query slice.

#![forbid(unsafe_code)]

use std::collections::HashMap;

use gql_ast::{BinaryOperator, Expression, PatternElement, QueryClause, Statement, UnaryOperator};
use gql_catalog::GqlCatalog;
use gql_ir::{
    BinaryOperator as IrBinaryOperator, Binding as IrBinding, EdgeDirection as IrEdgeDirection,
    EdgePattern as IrEdgePattern, Expression as IrExpression, GraphPattern as IrGraphPattern,
    GraphPatternElement as IrGraphPatternElement, LetBinding, NodePattern as IrNodePattern,
    PathPattern as IrPathPattern, Projection, QueryBlock, UnaryOperator as IrUnaryOperator,
};
use gql_source::{Diagnostic, Span};
use gql_types::ValueType;

/// Owned semantic analysis result for a parsed statement.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Analysis {
    /// Canonical graph-semantic IR when no semantic diagnostics exist.
    pub ir: Option<QueryBlock>,
    /// Semantic diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

/// Analyze a lowered statement against an ISO catalog context.
#[must_use]
pub fn analyze(statement: &Statement, _catalog: &dyn GqlCatalog) -> Analysis {
    let Statement::Query(query) = statement else {
        return Analysis {
            ir: None,
            diagnostics: vec![Diagnostic::error(
                "GQL-SEMA-NOT-YET-LOWERED",
                "catalog and data statements are not lowered by this foundation release",
                Span::default(),
            )],
        };
    };

    let mut block = QueryBlock::default();
    let mut diagnostics = Vec::new();
    let mut bindings = HashMap::<String, ValueType>::new();

    for clause in &query.clauses {
        match clause {
            QueryClause::Match(match_clause) => {
                if block.graph.is_some() {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-MULTIPLE-MATCH-BLOCKS",
                        "multiple MATCH clauses are not yet represented in one query block",
                        match_clause.span,
                    ));
                    continue;
                }
                let pattern_bindings = collect_pattern_bindings(&match_clause.pattern);
                for binding in pattern_bindings {
                    if bindings
                        .insert(binding.text.clone(), ValueType::Node)
                        .is_some()
                    {
                        diagnostics.push(Diagnostic::error(
                            "GQL-SEMA-DUPLICATE-BINDING",
                            format!("binding `{}` is declared more than once", binding.text),
                            binding.span,
                        ));
                    }
                }
                block.graph = Some(build_graph_pattern(&match_clause.pattern));
            }
            QueryClause::Where { expression } => {
                if let Some(filter) = lower_expression(expression, &bindings, &mut diagnostics) {
                    block.filters.push(filter);
                }
            }
            QueryClause::Let { binding, value } => {
                if binding.text.is_empty() {
                    continue;
                }
                if bindings.contains_key(&binding.text) {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-LET-DUPLICATE-BINDING",
                        format!("LET binding `{}` is already defined", binding.text),
                        binding.span,
                    ));
                    continue;
                }
                let Some(ir_value) = lower_expression(value, &bindings, &mut diagnostics) else {
                    continue;
                };
                let value_type = expression_type(value, &bindings).unwrap_or(ValueType::Any);
                bindings.insert(binding.text.clone(), value_type.clone());
                block.let_bindings.push(LetBinding {
                    binding: IrBinding {
                        name: binding.text.clone(),
                        value_type,
                    },
                    value: ir_value,
                });
            }
            QueryClause::Return { expressions } => {
                for expression in expressions {
                    if let Some(ir_expression) =
                        lower_expression(expression, &bindings, &mut diagnostics)
                    {
                        block.projection.push(Projection {
                            expression: ir_expression,
                        });
                    }
                }
            }
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
        PatternElement::Node(node) => IrGraphPatternElement::Node(IrNodePattern {
            binding: node.binding.as_ref().map(|binding| binding.text.clone()),
            labels: node.labels.iter().map(|label| label.text.clone()).collect(),
        }),
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

fn collect_pattern_bindings(pattern: &gql_ast::GraphPattern) -> Vec<gql_ast::Identifier> {
    let mut bindings = Vec::new();
    collect_pattern_bindings_inner(&pattern.elements, &mut bindings);
    bindings
}

fn collect_pattern_bindings_inner(
    elements: &[PatternElement],
    bindings: &mut Vec<gql_ast::Identifier>,
) {
    for element in elements {
        match element {
            PatternElement::Node(node) => {
                if let Some(binding) = &node.binding {
                    bindings.push(binding.clone());
                }
            }
            PatternElement::Path(path) => collect_pattern_bindings_inner(&path.elements, bindings),
            PatternElement::Edge(_) => {}
        }
    }
}

fn lower_expression(
    expression: &Expression,
    bindings: &HashMap<String, ValueType>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<IrExpression> {
    match expression {
        Expression::Name(identifier) => {
            if bindings.contains_key(&identifier.text) {
                Some(IrExpression::Binding(identifier.text.clone()))
            } else {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-UNRESOLVED-BINDING",
                    format!(
                        "expression references unknown binding `{}`",
                        identifier.text
                    ),
                    identifier.span,
                ));
                None
            }
        }
        Expression::String(value, _) => Some(IrExpression::String(value.clone())),
        Expression::Integer(value, _) => Some(IrExpression::Integer(*value)),
        Expression::Unary { operator, operand } => Some(IrExpression::Unary {
            operator: match operator {
                UnaryOperator::Not => IrUnaryOperator::Not,
            },
            operand: Box::new(lower_expression(operand, bindings, diagnostics)?),
        }),
        Expression::Binary {
            operator,
            left,
            right,
        } => Some(IrExpression::Binary {
            operator: match operator {
                BinaryOperator::Equals => IrBinaryOperator::Equals,
                BinaryOperator::NotEquals => IrBinaryOperator::NotEquals,
                BinaryOperator::LessThan => IrBinaryOperator::LessThan,
                BinaryOperator::LessThanOrEqual => IrBinaryOperator::LessThanOrEqual,
                BinaryOperator::GreaterThan => IrBinaryOperator::GreaterThan,
                BinaryOperator::GreaterThanOrEqual => IrBinaryOperator::GreaterThanOrEqual,
                BinaryOperator::And => IrBinaryOperator::And,
                BinaryOperator::Or => IrBinaryOperator::Or,
            },
            left: Box::new(lower_expression(left, bindings, diagnostics)?),
            right: Box::new(lower_expression(right, bindings, diagnostics)?),
        }),
    }
}

fn expression_type(
    expression: &Expression,
    bindings: &HashMap<String, ValueType>,
) -> Option<ValueType> {
    match expression {
        Expression::Name(identifier) => bindings.get(&identifier.text).cloned(),
        Expression::String(_, _) => Some(ValueType::String),
        Expression::Integer(_, _) => Some(ValueType::Integer),
        Expression::Unary { .. } | Expression::Binary { .. } => Some(ValueType::Boolean),
    }
}
