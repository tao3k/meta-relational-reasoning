//! Backend-neutral semantic analysis for the supported ISO GQL query slice.

#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};

use gql_ast::{BinaryOperator, Expression, PatternElement, QueryClause, Statement, UnaryOperator};
use gql_catalog::GqlCatalog;
use gql_ir::{
    BinaryOperator as IrBinaryOperator, Binding as IrBinding, CaseBranch as IrCaseBranch,
    EdgeDirection as IrEdgeDirection, EdgePattern as IrEdgePattern, Expression as IrExpression,
    GraphPattern as IrGraphPattern, GraphPatternElement as IrGraphPatternElement, LetBinding,
    NodePattern as IrNodePattern, PathPattern as IrPathPattern, PathQuantifier as IrPathQuantifier,
    Projection, QueryBlock, SortDirection as IrSortDirection, SortKey as IrSortKey,
    UnaryOperator as IrUnaryOperator,
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

    let mut diagnostics = Vec::new();
    let mut branches = vec![Vec::new()];
    for clause in &query.clauses {
        if matches!(clause, QueryClause::Union { .. }) {
            branches.push(Vec::new());
        } else {
            branches
                .last_mut()
                .expect("branch is initialized")
                .push(clause);
        }
    }

    let mut blocks = Vec::new();
    for branch in branches {
        if branch.is_empty() {
            diagnostics.push(Diagnostic::error(
                "GQL-SEMA-UNION-MISSING-BRANCH",
                "UNION requires a query block on both sides",
                query.span,
            ));
            continue;
        }
        blocks.push(analyze_query_block(&branch, &mut diagnostics));
    }

    if let Some(first) = blocks.first() {
        for branch in blocks.iter().skip(1) {
            if branch.projection.len() != first.projection.len() {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-UNION-PROJECTION-ARITY",
                    "UNION query blocks must project the same number of columns",
                    query.span,
                ));
            }
        }
    }

    let mut blocks = blocks.into_iter();
    let mut block = blocks.next().unwrap_or_default();
    block.union_branches.extend(blocks);

    Analysis {
        ir: diagnostics.is_empty().then_some(block),
        diagnostics,
    }
}

fn analyze_query_block(clauses: &[&QueryClause], diagnostics: &mut Vec<Diagnostic>) -> QueryBlock {
    let mut block = QueryBlock::default();
    let mut bindings = HashMap::<String, ValueType>::new();

    for clause in clauses {
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
                for (binding, value_type) in pattern_bindings {
                    if bindings.insert(binding.text.clone(), value_type).is_some() {
                        diagnostics.push(Diagnostic::error(
                            "GQL-SEMA-DUPLICATE-BINDING",
                            format!("binding `{}` is declared more than once", binding.text),
                            binding.span,
                        ));
                    }
                }
                block.graph = Some(build_graph_pattern(
                    &match_clause.pattern,
                    &bindings,
                    diagnostics,
                ));
            }
            QueryClause::OptionalMatch(match_clause) => {
                if block.graph.is_none() {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-OPTIONAL-MATCH-WITHOUT-MANDATORY",
                        "OPTIONAL MATCH requires a preceding mandatory MATCH",
                        match_clause.span,
                    ));
                    continue;
                }
                for (binding, value_type) in collect_pattern_bindings(&match_clause.pattern) {
                    if bindings.contains_key(&binding.text) {
                        continue;
                    }
                    bindings.insert(binding.text.clone(), value_type);
                }
                block.optional_graphs.push(build_graph_pattern(
                    &match_clause.pattern,
                    &bindings,
                    diagnostics,
                ));
            }
            QueryClause::Where { expression } => {
                if let Some(filter) = lower_expression(expression, &bindings, diagnostics) {
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
                let Some(ir_value) = lower_expression(value, &bindings, diagnostics) else {
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
                        lower_expression(expression, &bindings, diagnostics)
                    {
                        block.projection.push(Projection {
                            expression: ir_expression,
                            alias: None,
                        });
                    }
                }
            }
            QueryClause::ReturnAliased { projections } => {
                let mut aliases = HashSet::new();
                for projection in projections {
                    if let Some(alias) = &projection.alias
                        && !aliases.insert(alias.text.clone())
                    {
                        diagnostics.push(Diagnostic::error(
                            "GQL-SEMA-DUPLICATE-PROJECTION-ALIAS",
                            format!(
                                "projection alias `{}` is declared more than once",
                                alias.text
                            ),
                            alias.span,
                        ));
                        continue;
                    }
                    if let Some(expression) =
                        lower_expression(&projection.expression, &bindings, diagnostics)
                    {
                        block.projection.push(Projection {
                            expression,
                            alias: projection.alias.as_ref().map(|alias| alias.text.clone()),
                        });
                    }
                }
            }
            QueryClause::Union { .. } => unreachable!("UNION clauses delimit query blocks"),
            QueryClause::Limit { value, span } => {
                let Some(value) = value else {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-LIMIT-NON-INTEGER",
                        "LIMIT requires a positive integer literal",
                        *span,
                    ));
                    continue;
                };
                if *value == 0 {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-LIMIT-NONPOSITIVE",
                        "LIMIT must be greater than zero",
                        *span,
                    ));
                } else if block.limit.replace(*value).is_some() {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-DUPLICATE-LIMIT",
                        "a query block may contain at most one LIMIT clause",
                        *span,
                    ));
                }
            }
            QueryClause::OrderBy { keys, span } => {
                if keys.is_empty() {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-ORDER-BY-MISSING-EXPRESSION",
                        "ORDER BY requires at least one expression",
                        *span,
                    ));
                }
                for key in keys {
                    if let Some(expression) =
                        lower_expression(&key.expression, &bindings, diagnostics)
                    {
                        block.order_by.push(IrSortKey {
                            expression,
                            direction: match key.direction {
                                gql_ast::SortDirection::Ascending => IrSortDirection::Ascending,
                                gql_ast::SortDirection::Descending => IrSortDirection::Descending,
                            },
                        });
                    }
                }
            }
            QueryClause::Offset { value, span } => {
                let Some(value) = value else {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-OFFSET-NON-INTEGER",
                        "OFFSET requires a non-negative integer literal",
                        *span,
                    ));
                    continue;
                };
                if block.offset.replace(*value).is_some() {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-DUPLICATE-OFFSET",
                        "a query block may contain at most one OFFSET clause",
                        *span,
                    ));
                }
            }
        }
    }
    if block.offset.is_some() && block.limit.is_none() {
        diagnostics.push(Diagnostic::error(
            "GQL-SEMA-OFFSET-WITHOUT-LIMIT",
            "OFFSET requires LIMIT in this frontend profile",
            Span::default(),
        ));
    }
    block
}

fn build_graph_pattern(
    pattern: &gql_ast::GraphPattern,
    bindings: &HashMap<String, ValueType>,
    diagnostics: &mut Vec<Diagnostic>,
) -> IrGraphPattern {
    IrGraphPattern {
        elements: pattern
            .elements
            .iter()
            .map(|element| build_graph_pattern_element(element, bindings, diagnostics))
            .collect(),
    }
}

fn build_graph_pattern_element(
    element: &PatternElement,
    bindings: &HashMap<String, ValueType>,
    diagnostics: &mut Vec<Diagnostic>,
) -> IrGraphPatternElement {
    match element {
        PatternElement::Node(node) => IrGraphPatternElement::Node(IrNodePattern {
            binding: node.binding.as_ref().map(|binding| binding.text.clone()),
            labels: node.labels.iter().map(|label| label.text.clone()).collect(),
            properties: lower_property_constraints(&node.properties, bindings, diagnostics),
        }),
        PatternElement::Edge(edge) => IrGraphPatternElement::Edge(IrEdgePattern {
            binding: edge.binding.as_ref().map(|binding| binding.text.clone()),
            labels: edge.labels.iter().map(|label| label.text.clone()).collect(),
            properties: lower_property_constraints(&edge.properties, bindings, diagnostics),
            direction: match edge.direction {
                gql_ast::EdgeDirection::Out => IrEdgeDirection::Out,
                gql_ast::EdgeDirection::In => IrEdgeDirection::In,
                gql_ast::EdgeDirection::Undirected => IrEdgeDirection::Undirected,
            },
            quantifier: edge.quantifier.as_ref().map(|quantifier| IrPathQuantifier {
                min: quantifier.min,
                max: quantifier.max,
            }),
        }),
        PatternElement::Path(path) => IrGraphPatternElement::Path(IrPathPattern {
            binding: path.binding.as_ref().map(|binding| binding.text.clone()),
            elements: path
                .elements
                .iter()
                .map(|element| build_graph_pattern_element(element, bindings, diagnostics))
                .collect(),
        }),
    }
}

fn collect_pattern_bindings(
    pattern: &gql_ast::GraphPattern,
) -> Vec<(gql_ast::Identifier, ValueType)> {
    let mut bindings = Vec::new();
    collect_pattern_bindings_inner(&pattern.elements, &mut bindings);
    bindings
}

fn collect_pattern_bindings_inner(
    elements: &[PatternElement],
    bindings: &mut Vec<(gql_ast::Identifier, ValueType)>,
) {
    for element in elements {
        match element {
            PatternElement::Node(node) => {
                if let Some(binding) = &node.binding {
                    bindings.push((binding.clone(), ValueType::Node));
                }
            }
            PatternElement::Edge(edge) => {
                if let Some(binding) = &edge.binding {
                    bindings.push((binding.clone(), ValueType::Edge));
                }
            }
            PatternElement::Path(path) => {
                if let Some(binding) = &path.binding {
                    bindings.push((binding.clone(), ValueType::Path));
                }
                collect_pattern_bindings_inner(&path.elements, bindings);
            }
        }
    }
}

fn lower_property_constraints(
    properties: &[gql_ast::PropertyConstraint],
    bindings: &HashMap<String, ValueType>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<gql_ir::PropertyConstraint> {
    let mut property_names = HashSet::new();
    properties
        .iter()
        .filter_map(|property| {
            if !property_names.insert(property.key.text.clone()) {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-DUPLICATE-PATTERN-PROPERTY",
                    format!(
                        "property `{}` is constrained more than once",
                        property.key.text
                    ),
                    property.key.span,
                ));
                return None;
            }
            Some(gql_ir::PropertyConstraint {
                key: property.key.text.clone(),
                value: lower_expression(&property.value, bindings, diagnostics)?,
            })
        })
        .collect()
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
        Expression::Boolean(value, _) => Some(IrExpression::Boolean(*value)),
        Expression::Null(_) => Some(IrExpression::Null),
        Expression::String(value, _) => Some(IrExpression::String(value.clone())),
        Expression::Integer(value, _) => Some(IrExpression::Integer(*value)),
        Expression::Decimal(value, _) => Some(IrExpression::Decimal(value.clone())),
        Expression::List(items, _) => Some(IrExpression::List(
            items
                .iter()
                .map(|item| lower_expression(item, bindings, diagnostics))
                .collect::<Option<Vec<_>>>()?,
        )),
        Expression::Subscript { base, index } => {
            let base_ir = lower_expression(base, bindings, diagnostics)?;
            let index_ir = lower_expression(index, bindings, diagnostics)?;
            let base_type = expression_type(base, bindings).unwrap_or(ValueType::Any);
            let index_type = expression_type(index, bindings).unwrap_or(ValueType::Any);
            if !matches!(base_type, ValueType::List | ValueType::Any) {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-NON-LIST-SUBSCRIPT",
                    "collection subscript requires a list value",
                    Span::default(),
                ));
                return None;
            }
            if !matches!(index_type, ValueType::Integer | ValueType::Any) {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-NON-INTEGER-SUBSCRIPT",
                    "collection subscript requires an integer index",
                    Span::default(),
                ));
                return None;
            }
            Some(IrExpression::Subscript {
                base: Box::new(base_ir),
                index: Box::new(index_ir),
            })
        }
        Expression::PropertyAccess { base, property } => Some(IrExpression::PropertyAccess {
            base: Box::new(lower_expression(base, bindings, diagnostics)?),
            property: property.text.clone(),
        }),
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
        } => {
            let left_ir = lower_expression(left, bindings, diagnostics)?;
            let right_ir = lower_expression(right, bindings, diagnostics)?;
            if matches!(operator, BinaryOperator::In) {
                let right_type = expression_type(right, bindings).unwrap_or(ValueType::Any);
                if !matches!(right_type, ValueType::List | ValueType::Any) {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-NON-LIST-MEMBERSHIP",
                        "IN requires a list on the right-hand side",
                        Span::default(),
                    ));
                    return None;
                }
            }
            if matches!(
                operator,
                BinaryOperator::Add
                    | BinaryOperator::Subtract
                    | BinaryOperator::Multiply
                    | BinaryOperator::Divide
                    | BinaryOperator::Modulo
            ) {
                let left_type = expression_type(left, bindings).unwrap_or(ValueType::Any);
                let right_type = expression_type(right, bindings).unwrap_or(ValueType::Any);
                if !is_numeric_type(&left_type) || !is_numeric_type(&right_type) {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-NON-NUMERIC-ARITHMETIC",
                        "arithmetic operators require numeric operands",
                        Span::default(),
                    ));
                    return None;
                }
            }
            Some(IrExpression::Binary {
                operator: match operator {
                    BinaryOperator::Add => IrBinaryOperator::Add,
                    BinaryOperator::Subtract => IrBinaryOperator::Subtract,
                    BinaryOperator::Multiply => IrBinaryOperator::Multiply,
                    BinaryOperator::Divide => IrBinaryOperator::Divide,
                    BinaryOperator::Modulo => IrBinaryOperator::Modulo,
                    BinaryOperator::In => IrBinaryOperator::In,
                    BinaryOperator::Equals => IrBinaryOperator::Equals,
                    BinaryOperator::NotEquals => IrBinaryOperator::NotEquals,
                    BinaryOperator::LessThan => IrBinaryOperator::LessThan,
                    BinaryOperator::LessThanOrEqual => IrBinaryOperator::LessThanOrEqual,
                    BinaryOperator::GreaterThan => IrBinaryOperator::GreaterThan,
                    BinaryOperator::GreaterThanOrEqual => IrBinaryOperator::GreaterThanOrEqual,
                    BinaryOperator::And => IrBinaryOperator::And,
                    BinaryOperator::Or => IrBinaryOperator::Or,
                },
                left: Box::new(left_ir),
                right: Box::new(right_ir),
            })
        }
        Expression::Case {
            operand,
            branches,
            else_result,
            span: _,
        } => {
            let operand_ir = match operand.as_deref() {
                Some(operand) => Some(Box::new(lower_expression(operand, bindings, diagnostics)?)),
                None => None,
            };
            let operand_type = operand
                .as_deref()
                .and_then(|operand| expression_type(operand, bindings));
            let mut ir_branches = Vec::with_capacity(branches.len());
            for branch in branches {
                let condition_type =
                    expression_type(&branch.condition, bindings).unwrap_or(ValueType::Any);
                if operand.is_none()
                    && !matches!(condition_type, ValueType::Boolean | ValueType::Any)
                {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-CASE-CONDITION-NOT-BOOLEAN",
                        "searched CASE requires a boolean WHEN condition",
                        branch.span,
                    ));
                    continue;
                }
                if let Some(operand_type) = &operand_type
                    && !case_types_compatible(operand_type, &condition_type)
                {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-CASE-OPERAND-TYPE-MISMATCH",
                        "simple CASE operand and WHEN value are not comparable",
                        branch.span,
                    ));
                    continue;
                }
                let condition = lower_expression(&branch.condition, bindings, diagnostics)?;
                let result = lower_expression(&branch.result, bindings, diagnostics)?;
                ir_branches.push(IrCaseBranch { condition, result });
            }
            let else_ir = match else_result.as_deref() {
                Some(result) => Some(Box::new(lower_expression(result, bindings, diagnostics)?)),
                None => None,
            };
            if ir_branches.len() != branches.len() {
                return None;
            }
            Some(IrExpression::Case {
                operand: operand_ir,
                branches: ir_branches,
                else_result: else_ir,
            })
        }
    }
}

fn expression_type(
    expression: &Expression,
    bindings: &HashMap<String, ValueType>,
) -> Option<ValueType> {
    match expression {
        Expression::Name(identifier) => bindings.get(&identifier.text).cloned(),
        Expression::Boolean(_, _) => Some(ValueType::Boolean),
        Expression::Null(_) => Some(ValueType::Null),
        Expression::String(_, _) => Some(ValueType::String),
        Expression::Integer(_, _) => Some(ValueType::Integer),
        Expression::Decimal(_, _) => Some(ValueType::Decimal),
        Expression::List(_, _) => Some(ValueType::List),
        Expression::Subscript { .. } => Some(ValueType::Any),
        Expression::PropertyAccess { .. } => Some(ValueType::Any),
        Expression::Unary { .. } => Some(ValueType::Boolean),
        Expression::Binary {
            operator,
            left,
            right,
        } => match operator {
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Modulo => {
                let left_type = expression_type(left, bindings).unwrap_or(ValueType::Any);
                let right_type = expression_type(right, bindings).unwrap_or(ValueType::Any);
                if left_type == ValueType::Decimal || right_type == ValueType::Decimal {
                    Some(ValueType::Decimal)
                } else if is_numeric_type(&left_type) && is_numeric_type(&right_type) {
                    Some(ValueType::Integer)
                } else {
                    Some(ValueType::Any)
                }
            }
            _ => Some(ValueType::Boolean),
        },
        Expression::Case {
            branches,
            else_result,
            ..
        } => {
            let mut result_types = branches
                .iter()
                .filter_map(|branch| expression_type(&branch.result, bindings))
                .chain(
                    else_result
                        .as_deref()
                        .and_then(|result| expression_type(result, bindings)),
                );
            let first = result_types.next()?;
            if result_types.all(|value_type| value_type == first) {
                Some(first)
            } else {
                Some(ValueType::Any)
            }
        }
    }
}

fn case_types_compatible(left: &ValueType, right: &ValueType) -> bool {
    left == right
        || matches!(left, ValueType::Any | ValueType::Null)
        || matches!(right, ValueType::Any | ValueType::Null)
        || (is_numeric_type(left) && is_numeric_type(right))
}

fn is_numeric_type(value_type: &ValueType) -> bool {
    matches!(value_type, ValueType::Integer | ValueType::Decimal)
}
