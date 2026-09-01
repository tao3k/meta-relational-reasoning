//! Explicit GQL/Cypher adapters that lower the shared syntax slice directly
//! into `MetaQueryIr` without routing through the existing `gql-ir` contract.

use gql_ast::{self as ast, PatternElement, QueryClause, Statement};
use gql_source::Diagnostic;
use mrr_query::{
    BinaryOperator, Binding, Direction, Expression, Filter, GraphPattern, MetaQueryIr, NodePattern,
    Ordering, PathPattern, PathSegment, Projection, PropertyKey, QueryId, QueryIrError,
    QueryOperatorId, RelationId, RelationPattern, SortDirection, UnaryOperator, Value,
};

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
        if !parse.diagnostics.is_empty() {
            return Err(FrontendError::Diagnostics(parse.diagnostics));
        }
        let lowered = gql_ast::lower_from_syntax(&parse);
        if !lowered.diagnostics.is_empty() {
            return Err(FrontendError::Diagnostics(lowered.diagnostics));
        }
        let Statement::Query(query) = lowered.statement else {
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
    let mut projection_expressions = None;
    let mut explicit_projections = None;
    let mut order_keys = Vec::new();
    let mut limit = None;

    for clause in &query.clauses {
        match clause {
            QueryClause::Match(found) if match_clause.is_none() => match_clause = Some(found),
            QueryClause::Match(_) => return unsupported("multiple MATCH clauses"),
            QueryClause::Where { expression } => predicates.push(lower_expression(expression)?),
            QueryClause::Return { expressions } => {
                projection_expressions = Some(expressions.as_slice());
            }
            QueryClause::ReturnAliased { projections } => {
                explicit_projections = Some(projections.as_slice());
            }
            QueryClause::Limit { value, .. } => limit = *value,
            QueryClause::OrderBy { keys, .. } => order_keys.extend(keys),
            QueryClause::OptionalMatch(_) => return unsupported("OPTIONAL MATCH"),
            QueryClause::Let { .. } => return unsupported("LET"),
            QueryClause::Union { .. } => return unsupported("UNION"),
            QueryClause::Offset { .. } => return unsupported("OFFSET"),
        }
    }

    let matched = match_clause.ok_or_else(|| {
        FrontendError::Unsupported("the parity slice requires one MATCH clause".into())
    })?;
    let (graph, property_predicates) = lower_graph(query_id, &matched.pattern)?;
    predicates.splice(0..0, property_predicates);

    let filters = predicates
        .into_iter()
        .enumerate()
        .map(|(index, predicate)| Filter::new(operator_id(query_id, "filter", index), predicate))
        .collect();
    let projections = if let Some(expressions) = projection_expressions {
        expressions
            .iter()
            .enumerate()
            .map(|(index, expression)| {
                Ok(Projection::new(
                    operator_id(query_id, "projection", index),
                    lower_expression(expression)?,
                    Binding::new(format!("result_{index}"))?,
                ))
            })
            .collect::<Result<Vec<_>, FrontendError>>()?
    } else if let Some(projections) = explicit_projections {
        projections
            .iter()
            .enumerate()
            .map(|(index, projection)| {
                let alias = projection
                    .alias
                    .as_ref()
                    .map_or_else(|| format!("result_{index}"), |alias| alias.text.clone());
                Ok(Projection::new(
                    operator_id(query_id, "projection", index),
                    lower_expression(&projection.expression)?,
                    Binding::new(alias)?,
                ))
            })
            .collect::<Result<Vec<_>, FrontendError>>()?
    } else {
        return unsupported("RETURN projection");
    };
    let ordering = order_keys
        .into_iter()
        .enumerate()
        .map(|(index, key)| {
            Ok(Ordering::new(
                operator_id(query_id, "ordering", index),
                lower_expression(&key.expression)?,
                match key.direction {
                    ast::SortDirection::Ascending => SortDirection::Ascending,
                    ast::SortDirection::Descending => SortDirection::Descending,
                },
            ))
        })
        .collect::<Result<Vec<_>, FrontendError>>()?;

    MetaQueryIr::new(
        query_id,
        graph,
        filters,
        projections,
        Vec::new(),
        ordering,
        limit,
    )
    .map(MetaQueryIr::normalized)
    .map_err(FrontendError::InvalidQuery)
}

fn lower_graph(
    query: QueryId,
    graph: &ast::GraphPattern,
) -> Result<(GraphPattern, Vec<Expression>), FrontendError> {
    let path_elements: Vec<&[PatternElement]> = if graph
        .elements
        .iter()
        .all(|element| matches!(element, PatternElement::Path(_)))
    {
        graph
            .elements
            .iter()
            .filter_map(|element| match element {
                PatternElement::Path(path) => Some(path.elements.as_slice()),
                _ => None,
            })
            .collect()
    } else {
        vec![graph.elements.as_slice()]
    };
    let mut predicates = Vec::new();
    let mut paths = Vec::with_capacity(path_elements.len());
    for (path_index, elements) in path_elements.into_iter().enumerate() {
        let (path, path_predicates) = lower_path(elements, path_index)?;
        paths.push(path);
        predicates.extend(path_predicates);
    }
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
        ast::Expression::Boolean(value, _) => Expression::Literal(Value::Boolean(*value)),
        ast::Expression::Null(_) => Expression::Literal(Value::Null),
        ast::Expression::String(value, _) => Expression::Literal(Value::String(value.clone())),
        ast::Expression::Integer(value, _) => Expression::Literal(Value::Integer(*value)),
        ast::Expression::Decimal(value, _) => Expression::Literal(Value::Decimal(value.clone())),
        ast::Expression::List(values, _) => Expression::Literal(Value::List(
            values
                .iter()
                .map(lower_literal)
                .collect::<Result<Vec<_>, _>>()?,
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
        ast::Expression::Unary { operator, operand } => Expression::Unary {
            operator: match operator {
                ast::UnaryOperator::Not => UnaryOperator::Not,
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
        ast::Expression::Subscript { .. } => return unsupported("subscript expression"),
        ast::Expression::Case { .. } => return unsupported("CASE expression"),
    })
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
            append_pattern(key, &found.pattern.elements);
        }
        QueryClause::OptionalMatch(found) => {
            append(key, "optional-match");
            append_pattern(key, &found.pattern.elements);
        }
        QueryClause::Where { expression } => {
            append(key, "where");
            append_expression(key, expression);
        }
        QueryClause::Let { binding, value } => {
            append(key, "let");
            append(key, &binding.text);
            append_expression(key, value);
        }
        QueryClause::Return { expressions } => {
            append(key, "return");
            for expression in expressions {
                append_expression(key, expression);
            }
        }
        QueryClause::ReturnAliased { projections } => {
            append(key, "return");
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
        QueryClause::Union { .. } => append(key, "union"),
        QueryClause::Limit { value, .. } => {
            append(key, "limit");
            append(
                key,
                &value.map_or_else(String::new, |value| value.to_string()),
            );
        }
        QueryClause::OrderBy { keys, .. } => {
            append(key, "order");
            for sort_key in keys {
                append_expression(key, &sort_key.expression);
                append(key, &format!("{:?}", sort_key.direction));
            }
        }
        QueryClause::Offset { value, .. } => {
            append(key, "offset");
            append(
                key,
                &value.map_or_else(String::new, |value| value.to_string()),
            );
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
        ast::Expression::Boolean(value, _) => append(key, &format!("bool:{value}")),
        ast::Expression::Null(_) => append(key, "null"),
        ast::Expression::String(value, _) => append(key, &format!("string:{value}")),
        ast::Expression::Integer(value, _) => append(key, &format!("integer:{value}")),
        ast::Expression::Decimal(value, _) => append(key, &format!("decimal:{value}")),
        ast::Expression::List(values, _) => {
            append(key, "list");
            for value in values {
                append_expression(key, value);
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
        ast::Expression::Case { .. } => append(key, "case"),
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
