//! Explicit GQL/Cypher adapters that lower the shared syntax slice directly
//! into `MetaQueryIr` without routing through the existing `gql-ir` contract.

use gql_ast::{self as ast, PatternElement, QueryClause, Statement};
use gql_source::Diagnostic;
use mrr_query::{
    Aggregation, AggregationFunction, BinaryOperator, Binding, Direction, Expression, Filter,
    GraphPattern, MetaQueryIr, NodePattern, Ordering, Parameter, PathPattern, PathSegment,
    Projection, PropertyKey, QueryId, QueryIrError, QueryOperatorId, RelationId, RelationPattern,
    SetQuantifier, SortDirection, UnaryOperator, Value,
};

use crate::value_type_identity::{append, append_value_type};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Surface language selected by the caller at the frontend boundary.
pub enum QueryLanguage {
    /// ISO GQL surface interpretation.
    Gql,
    /// openCypher language-surface interpretation.
    Cypher,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Stateless compiler for one explicit query-language surface.
pub struct QueryFrontend {
    language: QueryLanguage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Fail-closed frontend diagnostics, unsupported syntax, or IR rejection.
pub enum FrontendError {
    /// Parser or AST lowering diagnostics prevented semantic admission.
    Diagnostics(Vec<Diagnostic>),
    /// The source requested a feature outside the bounded parity slice.
    Unsupported(String),
    /// The shared query owner rejected the lowered semantic contract.
    InvalidQuery(QueryIrError),
}

impl From<QueryIrError> for FrontendError {
    fn from(error: QueryIrError) -> Self {
        Self::InvalidQuery(error)
    }
}

impl QueryFrontend {
    #[must_use]
    pub const fn new(language: QueryLanguage) -> Self {
        Self { language }
    }

    #[must_use]
    pub const fn language(&self) -> QueryLanguage {
        self.language
    }

    pub fn compile(&self, name: &str, source: &str) -> Result<MetaQueryIr, FrontendError> {
        let parse = gql_syntax::parse(name, source);
        let lowered = gql_ast::lower_from_syntax(&parse);
        if !lowered.diagnostics.is_empty() {
            return Err(FrontendError::Diagnostics(lowered.diagnostics));
        }
        let Some(Statement::Query(query)) = lowered.statement else {
            return Err(FrontendError::Unsupported(
                "only query statements lower to MetaQueryIR".into(),
            ));
        };
        lower_query(&query)
    }
}

fn lower_query(query: &ast::Query) -> Result<MetaQueryIr, FrontendError> {
    let query_id = QueryId::from_canonical_bytes(semantic_key(query))
        .map_err(|error| FrontendError::Unsupported(error.to_string()))?;
    let mut match_clause = None;
    let mut predicates = Vec::new();
    let mut return_projections = None;
    let mut order_keys = Vec::new();
    let mut limit = None;

    for clause in &query.clauses {
        match clause {
            QueryClause::Match(found) if match_clause.is_none() => match_clause = Some(found),
            QueryClause::Match(_) => return unsupported("multiple MATCH clauses"),
            QueryClause::Where { expression, .. } => predicates.push(lower_expression(expression)?),
            QueryClause::Filter { expression, .. } => {
                predicates.push(lower_expression(expression)?)
            }
            QueryClause::Return {
                quantifier: Some(ast::SetQuantifier::Distinct),
                ..
            } => return unsupported("RETURN DISTINCT"),
            QueryClause::Return {
                all_bindings: true, ..
            } => return unsupported("RETURN *"),
            QueryClause::Return { projections, .. } if return_projections.is_none() => {
                return_projections = Some(projections.as_slice());
            }
            QueryClause::Return { .. } => return unsupported("multiple RETURN clauses"),
            QueryClause::Finish { .. } => return unsupported("FINISH result statement"),
            QueryClause::Limit { value, .. } => match value {
                ast::NonNegativeIntegerSpecification::Literal(value) => limit = Some(*value),
                ast::NonNegativeIntegerSpecification::Parameter(_) => {
                    return unsupported("dynamic LIMIT");
                }
            },
            QueryClause::OrderBy { keys, .. } => order_keys.extend(keys),
            QueryClause::OptionalMatch(_) => return unsupported("OPTIONAL MATCH"),
            QueryClause::Let { .. } => return unsupported("LET"),
            QueryClause::For { .. } => return unsupported("FOR collection expansion"),
            QueryClause::Union { .. } => return unsupported("UNION"),
            QueryClause::Offset { .. } => return unsupported("OFFSET"),
            QueryClause::GroupBy { .. } => return unsupported("GROUP BY"),
            QueryClause::Insert { .. } => return unsupported("INSERT"),
            QueryClause::Set { .. } => return unsupported("SET"),
            QueryClause::Remove { .. } => return unsupported("REMOVE"),
            QueryClause::Delete { .. } => return unsupported("DELETE"),
        }
    }

    let matched = match_clause.ok_or_else(|| {
        FrontendError::Unsupported("the parity slice requires one MATCH clause".into())
    })?;
    if matched.mode.is_some() {
        return unsupported("graph match mode");
    }
    if matched.keep.is_some() {
        return unsupported("KEEP path prefix");
    }
    let [pattern] = matched.patterns.as_slice() else {
        return unsupported("multiple MATCH patterns");
    };
    if pattern.prefix.is_some() {
        return unsupported("path search prefix");
    }
    let (graph, property_predicates) = lower_graph(query_id, pattern)?;
    predicates.splice(0..0, property_predicates);

    let filters = predicates
        .into_iter()
        .enumerate()
        .map(|(index, predicate)| Filter::new(operator_id(query_id, "filter", index), predicate))
        .collect();
    let return_projections =
        return_projections.ok_or_else(|| FrontendError::Unsupported("RETURN projection".into()))?;
    let mut projections = Vec::new();
    let mut aggregations = Vec::new();
    for (index, projection) in return_projections.iter().enumerate() {
        let alias = projection
            .alias
            .as_ref()
            .map_or_else(|| format!("result_{index}"), |alias| alias.text.clone());
        if let ast::Expression::AggregateCall {
            function,
            quantifier,
            arguments,
            count_star,
            ..
        } = &projection.expression
        {
            aggregations.push(Aggregation::new(
                operator_id(query_id, "aggregation", index),
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
                operator_id(query_id, "projection", index),
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

    MetaQueryIr::new(
        query_id,
        graph,
        filters,
        projections,
        aggregations,
        ordering,
        limit,
    )
    .map(MetaQueryIr::normalized)
    .map_err(FrontendError::InvalidQuery)
}

fn lower_graph(
    query: QueryId,
    graph: &ast::PathPattern,
) -> Result<(GraphPattern, Vec<Expression>), FrontendError> {
    let mut predicates = Vec::new();
    let (path, path_predicates) = lower_path(&graph.elements, 0)?;
    predicates.extend(path_predicates);
    Ok((
        GraphPattern::new(operator_id(query, "graph", 0), vec![path])?,
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
    let (min, max) = edge
        .quantifier
        .as_ref()
        .map_or((1, Some(1)), |quantifier| (quantifier.min, quantifier.max));
    Ok(RelationPattern::new(binding, types, direction, min, max)?)
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
        ast::Expression::Boolean(value, _) => Expression::Literal(Value::Boolean(*value)),
        ast::Expression::Null(_) => Expression::Literal(Value::Null),
        ast::Expression::String(literal) => {
            Expression::Literal(Value::String(literal.value.clone()))
        }
        ast::Expression::ByteString(value, _) => {
            Expression::Literal(Value::ByteString(value.clone()))
        }
        ast::Expression::Date(value, _) => Expression::Literal(Value::Date(value.clone())),
        ast::Expression::Time(value, _) => Expression::Literal(Value::Time(value.clone())),
        ast::Expression::Timestamp(value, _) => {
            Expression::Literal(Value::Timestamp(value.clone()))
        }
        ast::Expression::Duration(value, _) => Expression::Literal(Value::Duration(value.clone())),
        ast::Expression::Integer(value, _) => Expression::Literal(Value::Integer(*value)),
        ast::Expression::Decimal(value, _) => Expression::Literal(Value::Decimal(value.clone())),
        ast::Expression::ApproximateNumeric(value, _) => {
            Expression::Literal(Value::Float(value.clone()))
        }
        ast::Expression::List(values, _) => Expression::Literal(Value::List(
            values
                .iter()
                .map(lower_literal)
                .collect::<Result<Vec<_>, _>>()?,
        )),
        ast::Expression::Record(fields, _) => Expression::Literal(Value::Record(
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
        ast::Expression::ValueTypePredicate { .. } => {
            return unsupported("value-type predicate expression");
        }
        ast::Expression::DirectedPredicate { .. }
        | ast::Expression::EndpointPredicate { .. }
        | ast::Expression::ElementIdentityPredicate { .. }
        | ast::Expression::PropertyExistsPredicate { .. } => {
            return unsupported("graph-element predicate expression");
        }
        ast::Expression::Binary {
            operator,
            left,
            right,
        } => Expression::Binary {
            left: Box::new(lower_expression(left)?),
            operator: lower_binary_operator(*operator)?,
            right: Box::new(lower_expression(right)?),
        },
        ast::Expression::IsLabeled { .. } => {
            return unsupported("label predicate expression");
        }
        ast::Expression::Subscript { .. } => return unsupported("subscript expression"),
        ast::Expression::Case { .. } => return unsupported("CASE expression"),
        ast::Expression::FunctionCall { .. } => return unsupported("function call expression"),
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
        ast::BinaryOperator::Modulo => return unsupported("modulo expression"),
        ast::BinaryOperator::In => return unsupported("IN expression"),
        ast::BinaryOperator::Concatenate => return unsupported("concatenation expression"),
        ast::BinaryOperator::Xor => return unsupported("XOR expression"),
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
        QueryClause::OptionalMatch(found) => {
            append(key, "optional-match");
            for pattern in &found.patterns {
                append(key, "pattern");
                append_pattern(key, &pattern.elements);
            }
        }
        QueryClause::Where { expression, .. } => {
            append(key, "where");
            append_expression(key, expression);
        }
        QueryClause::Filter { expression, .. } => {
            append(key, "filter");
            append_expression(key, expression);
        }
        QueryClause::For { item, .. } => {
            append(key, "for");
            append(key, &item.binding.text);
            append_expression(key, &item.source);
            if let Some(position) = &item.ordinality {
                append(
                    key,
                    match position.kind {
                        ast::ForOrdinalityKind::Ordinality => "ordinality",
                        ast::ForOrdinalityKind::Offset => "offset",
                    },
                );
                append(key, &position.binding.text);
            }
        }
        QueryClause::Let { bindings, .. } => {
            append(key, "let");
            for binding in bindings {
                append(key, &binding.binding.text);
                append_expression(key, &binding.value);
            }
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
        QueryClause::Finish { .. } => append(key, "finish"),
        QueryClause::Union { .. } => append(key, "union"),
        QueryClause::Limit { value, .. } => {
            append(key, "limit");
            append_non_negative_integer_specification(key, value);
        }
        QueryClause::OrderBy { keys, .. } => {
            append(key, "order");
            for sort_key in keys {
                append_expression(key, &sort_key.expression);
                append(key, &format!("{:?}", sort_key.direction));
                append(key, &format!("{:?}", sort_key.null_ordering));
            }
        }
        QueryClause::Offset { value, .. } => {
            append(key, "offset");
            append_non_negative_integer_specification(key, value);
        }
        QueryClause::GroupBy { keys, .. } => {
            append(key, "group");
            for expression in keys {
                append_expression(key, expression);
            }
        }
        QueryClause::Insert { patterns, .. } => {
            append(key, "insert");
            for pattern in patterns {
                append_pattern(key, &pattern.elements);
            }
        }
        QueryClause::Set { items, .. } => {
            append(key, "set");
            for item in items {
                append_expression(key, &item.target);
                append_expression(key, &item.value);
            }
        }
        QueryClause::Remove { targets, .. } => {
            append(key, "remove");
            for target in targets {
                append_expression(key, target);
            }
        }
        QueryClause::Delete {
            targets, detach, ..
        } => {
            append(key, if *detach { "detach-delete" } else { "delete" });
            for target in targets {
                append_expression(key, target);
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
                if let Some(quantifier) = &edge.quantifier {
                    append(key, &quantifier.min.to_string());
                    append(
                        key,
                        &quantifier
                            .max
                            .map_or_else(String::new, |value| value.to_string()),
                    );
                }
            }
            PatternElement::Path(path) => {
                append(key, "path");
                append_pattern(key, &path.elements);
            }
        }
    }
}

fn append_expression(key: &mut Vec<u8>, expression: &ast::Expression) {
    match expression {
        ast::Expression::Name(value) => append(key, &format!("name:{}", value.text)),
        ast::Expression::Parameter(value) => append(key, &format!("parameter:{}", value.name)),
        ast::Expression::Boolean(value, _) => append(key, &format!("bool:{value}")),
        ast::Expression::Null(_) => append(key, "null"),
        ast::Expression::String(literal) => append(key, &format!("string:{}", literal.value)),
        ast::Expression::ByteString(value, _) => {
            append(key, "bytes");
            for byte in value {
                append(key, &format!("{byte:02X}"));
            }
        }
        ast::Expression::Date(value, _) => append(key, &format!("date:{value}")),
        ast::Expression::Time(value, _) => append(key, &format!("time:{value}")),
        ast::Expression::Timestamp(value, _) => append(key, &format!("timestamp:{value}")),
        ast::Expression::Duration(value, _) => append(key, &format!("duration:{value}")),
        ast::Expression::Integer(value, _) => append(key, &format!("integer:{value}")),
        ast::Expression::Decimal(value, _) => append(key, &format!("decimal:{value}")),
        ast::Expression::ApproximateNumeric(value, _) => {
            append(key, &format!("float:{value}"));
        }
        ast::Expression::List(values, _) => {
            append(key, "list");
            for value in values {
                append_expression(key, value);
            }
        }
        ast::Expression::Record(fields, _) => {
            append(key, "record");
            for field in fields {
                append(key, &field.name.canonical_text());
                append_expression(key, &field.value);
            }
        }
        ast::Expression::Subscript { base, index } => {
            append(key, "subscript");
            append_expression(key, base);
            append_expression(key, index);
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
        ast::Expression::ValueTypePredicate {
            operand,
            value_type,
            negated,
            ..
        } => {
            append(
                key,
                if *negated {
                    "is-not-value-type"
                } else {
                    "is-value-type"
                },
            );
            append_expression(key, operand);
            append_value_type(key, value_type);
        }
        ast::Expression::DirectedPredicate { edge, negated, .. } => {
            append(
                key,
                if *negated {
                    "is-not-directed"
                } else {
                    "is-directed"
                },
            );
            append_expression(key, edge);
        }
        ast::Expression::EndpointPredicate {
            node,
            edge,
            endpoint,
            negated,
            ..
        } => {
            append(key, &format!("endpoint:{endpoint:?}:negated:{negated}"));
            append_expression(key, node);
            append_expression(key, edge);
        }
        ast::Expression::ElementIdentityPredicate { kind, elements, .. } => {
            append(key, &format!("element-identity:{kind:?}"));
            for element in elements {
                append_expression(key, element);
            }
        }
        ast::Expression::PropertyExistsPredicate {
            element, property, ..
        } => {
            append(key, "property-exists");
            append_expression(key, element);
            append(key, &property.canonical_text());
        }
        ast::Expression::IsLabeled {
            operand,
            label,
            negated,
            ..
        } => {
            append(
                key,
                if *negated {
                    "is-not-labeled"
                } else {
                    "is-labeled"
                },
            );
            append_expression(key, operand);
            append_label_expression(key, label);
        }
        ast::Expression::Case { .. } => append(key, "case"),
        ast::Expression::FunctionCall {
            name, arguments, ..
        } => {
            append(key, "function");
            append(key, &name.text);
            for argument in arguments {
                append_expression(key, argument);
            }
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

fn append_label_expression(key: &mut Vec<u8>, label: &ast::LabelExpression) {
    match label {
        ast::LabelExpression::Name(identifier) => {
            append(key, "label-name");
            append(key, &identifier.text);
        }
        ast::LabelExpression::Wildcard => append(key, "label-wildcard"),
        ast::LabelExpression::Not(operand) => {
            append(key, "label-not");
            append_label_expression(key, operand);
        }
        ast::LabelExpression::And(left, right) => {
            append(key, "label-and");
            append_label_expression(key, left);
            append_label_expression(key, right);
        }
        ast::LabelExpression::Or(left, right) => {
            append(key, "label-or");
            append_label_expression(key, left);
            append_label_expression(key, right);
        }
    }
}

fn unsupported<T>(feature: &str) -> Result<T, FrontendError> {
    Err(FrontendError::Unsupported(feature.into()))
}
