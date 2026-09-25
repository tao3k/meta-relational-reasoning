// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Parser-owned GQL and Cypher semantic projection into MetaQueryIR.

use mrr_gerbil::ParserLanguage;
use mrr_query::{
    Aggregation, AggregationFunction, BinaryOperator, Binding, Direction, Expression, Filter,
    GraphPattern, Grouping, MetaQueryIr, NodePattern, Ordering, Parameter, PathPattern,
    PathSegment, Projection, PropertyKey, QueryId, QueryIrError, QueryOperatorId, QueryResult,
    RelationId, RelationPattern, SetQuantifier, SortDirection, UnaryOperator, Value,
};

/// Stable V1 schema for source-bound parser-owned compilation evidence.
pub const PARSER_OWNED_COMPILATION_SCHEMA_V1: &str = "mrr.parser-owned-compilation.v1";

use crate::projection as ast;
use crate::projection::{PatternElement, QueryClause};
use crate::result_lowering::{lower_page_value, visible_bindings};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Stateless parser-owned compiler with an explicit AOT language selection.
pub struct QueryFrontend {
    language: ParserLanguage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Fail-closed frontend diagnostics, unsupported syntax, or IR rejection.
pub enum FrontendError {
    /// The source requested a feature outside the bounded parity slice.
    Unsupported(String),
    /// The shared query owner rejected the lowered semantic contract.
    InvalidQuery(QueryIrError),
    /// The parser-owned native runtime or its lossless CST rejected the request.
    ParserOwned(String),
}

/// Authority and source binding retained outside language-neutral MetaQueryIR.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParserOwnedCompilationReceipt {
    /// Stable V1 receipt schema.
    pub schema: &'static str,
    /// Explicit parser-owned AOT language selected for this compilation.
    pub language: ParserLanguage,
    /// Caller-owned source identity.
    pub source_name: String,
    /// Digest of the exact UTF-8 source.
    pub source_digest: String,
    /// Digest of the selected immutable parser grammar.
    pub grammar_digest: String,
    /// Language-neutral identity of the admitted query.
    pub query_id: QueryId,
}

/// Parser-owned compilation result plus its V1 provenance receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParserOwnedCompilation {
    pub query: MetaQueryIr,
    pub receipt: ParserOwnedCompilationReceipt,
}

impl From<QueryIrError> for FrontendError {
    fn from(error: QueryIrError) -> Self {
        Self::InvalidQuery(error)
    }
}

impl QueryFrontend {
    #[must_use]
    /// Creates a frontend that consumes one explicit parser-owned AOT language.
    pub const fn new(language: ParserLanguage) -> Self {
        Self { language }
    }

    /// Compiles the admitted GQL/Cypher parity slice through the selected AOT CST.
    ///
    /// This is an explicit migration boundary: it does not fall back to the
    /// parser-owned grammar accepts a shape whose semantic lowering is not yet
    /// owned here.
    pub fn compile(&self, name: &str, source: &str) -> Result<MetaQueryIr, FrontendError> {
        self.compile_with_receipt(name, source)
            .map(|compilation| compilation.query)
    }

    /// Compiles through the parser-owned path and retains source authority.
    pub fn compile_with_receipt(
        &self,
        name: &str,
        source: &str,
    ) -> Result<ParserOwnedCompilation, FrontendError> {
        let lowered = crate::parser_owned::lower_parser_owned_ast(self.language, source)?;
        let query = lower_query(&lowered.query)?;
        let receipt = ParserOwnedCompilationReceipt {
            schema: PARSER_OWNED_COMPILATION_SCHEMA_V1,
            language: lowered.language,
            source_name: name.into(),
            source_digest: lowered.source_digest,
            grammar_digest: lowered.grammar_digest,
            query_id: query.id(),
        };
        Ok(ParserOwnedCompilation { query, receipt })
    }
}

fn lower_query(query: &ast::Query) -> Result<MetaQueryIr, FrontendError> {
    let query_id = QueryId::from_canonical_bytes(semantic_key(query))
        .map_err(|error| FrontendError::Unsupported(error.to_string()))?;
    let mut match_clause = None;
    let mut predicates = Vec::new();
    let mut return_projections = None;
    let mut return_quantifier = SetQuantifier::All;
    let mut return_all_bindings = false;
    let mut finish = false;
    let mut order_keys = Vec::new();
    let mut group_keys = Vec::new();
    let mut offset = None;
    let mut limit = None;

    for clause in &query.clauses {
        match clause {
            QueryClause::Match(found) if match_clause.is_none() => match_clause = Some(found),
            QueryClause::Match(_) => return unsupported("multiple MATCH clauses"),
            QueryClause::Where(expression) | QueryClause::Filter(expression) => {
                predicates.push(lower_expression(expression)?);
            }
            QueryClause::Return {
                quantifier,
                all_bindings,
                projections,
                ..
            } if return_projections.is_none() && !finish => {
                return_projections = Some(projections.as_slice());
                return_quantifier = match quantifier {
                    Some(ast::SetQuantifier::Distinct) => SetQuantifier::Distinct,
                    Some(ast::SetQuantifier::All) | None => SetQuantifier::All,
                };
                return_all_bindings = *all_bindings;
            }
            QueryClause::Return { .. } => return unsupported("multiple RETURN clauses"),
            QueryClause::Finish if return_projections.is_none() && !finish => finish = true,
            QueryClause::Finish => return unsupported("multiple result statements"),
            QueryClause::Limit(value) => limit = Some(lower_page_value(value)?),
            QueryClause::OrderBy(keys) => order_keys.extend(keys),
            QueryClause::Offset(value) => offset = Some(lower_page_value(value)?),
            QueryClause::GroupBy(keys) => group_keys.extend(keys),
        }
    }

    let matched = match_clause.ok_or_else(|| {
        FrontendError::Unsupported("the parity slice requires one MATCH clause".into())
    })?;
    if matched.patterns.is_empty() {
        return unsupported("empty MATCH pattern list");
    }
    let (graph, property_predicates) = lower_graph(query_id, &matched.patterns)?;
    predicates.splice(0..0, property_predicates);

    let filters = predicates
        .into_iter()
        .enumerate()
        .map(|(index, predicate)| Filter::new(operator_id(query_id, "filter", index), predicate))
        .collect();
    let mut projections = Vec::new();
    let mut aggregations = Vec::new();
    if return_all_bindings {
        projections.extend(visible_bindings(&graph).into_iter().enumerate().map(
            |(index, binding)| {
                Projection::new(
                    operator_id(query_id, "projection", index),
                    Expression::Binding(binding.clone()),
                    binding,
                )
            },
        ));
    }
    for (index, projection) in return_projections.unwrap_or_default().iter().enumerate() {
        let operator_index = projections.len() + index;
        let alias = projection.alias.as_ref().map_or_else(
            || format!("result_{operator_index}"),
            |alias| alias.text.clone(),
        );
        if let ast::Expression::AggregateCall {
            function,
            quantifier,
            arguments,
            count_star,
            ..
        } = &projection.expression
        {
            aggregations.push(Aggregation::new(
                operator_id(query_id, "aggregation", operator_index),
                lower_aggregation_function(*function),
                quantifier.map(|quantifier| match quantifier {
                    ast::SetQuantifier::All => SetQuantifier::All,
                    ast::SetQuantifier::Distinct => SetQuantifier::Distinct,
                }),
                arguments
                    .iter()
                    .map(lower_expression)
                    .collect::<Result<Vec<_>, _>>()?,
                *count_star,
                Binding::new(alias)?,
            ));
        } else {
            projections.push(Projection::new(
                operator_id(query_id, "projection", operator_index),
                lower_expression(&projection.expression)?,
                Binding::new(alias)?,
            ));
        }
    }
    let ordering = order_keys
        .into_iter()
        .enumerate()
        .map(|(index, key)| {
            if key.null_ordering.is_some() {
                return unsupported("NULLS ordering");
            }
            Ok(Ordering::new(
                operator_id(query_id, "ordering", index),
                lower_expression(&key.expression)?,
                match key.direction {
                    None | Some(ast::SortDirection::Ascending) => SortDirection::Ascending,
                    Some(ast::SortDirection::Descending) => SortDirection::Descending,
                },
            ))
        })
        .collect::<Result<Vec<_>, FrontendError>>()?;

    let grouping = group_keys
        .into_iter()
        .enumerate()
        .map(|(index, expression)| {
            Ok(Grouping::new(
                operator_id(query_id, "grouping", index),
                lower_expression(expression)?,
            ))
        })
        .collect::<Result<Vec<_>, FrontendError>>()?;

    let result = if finish {
        QueryResult::finish()
    } else {
        QueryResult::returning(return_quantifier)
            .with_projections(projections)
            .with_aggregations(aggregations)
            .with_grouping(grouping)
            .with_ordering(ordering)
            .with_offset(offset)
            .with_limit(limit)
    };

    MetaQueryIr::new(query_id, graph, filters, result)
        .map(MetaQueryIr::normalized)
        .map_err(FrontendError::InvalidQuery)
}

fn lower_graph(
    query: QueryId,
    patterns: &[ast::PathPattern],
) -> Result<(GraphPattern, Vec<Expression>), FrontendError> {
    let mut predicates = Vec::new();
    let paths = patterns
        .iter()
        .enumerate()
        .map(|(index, pattern)| {
            let (path, path_predicates) = lower_path(&pattern.elements, index)?;
            predicates.extend(path_predicates);
            Ok(path)
        })
        .collect::<Result<Vec<_>, FrontendError>>()?;
    Ok((
        GraphPattern::new(operator_id(query, "graph", 0), paths)?,
        predicates,
    ))
}

fn lower_path(
    elements: &[PatternElement],
    path_index: usize,
) -> Result<(PathPattern, Vec<Expression>), FrontendError> {
    let Some(PatternElement::Node(start)) = elements.first() else {
        return unsupported("a path that does not begin with a node");
    };
    let (start, mut predicates) = lower_node(start, path_index, 0)?;
    let mut segments = Vec::new();
    let mut remaining = &elements[1..];
    let mut node_index = 1;
    while !remaining.is_empty() {
        let [
            PatternElement::Edge(edge),
            PatternElement::Node(node),
            tail @ ..,
        ] = remaining
        else {
            return unsupported("a path that is not an alternating node-edge-node sequence");
        };
        let relation = lower_relation(edge)?;
        predicates.extend(lower_properties(
            edge.binding.as_ref().map(|binding| binding.text.as_str()),
            &edge.properties,
        )?);
        let (node, node_predicates) = lower_node(node, path_index, node_index)?;
        predicates.extend(node_predicates);
        segments.push(PathSegment::new(relation, node));
        remaining = tail;
        node_index += 1;
    }
    Ok((PathPattern::new(start, segments), predicates))
}

fn lower_node(
    node: &ast::NodePattern,
    path_index: usize,
    node_index: usize,
) -> Result<(NodePattern, Vec<Expression>), FrontendError> {
    let binding_name = node.binding.as_ref().map_or_else(
        || format!("_node_{path_index}_{node_index}"),
        |binding| binding.text.clone(),
    );
    let binding = Binding::new(binding_name.clone())?;
    let types = node
        .labels
        .iter()
        .map(|label| {
            mrr_query::EntityId::from_canonical_bytes(format!(
                "mrr.frontend.entity-type.v1\0{}",
                label.text
            ))
            .map_err(|error| FrontendError::Unsupported(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let predicates = lower_properties(Some(&binding_name), &node.properties)?;
    Ok((NodePattern::new(binding, types), predicates))
}

fn lower_relation(edge: &ast::EdgePattern) -> Result<RelationPattern, FrontendError> {
    let binding = edge
        .binding
        .as_ref()
        .map(|binding| Binding::new(binding.text.clone()))
        .transpose()?;
    let types = edge
        .labels
        .iter()
        .map(|label| {
            RelationId::from_canonical_bytes(format!(
                "mrr.frontend.relation-type.v1\0{}",
                label.text
            ))
            .map_err(|error| FrontendError::Unsupported(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let direction = match edge.direction {
        ast::EdgeDirection::Out => Direction::Outgoing,
        ast::EdgeDirection::In => Direction::Incoming,
        ast::EdgeDirection::Undirected => Direction::Undirected,
    };
    Ok(RelationPattern::new(binding, types, direction, 1, Some(1))?)
}

fn lower_properties(
    binding: Option<&str>,
    properties: &[ast::PropertyConstraint],
) -> Result<Vec<Expression>, FrontendError> {
    if properties.is_empty() {
        return Ok(Vec::new());
    }
    let binding = binding.ok_or_else(|| {
        FrontendError::Unsupported("properties require an explicit relation binding".into())
    })?;
    properties
        .iter()
        .map(|property| {
            Ok(Expression::Binary {
                left: Box::new(Expression::Property {
                    binding: Binding::new(binding)?,
                    key: PropertyKey::new(property.key.text.clone())?,
                }),
                operator: BinaryOperator::Equal,
                right: Box::new(lower_expression(&property.value)?),
            })
        })
        .collect()
}

fn lower_expression(expression: &ast::Expression) -> Result<Expression, FrontendError> {
    Ok(match expression {
        ast::Expression::Name(identifier) => {
            Expression::Binding(Binding::new(identifier.text.clone())?)
        }
        ast::Expression::Parameter(parameter) => {
            Expression::Parameter(Parameter::new(parameter.name.clone())?)
        }
        ast::Expression::Boolean(value) => Expression::Literal(Value::Boolean(*value)),
        ast::Expression::Null => Expression::Literal(Value::Null),
        ast::Expression::String(value) => Expression::Literal(Value::String(value.clone())),
        ast::Expression::Date(value) => Expression::Literal(Value::Date(value.clone())),
        ast::Expression::Time(value) => Expression::Literal(Value::Time(value.clone())),
        ast::Expression::Timestamp(value) => Expression::Literal(Value::Timestamp(value.clone())),
        ast::Expression::Duration(value) => Expression::Literal(Value::Duration(value.clone())),
        ast::Expression::Integer(value) => Expression::Literal(Value::Integer(*value)),
        ast::Expression::Decimal(value) => Expression::Literal(Value::Decimal(value.clone())),
        ast::Expression::ApproximateNumeric(value) => {
            Expression::Literal(Value::Float(value.clone()))
        }
        ast::Expression::List(values) => Expression::Literal(Value::List(
            values
                .iter()
                .map(lower_literal)
                .collect::<Result<Vec<_>, _>>()?,
        )),
        ast::Expression::Record(fields) => Expression::Literal(Value::Record(
            fields
                .iter()
                .map(|field| Ok((field.name.canonical_text(), lower_literal(&field.value)?)))
                .collect::<Result<Vec<_>, FrontendError>>()?,
        )),
        ast::Expression::PropertyAccess { base, property } => {
            let ast::Expression::Name(binding) = base.as_ref() else {
                return unsupported("nested property base");
            };
            Expression::Property {
                binding: Binding::new(binding.text.clone())?,
                key: PropertyKey::new(property.text.clone())?,
            }
        }
        ast::Expression::Unary { operator, operand } => match operator {
            ast::UnaryOperator::Not => Expression::Unary {
                operator: UnaryOperator::Not,
                operand: Box::new(lower_expression(operand)?),
            },
            ast::UnaryOperator::Negate => Expression::Unary {
                operator: UnaryOperator::Negate,
                operand: Box::new(lower_expression(operand)?),
            },
            ast::UnaryOperator::Plus => lower_expression(operand)?,
        },
        ast::Expression::NullPredicate {
            operand, negated, ..
        } => Expression::Unary {
            operator: if *negated {
                UnaryOperator::IsNotNull
            } else {
                UnaryOperator::IsNull
            },
            operand: Box::new(lower_expression(operand)?),
        },
        ast::Expression::TruthPredicate {
            operand,
            value,
            negated,
            ..
        } => Expression::Unary {
            operator: match (*value, *negated) {
                (ast::TruthValue::True, false) => UnaryOperator::IsTrue,
                (ast::TruthValue::True, true) => UnaryOperator::IsNotTrue,
                (ast::TruthValue::False, false) => UnaryOperator::IsFalse,
                (ast::TruthValue::False, true) => UnaryOperator::IsNotFalse,
                (ast::TruthValue::Unknown, false) => UnaryOperator::IsUnknown,
                (ast::TruthValue::Unknown, true) => UnaryOperator::IsNotUnknown,
            },
            operand: Box::new(lower_expression(operand)?),
        },
        ast::Expression::Binary {
            operator,
            left,
            right,
        } => Expression::Binary {
            left: Box::new(lower_expression(left)?),
            operator: lower_binary_operator(*operator)?,
            right: Box::new(lower_expression(right)?),
        },
        ast::Expression::AggregateCall { .. } => return unsupported("nested aggregate expression"),
    })
}

fn lower_aggregation_function(function: ast::AggregateFunction) -> AggregationFunction {
    match function {
        ast::AggregateFunction::Average => AggregationFunction::Average,
        ast::AggregateFunction::Count => AggregationFunction::Count,
        ast::AggregateFunction::Maximum => AggregationFunction::Maximum,
        ast::AggregateFunction::Minimum => AggregationFunction::Minimum,
        ast::AggregateFunction::Sum => AggregationFunction::Sum,
        ast::AggregateFunction::CollectList => AggregationFunction::CollectList,
        ast::AggregateFunction::StandardDeviationSample => {
            AggregationFunction::StandardDeviationSample
        }
        ast::AggregateFunction::StandardDeviationPopulation => {
            AggregationFunction::StandardDeviationPopulation
        }
        ast::AggregateFunction::PercentileContinuous => AggregationFunction::PercentileContinuous,
        ast::AggregateFunction::PercentileDiscrete => AggregationFunction::PercentileDiscrete,
    }
}

fn lower_literal(expression: &ast::Expression) -> Result<Value, FrontendError> {
    match lower_expression(expression)? {
        Expression::Literal(value) => Ok(value),
        _ => unsupported("non-literal list element"),
    }
}

fn lower_binary_operator(operator: ast::BinaryOperator) -> Result<BinaryOperator, FrontendError> {
    Ok(match operator {
        ast::BinaryOperator::Add => BinaryOperator::Add,
        ast::BinaryOperator::Subtract => BinaryOperator::Subtract,
        ast::BinaryOperator::Multiply => BinaryOperator::Multiply,
        ast::BinaryOperator::Divide => BinaryOperator::Divide,
        ast::BinaryOperator::Equals => BinaryOperator::Equal,
        ast::BinaryOperator::NotEquals => BinaryOperator::NotEqual,
        ast::BinaryOperator::LessThan => BinaryOperator::Less,
        ast::BinaryOperator::LessThanOrEqual => BinaryOperator::LessOrEqual,
        ast::BinaryOperator::GreaterThan => BinaryOperator::Greater,
        ast::BinaryOperator::GreaterThanOrEqual => BinaryOperator::GreaterOrEqual,
        ast::BinaryOperator::And => BinaryOperator::And,
        ast::BinaryOperator::Or => BinaryOperator::Or,
    })
}

fn operator_id(query: QueryId, role: &str, index: usize) -> QueryOperatorId {
    QueryOperatorId::from_canonical_bytes(format!("{query}\0{role}\0{index}"))
        .expect("query and static operator role are canonical")
}

fn semantic_key(query: &ast::Query) -> Vec<u8> {
    let mut key = b"mrr.frontend.semantic-query.v1\0".to_vec();
    for clause in &query.clauses {
        append_clause(&mut key, clause);
    }
    key
}

fn append_clause(key: &mut Vec<u8>, clause: &QueryClause) {
    match clause {
        QueryClause::Match(found) => {
            append(key, "match");
            for pattern in &found.patterns {
                append(key, "pattern");
                append_pattern(key, &pattern.elements);
            }
        }
        QueryClause::Where(expression) => {
            append(key, "where");
            append_expression(key, expression);
        }
        QueryClause::Filter(expression) => {
            append(key, "filter");
            append_expression(key, expression);
        }
        QueryClause::Return {
            quantifier,
            all_bindings,
            projections,
            ..
        } => {
            append(key, "return");
            append(key, &format!("{quantifier:?}"));
            append(key, if *all_bindings { "star" } else { "items" });
            for projection in projections {
                append_expression(key, &projection.expression);
                append(
                    key,
                    projection
                        .alias
                        .as_ref()
                        .map_or("", |alias| alias.text.as_str()),
                );
            }
        }
        QueryClause::Finish => append(key, "finish"),
        QueryClause::Limit(value) => {
            append(key, "limit");
            append_non_negative_integer_specification(key, value);
        }
        QueryClause::OrderBy(keys) => {
            append(key, "order");
            for sort_key in keys {
                append_expression(key, &sort_key.expression);
                append(key, &format!("{:?}", sort_key.direction));
                append(key, &format!("{:?}", sort_key.null_ordering));
            }
        }
        QueryClause::Offset(value) => {
            append(key, "offset");
            append_non_negative_integer_specification(key, value);
        }
        QueryClause::GroupBy(keys) => {
            append(key, "group");
            for expression in keys {
                append_expression(key, expression);
            }
        }
    }
}

fn append_non_negative_integer_specification(
    key: &mut Vec<u8>,
    value: &ast::NonNegativeIntegerSpecification,
) {
    match value {
        ast::NonNegativeIntegerSpecification::Literal(value) => {
            append(key, "literal");
            append(key, &value.to_string());
        }
        ast::NonNegativeIntegerSpecification::Parameter(parameter) => {
            append(key, "parameter");
            append(key, &parameter.name);
        }
    }
}

fn append_pattern(key: &mut Vec<u8>, elements: &[PatternElement]) {
    for element in elements {
        match element {
            PatternElement::Node(node) => {
                append(key, "node");
                append(key, node.binding.as_ref().map_or("", |item| &item.text));
                for label in &node.labels {
                    append(key, &label.text);
                }
                for property in &node.properties {
                    append(key, &property.key.text);
                    append_expression(key, &property.value);
                }
            }
            PatternElement::Edge(edge) => {
                append(key, "relation");
                append(key, edge.binding.as_ref().map_or("", |item| &item.text));
                append(key, &format!("{:?}", edge.direction));
                for label in &edge.labels {
                    append(key, &label.text);
                }
            }
        }
    }
}

fn append_expression(key: &mut Vec<u8>, expression: &ast::Expression) {
    match expression {
        ast::Expression::Name(value) => append(key, &format!("name:{}", value.text)),
        ast::Expression::Parameter(value) => append(key, &format!("parameter:{}", value.name)),
        ast::Expression::Boolean(value) => append(key, &format!("bool:{value}")),
        ast::Expression::Null => append(key, "null"),
        ast::Expression::String(value) => append(key, &format!("string:{value}")),
        ast::Expression::Date(value) => append(key, &format!("date:{value}")),
        ast::Expression::Time(value) => append(key, &format!("time:{value}")),
        ast::Expression::Timestamp(value) => append(key, &format!("timestamp:{value}")),
        ast::Expression::Duration(value) => append(key, &format!("duration:{value}")),
        ast::Expression::Integer(value) => append(key, &format!("integer:{value}")),
        ast::Expression::Decimal(value) => append(key, &format!("decimal:{value}")),
        ast::Expression::ApproximateNumeric(value) => {
            append(key, &format!("float:{value}"));
        }
        ast::Expression::List(values) => {
            append(key, "list");
            for value in values {
                append_expression(key, value);
            }
        }
        ast::Expression::Record(fields) => {
            append(key, "record");
            for field in fields {
                append(key, &field.name.canonical_text());
                append_expression(key, &field.value);
            }
        }
        ast::Expression::PropertyAccess { base, property } => {
            append(key, "property");
            append_expression(key, base);
            append(key, &property.text);
        }
        ast::Expression::Unary { operator, operand } => {
            append(key, &format!("unary:{operator:?}"));
            append_expression(key, operand);
        }
        ast::Expression::Binary {
            operator,
            left,
            right,
        } => {
            append(key, &format!("binary:{operator:?}"));
            append_expression(key, left);
            append_expression(key, right);
        }
        ast::Expression::NullPredicate {
            operand, negated, ..
        } => {
            append(key, if *negated { "is-not-null" } else { "is-null" });
            append_expression(key, operand);
        }
        ast::Expression::TruthPredicate {
            operand,
            value,
            negated,
            ..
        } => {
            append(
                key,
                match (*value, *negated) {
                    (ast::TruthValue::True, false) => "is-true",
                    (ast::TruthValue::True, true) => "is-not-true",
                    (ast::TruthValue::False, false) => "is-false",
                    (ast::TruthValue::False, true) => "is-not-false",
                    (ast::TruthValue::Unknown, false) => "is-unknown",
                    (ast::TruthValue::Unknown, true) => "is-not-unknown",
                },
            );
            append_expression(key, operand);
        }
        ast::Expression::AggregateCall {
            function,
            quantifier,
            arguments,
            count_star,
            ..
        } => {
            append(key, &format!("aggregate:{function:?}"));
            append(key, &format!("quantifier:{quantifier:?}"));
            append(key, if *count_star { "count-star" } else { "values" });
            for argument in arguments {
                append_expression(key, argument);
            }
        }
    }
}

fn append(key: &mut Vec<u8>, value: &str) {
    key.extend_from_slice(value.len().to_string().as_bytes());
    key.push(b':');
    key.extend_from_slice(value.as_bytes());
    key.push(0);
}

fn unsupported<T>(feature: &str) -> Result<T, FrontendError> {
    Err(FrontendError::Unsupported(feature.into()))
}
