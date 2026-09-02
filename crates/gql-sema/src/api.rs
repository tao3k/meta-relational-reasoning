//! Backend-neutral semantic analysis for the supported ISO GQL query slice.

#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};

use gql_ast::{
    BinaryOperator, Expression, LabelExpression, PatternElement, QueryClause, Statement,
    UnaryOperator,
};
use gql_catalog::GqlCatalog;
use gql_ir::{
    AggregateFunction as IrAggregateFunction, BinaryOperator as IrBinaryOperator,
    Binding as IrBinding, CaseBranch as IrCaseBranch, CatalogCommand,
    EdgeDirection as IrEdgeDirection, EdgePattern as IrEdgePattern, Expression as IrExpression,
    GraphPattern as IrGraphPattern, GraphPatternElement as IrGraphPatternElement,
    LabelExpression as IrLabelExpression, LetBinding, NodePattern as IrNodePattern,
    OptionalMatch as IrOptionalMatch, PathMode as IrPathMode, PathPattern as IrPathPattern,
    PathQuantifier as IrPathQuantifier, ProcedureCommand, Projection, QueryBlock,
    SessionCommand as IrSessionCommand, SetOperation as IrSetOperation,
    SetOperator as IrSetOperator, SortDirection as IrSortDirection, SortKey as IrSortKey,
    TransactionCommand as IrTransactionCommand, UnaryOperator as IrUnaryOperator,
};
use gql_source::{Diagnostic, Span};
use gql_types::ValueType;

use crate::aggregate_analysis::contains_aggregate;
use crate::data_management::{analyze_data_clause, analyze_non_query_statement};
use crate::record_lowering::lower_record_expression;
use crate::type_inference::{case_types_compatible, expression_type, is_numeric_type};

/// Owned semantic analysis result for a parsed statement.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Analysis {
    /// Canonical graph-semantic IR when no semantic diagnostics exist.
    pub ir: Option<QueryBlock>,
    /// Canonical catalog command when a catalog statement is admitted.
    pub catalog_command: Option<CatalogCommand>,
    /// Canonical procedure invocation when a CALL statement is admitted.
    pub procedure_command: Option<ProcedureCommand>,
    /// Canonical transaction-control intent when admitted.
    pub transaction_command: Option<IrTransactionCommand>,
    /// Canonical session-control intent when admitted.
    pub session_command: Option<IrSessionCommand>,
    /// Semantic diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

/// Analyze a lowered statement against an ISO catalog context.
#[must_use]
pub fn analyze(statement: &Statement, catalog: &dyn GqlCatalog) -> Analysis {
    if let Some(analysis) = analyze_non_query_statement(statement, catalog) {
        return analysis;
    }
    let Statement::Query(query) = statement else {
        unreachable!("non-query statements return from typed dispatch")
    };

    if query.clauses.is_empty() {
        return Analysis {
            ir: None,
            catalog_command: None,
            procedure_command: None,
            transaction_command: None,
            session_command: None,
            diagnostics: vec![Diagnostic::error(
                "GQL-SEMA-EMPTY-QUERY",
                "query contains no clauses",
                query.span,
            )],
        };
    }

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
        if !branch
            .iter()
            .any(|clause| matches!(clause, QueryClause::Return { .. }))
            && !branch.iter().any(|clause| {
                matches!(
                    clause,
                    QueryClause::Insert { .. }
                        | QueryClause::Set { .. }
                        | QueryClause::Remove { .. }
                        | QueryClause::Delete { .. }
                )
            })
        {
            diagnostics.push(Diagnostic::error(
                "GQL-SEMA-QUERY-BRANCH-MISSING-RETURN",
                "every query branch requires a RETURN projection",
                query.span,
            ));
        }
        blocks.push(analyze_query_block(&branch, &mut diagnostics));
    }

    if let Some(first) = blocks.first()
        && !first.projection.is_empty()
    {
        for branch in blocks.iter().skip(1) {
            if !branch.projection.is_empty() && branch.projection.len() != first.projection.len() {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-UNION-PROJECTION-ARITY",
                    "UNION query blocks must project the same number of columns",
                    query.span,
                ));
                continue;
            }
            for (left, right) in first.projection.iter().zip(&branch.projection) {
                if left.value_type != right.value_type
                    && !matches!(&left.value_type, ValueType::Any | ValueType::Null)
                    && !matches!(&right.value_type, ValueType::Any | ValueType::Null)
                {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-UNION-PROJECTION-TYPE",
                        "UNION output columns must have compatible value types",
                        query.span,
                    ));
                    break;
                }
                if left.alias != right.alias {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-UNION-PROJECTION-NAME",
                        "UNION output columns must have the same canonical alias",
                        query.span,
                    ));
                    break;
                }
            }
        }
    }

    let mut blocks = blocks.into_iter();
    let Some(mut block) = blocks.next() else {
        return Analysis {
            ir: None,
            catalog_command: None,
            procedure_command: None,
            transaction_command: None,
            session_command: None,
            diagnostics,
        };
    };
    block
        .set_operations
        .extend(blocks.map(|right| IrSetOperation {
            operator: IrSetOperator::UnionDistinct,
            right: Box::new(right),
        }));

    Analysis {
        ir: diagnostics.is_empty().then_some(block),
        catalog_command: None,
        procedure_command: None,
        transaction_command: None,
        session_command: None,
        diagnostics,
    }
}

fn analyze_query_block(clauses: &[&QueryClause], diagnostics: &mut Vec<Diagnostic>) -> QueryBlock {
    let mut block = QueryBlock::default();
    let mut bindings = HashMap::<String, ValueType>::new();
    let mut pending_optional_match = None;
    let mut seen_return = false;
    let mut clause_after_return_emitted = false;

    for clause in clauses {
        if seen_return
            && !matches!(
                clause,
                QueryClause::OrderBy { .. }
                    | QueryClause::Offset { .. }
                    | QueryClause::Limit { .. }
                    | QueryClause::GroupBy { .. }
            )
        {
            if !clause_after_return_emitted {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-CLAUSE-AFTER-RETURN",
                    "graph and binding clauses cannot follow RETURN in the same query branch",
                    query_clause_span(clause),
                ));
                clause_after_return_emitted = true;
            }
            continue;
        }
        if !matches!(clause, QueryClause::Where { .. }) {
            pending_optional_match = None;
        }
        if analyze_data_clause(clause, &mut block, &mut bindings, diagnostics) {
            continue;
        }
        match clause {
            QueryClause::Match(match_clause) => {
                for pattern in &match_clause.patterns {
                    register_pattern_bindings(pattern, &mut bindings, diagnostics);
                }
                block
                    .graphs
                    .extend(match_clause.patterns.iter().map(|pattern| {
                        build_graph_pattern(pattern, match_clause.mode, &bindings, diagnostics)
                    }));
            }
            QueryClause::OptionalMatch(match_clause) => {
                for pattern in &match_clause.patterns {
                    register_pattern_bindings(pattern, &mut bindings, diagnostics);
                }
                let graphs = match_clause
                    .patterns
                    .iter()
                    .map(|pattern| {
                        build_graph_pattern(pattern, match_clause.mode, &bindings, diagnostics)
                    })
                    .collect();
                block.optional_matches.push(IrOptionalMatch {
                    graphs,
                    predicate: None,
                });
                pending_optional_match = Some(block.optional_matches.len() - 1);
            }
            QueryClause::Where { expression, span } => {
                if block.graphs.is_empty() && block.optional_matches.is_empty() {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-WHERE-WITHOUT-PATTERN-SCOPE",
                        "WHERE requires a preceding graph pattern scope",
                        *span,
                    ));
                    pending_optional_match = None;
                    continue;
                }
                if expression_type(expression, &bindings).is_some_and(|value_type| {
                    !matches!(value_type, ValueType::Boolean | ValueType::Any)
                }) {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-WHERE-NOT-BOOLEAN",
                        "WHERE expression must have Boolean type",
                        *span,
                    ));
                    pending_optional_match = None;
                    continue;
                }
                if let Some(filter) = lower_expression(expression, &bindings, diagnostics) {
                    if let Some(index) = pending_optional_match.take() {
                        block.optional_matches[index].predicate = Some(filter);
                    } else {
                        block.filters.push(filter);
                    }
                }
            }
            QueryClause::Let {
                bindings: found, ..
            } => {
                for found in found {
                    let binding = &found.binding;
                    let value = &found.value;
                    if binding.text.is_empty() {
                        continue;
                    }
                    let canonical_binding = binding.canonical_text();
                    if bindings.contains_key(&canonical_binding) {
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
                    bindings.insert(canonical_binding.clone(), value_type.clone());
                    block.let_bindings.push(LetBinding {
                        binding: IrBinding {
                            name: canonical_binding,
                            value_type,
                        },
                        value: ir_value,
                    });
                }
            }
            QueryClause::Return { projections, .. } => {
                seen_return = true;
                let mut aliases = HashSet::new();
                let mut result_bindings = Vec::new();
                for projection in projections {
                    if let Some(alias) = &projection.alias
                        && !aliases.insert(alias.canonical_text())
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
                        let value_type = expression_type(&projection.expression, &bindings)
                            .unwrap_or(ValueType::Any);
                        let alias = projection
                            .alias
                            .as_ref()
                            .map(gql_ast::Identifier::canonical_text);
                        block.projection.push(Projection {
                            expression,
                            alias: alias.clone(),
                            value_type: value_type.clone(),
                        });
                        if let Some(alias) = alias {
                            result_bindings.push((alias, value_type));
                        }
                    }
                }
                bindings.extend(result_bindings);
            }
            QueryClause::Insert { .. }
            | QueryClause::Set { .. }
            | QueryClause::Remove { .. }
            | QueryClause::Delete { .. } => {
                unreachable!("data clauses are handled by the data-management owner")
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
            QueryClause::GroupBy { keys, span } => {
                if keys.is_empty() {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-GROUP-BY-MISSING-EXPRESSION",
                        "GROUP BY requires at least one expression",
                        *span,
                    ));
                }
                for key in keys {
                    if let Some(expression) = lower_expression(key, &bindings, diagnostics) {
                        block.group_by.push(expression);
                    }
                }
            }
        }
    }
    if block
        .projection
        .iter()
        .any(|projection| contains_aggregate(&projection.expression))
    {
        for projection in &block.projection {
            if !contains_aggregate(&projection.expression)
                && !block.group_by.contains(&projection.expression)
            {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-NON-GROUPED-PROJECTION",
                    "non-aggregate RETURN expressions must appear in GROUP BY",
                    Span::default(),
                ));
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

fn query_clause_span(clause: &QueryClause) -> Span {
    match clause {
        QueryClause::Match(found) | QueryClause::OptionalMatch(found) => found.span,
        QueryClause::Where { span, .. }
        | QueryClause::Let { span, .. }
        | QueryClause::Return { span, .. }
        | QueryClause::Union { span }
        | QueryClause::Limit { span, .. }
        | QueryClause::OrderBy { span, .. }
        | QueryClause::Offset { span, .. }
        | QueryClause::GroupBy { span, .. }
        | QueryClause::Insert { span, .. }
        | QueryClause::Set { span, .. }
        | QueryClause::Remove { span, .. }
        | QueryClause::Delete { span, .. } => *span,
    }
}

pub(crate) fn build_graph_pattern(
    pattern: &gql_ast::GraphPattern,
    mode: gql_ast::PathMode,
    bindings: &HashMap<String, ValueType>,
    diagnostics: &mut Vec<Diagnostic>,
) -> IrGraphPattern {
    IrGraphPattern {
        mode: match mode {
            gql_ast::PathMode::Walk => IrPathMode::Walk,
            gql_ast::PathMode::Trail => IrPathMode::Trail,
            gql_ast::PathMode::Acyclic => IrPathMode::Acyclic,
            gql_ast::PathMode::Simple => IrPathMode::Simple,
        },
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
            binding: node
                .binding
                .as_ref()
                .map(gql_ast::Identifier::canonical_text),
            labels: node
                .labels
                .iter()
                .map(gql_ast::Identifier::canonical_text)
                .collect(),
            properties: lower_property_constraints(&node.properties, bindings, diagnostics),
            predicate: node
                .predicate
                .as_ref()
                .and_then(|predicate| lower_expression(predicate, bindings, diagnostics)),
        }),
        PatternElement::Edge(edge) => IrGraphPatternElement::Edge(IrEdgePattern {
            binding: edge
                .binding
                .as_ref()
                .map(gql_ast::Identifier::canonical_text),
            labels: edge
                .labels
                .iter()
                .map(gql_ast::Identifier::canonical_text)
                .collect(),
            properties: lower_property_constraints(&edge.properties, bindings, diagnostics),
            predicate: edge
                .predicate
                .as_ref()
                .and_then(|predicate| lower_expression(predicate, bindings, diagnostics)),
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
            binding: path
                .binding
                .as_ref()
                .map(gql_ast::Identifier::canonical_text),
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

pub(crate) fn register_pattern_bindings(
    pattern: &gql_ast::GraphPattern,
    bindings: &mut HashMap<String, ValueType>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (binding, value_type) in collect_pattern_bindings(pattern) {
        let canonical = binding.canonical_text();
        if let Some(existing) = bindings.get(&canonical) {
            if existing != &value_type {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-BINDING-KIND-CONFLICT",
                    format!(
                        "binding `{}` cannot denote both {existing:?} and {value_type:?}",
                        binding.text
                    ),
                    binding.span,
                ));
            }
        } else {
            bindings.insert(canonical, value_type);
        }
    }
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
            let canonical_key = property.key.canonical_text();
            if !property_names.insert(canonical_key.clone()) {
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
                key: canonical_key,
                value: lower_expression(&property.value, bindings, diagnostics)?,
            })
        })
        .collect()
}

pub(crate) fn lower_expression(
    expression: &Expression,
    bindings: &HashMap<String, ValueType>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<IrExpression> {
    match expression {
        Expression::Name(identifier) => {
            let canonical_identifier = identifier.canonical_text();
            if bindings.contains_key(&canonical_identifier) {
                Some(IrExpression::Binding(canonical_identifier))
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
        Expression::String(literal) => Some(IrExpression::String(literal.value.clone())),
        Expression::ByteString(value, _) => Some(IrExpression::ByteString(value.clone())),
        Expression::Date(value, _) => Some(IrExpression::Date(value.clone())),
        Expression::Time(value, _) => Some(IrExpression::Time(value.clone())),
        Expression::Timestamp(value, _) => Some(IrExpression::Timestamp(value.clone())),
        Expression::Duration(value, _) => Some(IrExpression::Duration(value.clone())),
        Expression::Integer(value, _) => Some(IrExpression::Integer(*value)),
        Expression::Decimal(value, _) => Some(IrExpression::Decimal(value.clone())),
        Expression::ApproximateNumeric(value, _) => {
            Some(IrExpression::ApproximateNumeric(value.clone()))
        }
        Expression::List(items, _) => Some(IrExpression::List(
            items
                .iter()
                .map(|item| lower_expression(item, bindings, diagnostics))
                .collect::<Option<Vec<_>>>()?,
        )),
        Expression::Record(fields, _) => lower_record_expression(fields, bindings, diagnostics),
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
            property: property.canonical_text(),
        }),
        Expression::FunctionCall {
            name,
            arguments,
            span,
        } => {
            if name.canonical_text() != "COUNT" {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-UNSUPPORTED-FUNCTION",
                    format!(
                        "function `{}` is not admitted by this query profile",
                        name.text
                    ),
                    *span,
                ));
                return None;
            }
            if arguments.len() != 1 {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-AGGREGATE-ARITY",
                    "COUNT requires exactly one argument",
                    *span,
                ));
                return None;
            }
            let lowered = arguments
                .iter()
                .filter_map(|argument| lower_expression(argument, bindings, diagnostics))
                .collect::<Vec<_>>();
            (lowered.len() == arguments.len()).then_some(IrExpression::Aggregate {
                function: IrAggregateFunction::Count,
                arguments: lowered,
            })
        }
        Expression::Unary { operator, operand } => {
            let operand_ir = lower_expression(operand, bindings, diagnostics)?;
            let operand_type = expression_type(operand, bindings).unwrap_or(ValueType::Any);
            match operator {
                UnaryOperator::Not if operand_type != ValueType::Boolean => {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-NON-BOOLEAN-LOGIC",
                        "NOT requires a boolean operand",
                        Span::default(),
                    ));
                    return None;
                }
                UnaryOperator::Plus | UnaryOperator::Negate if !is_numeric_type(&operand_type) => {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-NON-NUMERIC-ARITHMETIC",
                        "unary sign requires a numeric operand",
                        Span::default(),
                    ));
                    return None;
                }
                _ => {}
            }
            Some(IrExpression::Unary {
                operator: match operator {
                    UnaryOperator::Not => IrUnaryOperator::Not,
                    UnaryOperator::Plus => IrUnaryOperator::Plus,
                    UnaryOperator::Negate => IrUnaryOperator::Negate,
                },
                operand: Box::new(operand_ir),
            })
        }
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
            if matches!(
                operator,
                BinaryOperator::And | BinaryOperator::Xor | BinaryOperator::Or
            ) {
                let left_type = expression_type(left, bindings).unwrap_or(ValueType::Any);
                let right_type = expression_type(right, bindings).unwrap_or(ValueType::Any);
                if left_type != ValueType::Boolean || right_type != ValueType::Boolean {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-NON-BOOLEAN-LOGIC",
                        "boolean operators require boolean operands",
                        Span::default(),
                    ));
                    return None;
                }
            }
            if matches!(operator, BinaryOperator::Concatenate) {
                let left_type = expression_type(left, bindings).unwrap_or(ValueType::Any);
                let right_type = expression_type(right, bindings).unwrap_or(ValueType::Any);
                if left_type != ValueType::String || right_type != ValueType::String {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-NON-STRING-CONCATENATION",
                        "string concatenation requires string operands",
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
                    BinaryOperator::Concatenate => IrBinaryOperator::Concatenate,
                    BinaryOperator::In => IrBinaryOperator::In,
                    BinaryOperator::Equals => IrBinaryOperator::Equals,
                    BinaryOperator::NotEquals => IrBinaryOperator::NotEquals,
                    BinaryOperator::LessThan => IrBinaryOperator::LessThan,
                    BinaryOperator::LessThanOrEqual => IrBinaryOperator::LessThanOrEqual,
                    BinaryOperator::GreaterThan => IrBinaryOperator::GreaterThan,
                    BinaryOperator::GreaterThanOrEqual => IrBinaryOperator::GreaterThanOrEqual,
                    BinaryOperator::And => IrBinaryOperator::And,
                    BinaryOperator::Xor => IrBinaryOperator::Xor,
                    BinaryOperator::Or => IrBinaryOperator::Or,
                },
                left: Box::new(left_ir),
                right: Box::new(right_ir),
            })
        }
        Expression::IsLabeled {
            operand,
            label,
            negated,
            span,
        } => {
            let operand_ir = lower_expression(operand, bindings, diagnostics)?;
            let operand_type = expression_type(operand, bindings).unwrap_or(ValueType::Any);
            if !matches!(operand_type, ValueType::Node | ValueType::Edge) {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-LABEL-PREDICATE-NON-ELEMENT",
                    "IS LABELED requires a node or edge expression",
                    *span,
                ));
                return None;
            }
            Some(IrExpression::IsLabeled {
                operand: Box::new(operand_ir),
                label: lower_label_expression(label),
                negated: *negated,
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

fn lower_label_expression(expression: &LabelExpression) -> IrLabelExpression {
    match expression {
        LabelExpression::Name(name) => IrLabelExpression::Name(name.canonical_text()),
        LabelExpression::Wildcard => IrLabelExpression::Wildcard,
        LabelExpression::Not(operand) => {
            IrLabelExpression::Not(Box::new(lower_label_expression(operand)))
        }
        LabelExpression::And(left, right) => IrLabelExpression::And(
            Box::new(lower_label_expression(left)),
            Box::new(lower_label_expression(right)),
        ),
        LabelExpression::Or(left, right) => IrLabelExpression::Or(
            Box::new(lower_label_expression(left)),
            Box::new(lower_label_expression(right)),
        ),
    }
}
