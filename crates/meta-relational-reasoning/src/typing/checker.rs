// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Static query typing derived from admitted semantic catalogs.

use std::collections::{BTreeMap, BTreeSet};

use mrr_bundle::ReasoningBundle;
use mrr_identity::{EntityId, RelationId};
use mrr_query::{
    Aggregation, AggregationFunction, BinaryOperator, Binding, Expression, MetaQueryIr, PageValue,
    Parameter, PropertyKey, UnaryOperator,
};
use mrr_relation::{ValueKind, ValueSchema};

use crate::{QueryCatalogBindingError, ResolvedProperty};

use super::catalog::resolve_property;
use super::model::{ExpressionType, ParameterType, QueryType, ResultField, StaticQueryTyping};
use super::rules::{
    literal_type, merge_compatible, operator_name, require_compatible, require_kind,
    require_numeric, require_ordered,
};

#[derive(Clone, Debug)]
pub(super) enum BindingTarget {
    Node(BTreeSet<EntityId>),
    Relation(BTreeSet<RelationId>),
    Scalar(ExpressionType),
}

struct TypeChecker<'a> {
    bundle: &'a ReasoningBundle,
    targets: BTreeMap<Binding, BindingTarget>,
    properties: Vec<ResolvedProperty>,
    parameters: BTreeMap<String, ParameterType>,
}

pub(crate) fn type_query(
    query: &MetaQueryIr,
    bundle: &ReasoningBundle,
) -> Result<StaticQueryTyping, QueryCatalogBindingError> {
    let mut checker = TypeChecker::new(query, bundle)?;
    let boolean = ExpressionType::schema(ValueSchema::Boolean, true);
    for filter in query.filters() {
        let actual = checker.infer_expression(filter.predicate(), Some(&boolean))?;
        require_kind(&actual, ValueKind::Boolean, "filter predicate")?;
    }

    let mut projection_types = Vec::with_capacity(query.projections().len());
    for projection in query.projections() {
        projection_types.push(checker.infer_expression(projection.expression(), None)?);
    }
    let mut aggregation_types = Vec::with_capacity(query.aggregations().len());
    for aggregation in query.aggregations() {
        aggregation_types.push(checker.infer_aggregation(aggregation)?);
    }
    for grouping in query.grouping() {
        checker.infer_expression(grouping.expression(), None)?;
    }
    if !query.aggregations().is_empty() {
        for projection in query.projections() {
            if !query
                .grouping()
                .iter()
                .any(|grouping| grouping.expression() == projection.expression())
            {
                return Err(QueryCatalogBindingError::UngroupedProjection(
                    projection.alias().clone(),
                ));
            }
        }
    }

    let mut result_fields = Vec::with_capacity(projection_types.len() + aggregation_types.len());
    for (projection, expression_type) in query.projections().iter().zip(projection_types) {
        checker.register_scalar(projection.alias(), expression_type.clone())?;
        result_fields.push(ResultField {
            name: projection.alias().clone(),
            expression_type,
        });
    }
    for (aggregation, expression_type) in query.aggregations().iter().zip(aggregation_types) {
        checker.register_scalar(aggregation.alias(), expression_type.clone())?;
        result_fields.push(ResultField {
            name: aggregation.alias().clone(),
            expression_type,
        });
    }
    for ordering in query.ordering() {
        let expression_type = checker.infer_expression(ordering.expression(), None)?;
        require_ordered(&expression_type, "ordering expression")?;
    }
    for page in [query.offset(), query.limit()].into_iter().flatten() {
        if let PageValue::Parameter(parameter) = page {
            checker.constrain_parameter(
                parameter,
                ExpressionType::schema(ValueSchema::Integer, false),
            )?;
        }
    }

    checker.properties.sort_by(|left, right| {
        (left.binding().as_str(), left.key().as_str())
            .cmp(&(right.binding().as_str(), right.key().as_str()))
    });
    checker
        .properties
        .dedup_by(|left, right| left.binding() == right.binding() && left.key() == right.key());
    Ok(StaticQueryTyping {
        resolved_properties: checker.properties,
        parameters: checker.parameters.into_values().collect(),
        result_fields,
    })
}

impl<'a> TypeChecker<'a> {
    fn new(
        query: &MetaQueryIr,
        bundle: &'a ReasoningBundle,
    ) -> Result<Self, QueryCatalogBindingError> {
        let mut checker = Self {
            bundle,
            targets: BTreeMap::new(),
            properties: Vec::new(),
            parameters: BTreeMap::new(),
        };
        for path in query.graph().paths() {
            checker.register_node(path.start().binding(), path.start().types())?;
            for segment in path.segments() {
                if let Some(binding) = segment.relation().binding() {
                    checker.register_relation(binding, segment.relation().types())?;
                }
                checker.register_node(segment.node().binding(), segment.node().types())?;
            }
        }
        Ok(checker)
    }

    fn register_node(
        &mut self,
        binding: &Binding,
        types: &[EntityId],
    ) -> Result<(), QueryCatalogBindingError> {
        let types = if types.is_empty() {
            self.bundle
                .entity_catalog()
                .entities()
                .iter()
                .map(mrr_relation::EntitySchema::id)
                .collect()
        } else {
            types.iter().copied().collect()
        };
        self.register_target(binding, BindingTarget::Node(types))
    }

    fn register_relation(
        &mut self,
        binding: &Binding,
        types: &[RelationId],
    ) -> Result<(), QueryCatalogBindingError> {
        let types = if types.is_empty() {
            self.bundle
                .relation_catalog()
                .relations()
                .iter()
                .map(mrr_relation::RelationSchema::id)
                .collect()
        } else {
            types.iter().copied().collect()
        };
        self.register_target(binding, BindingTarget::Relation(types))
    }

    fn register_scalar(
        &mut self,
        binding: &Binding,
        expression_type: ExpressionType,
    ) -> Result<(), QueryCatalogBindingError> {
        self.register_target(binding, BindingTarget::Scalar(expression_type))
    }

    fn register_target(
        &mut self,
        binding: &Binding,
        target: BindingTarget,
    ) -> Result<(), QueryCatalogBindingError> {
        match self.targets.get_mut(binding) {
            None => {
                self.targets.insert(binding.clone(), target);
                Ok(())
            }
            Some(BindingTarget::Node(existing)) => match target {
                BindingTarget::Node(types) => {
                    existing.extend(types);
                    Ok(())
                }
                _ => Err(QueryCatalogBindingError::ConflictingBinding(
                    binding.clone(),
                )),
            },
            Some(BindingTarget::Relation(existing)) => match target {
                BindingTarget::Relation(types) => {
                    existing.extend(types);
                    Ok(())
                }
                _ => Err(QueryCatalogBindingError::ConflictingBinding(
                    binding.clone(),
                )),
            },
            Some(BindingTarget::Scalar(_)) => Err(QueryCatalogBindingError::ConflictingBinding(
                binding.clone(),
            )),
        }
    }

    fn infer_expression(
        &mut self,
        expression: &Expression,
        expected: Option<&ExpressionType>,
    ) -> Result<ExpressionType, QueryCatalogBindingError> {
        let inferred = match expression {
            Expression::Binding(binding) => self.type_binding(binding)?,
            Expression::Parameter(parameter) => {
                let expected = expected
                    .cloned()
                    .or_else(|| {
                        self.parameters
                            .get(parameter.as_str())
                            .map(|value| value.expression_type.clone())
                    })
                    .ok_or_else(|| {
                        QueryCatalogBindingError::UnconstrainedParameter(parameter.clone())
                    })?;
                self.constrain_parameter(parameter, expected.clone())?;
                expected
            }
            Expression::Property { binding, key } => self.type_property(binding, key)?,
            Expression::Literal(value) => literal_type(value),
            Expression::Unary { operator, operand } => {
                self.infer_unary(*operator, operand, expected)?
            }
            Expression::Binary {
                left,
                operator,
                right,
            } => self.infer_binary(left, *operator, right)?,
        };
        if let Some(expected) = expected {
            require_compatible(expected, &inferred, "expression")?;
        }
        Ok(inferred)
    }

    fn type_binding(&self, binding: &Binding) -> Result<ExpressionType, QueryCatalogBindingError> {
        match self
            .targets
            .get(binding)
            .ok_or_else(|| QueryCatalogBindingError::UnknownBinding(binding.clone()))?
        {
            BindingTarget::Node(types) => Ok(ExpressionType::new(
                QueryType::Node(types.iter().copied().collect()),
                false,
            )),
            BindingTarget::Relation(types) => Ok(ExpressionType::new(
                QueryType::Relation(types.iter().copied().collect()),
                false,
            )),
            BindingTarget::Scalar(expression_type) => Ok(expression_type.clone()),
        }
    }

    fn type_property(
        &mut self,
        binding: &Binding,
        key: &PropertyKey,
    ) -> Result<ExpressionType, QueryCatalogBindingError> {
        let target = self
            .targets
            .get(binding)
            .ok_or_else(|| QueryCatalogBindingError::UnknownBinding(binding.clone()))?;
        let field = resolve_property(target, binding, key, self.bundle)?;
        let resolved = ResolvedProperty {
            binding: binding.clone(),
            key: key.clone(),
            schema: field.schema().clone(),
            nullable: field.nullable(),
        };
        let expression_type = ExpressionType::schema(resolved.schema.clone(), resolved.nullable);
        self.properties.push(resolved);
        Ok(expression_type)
    }

    fn infer_unary(
        &mut self,
        operator: UnaryOperator,
        operand: &Expression,
        expected: Option<&ExpressionType>,
    ) -> Result<ExpressionType, QueryCatalogBindingError> {
        match operator {
            UnaryOperator::Not => {
                let boolean = ExpressionType::schema(ValueSchema::Boolean, true);
                let operand = self.infer_expression(operand, Some(&boolean))?;
                require_kind(&operand, ValueKind::Boolean, "NOT operand")?;
                Ok(ExpressionType::schema(
                    ValueSchema::Boolean,
                    operand.nullable,
                ))
            }
            UnaryOperator::Negate => {
                let operand = self.infer_expression(operand, expected)?;
                require_numeric(&operand, "negation operand")?;
                Ok(operand)
            }
            UnaryOperator::IsNull | UnaryOperator::IsNotNull => {
                let any = ExpressionType::new(QueryType::AnyValue, true);
                self.infer_expression(operand, Some(&any))?;
                Ok(ExpressionType::schema(ValueSchema::Boolean, false))
            }
            UnaryOperator::IsTrue
            | UnaryOperator::IsNotTrue
            | UnaryOperator::IsFalse
            | UnaryOperator::IsNotFalse
            | UnaryOperator::IsUnknown
            | UnaryOperator::IsNotUnknown => {
                let boolean = ExpressionType::schema(ValueSchema::Boolean, true);
                let operand = self.infer_expression(operand, Some(&boolean))?;
                require_kind(&operand, ValueKind::Boolean, "truth predicate operand")?;
                Ok(ExpressionType::schema(ValueSchema::Boolean, false))
            }
        }
    }

    fn infer_binary(
        &mut self,
        left: &Expression,
        operator: BinaryOperator,
        right: &Expression,
    ) -> Result<ExpressionType, QueryCatalogBindingError> {
        let (left_type, right_type) = self.infer_pair(left, right)?;
        let nullable = left_type.nullable || right_type.nullable;
        match operator {
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                require_compatible(&left_type, &right_type, "equality operands")?;
                Ok(ExpressionType::schema(ValueSchema::Boolean, nullable))
            }
            BinaryOperator::Less
            | BinaryOperator::LessOrEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterOrEqual => {
                require_ordered(&left_type, "ordered comparison")?;
                require_ordered(&right_type, "ordered comparison")?;
                require_compatible(&left_type, &right_type, "comparison operands")?;
                Ok(ExpressionType::schema(ValueSchema::Boolean, nullable))
            }
            BinaryOperator::And | BinaryOperator::Or => {
                require_kind(&left_type, ValueKind::Boolean, "Boolean operand")?;
                require_kind(&right_type, ValueKind::Boolean, "Boolean operand")?;
                Ok(ExpressionType::schema(ValueSchema::Boolean, nullable))
            }
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide => {
                require_numeric(&left_type, "arithmetic operand")?;
                require_numeric(&right_type, "arithmetic operand")?;
                merge_compatible(&left_type, &right_type, operator_name(operator)).map(
                    |mut result| {
                        result.nullable = nullable;
                        result
                    },
                )
            }
        }
    }

    fn infer_pair(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> Result<(ExpressionType, ExpressionType), QueryCatalogBindingError> {
        match (left, right) {
            (Expression::Parameter(left), Expression::Parameter(_)) => Err(
                QueryCatalogBindingError::UnconstrainedParameter(left.clone()),
            ),
            (Expression::Parameter(parameter), _) => {
                let right_type = self.infer_expression(right, None)?;
                let left_type = self.constrain_parameter(parameter, right_type.clone())?;
                Ok((left_type, right_type))
            }
            (_, Expression::Parameter(parameter)) => {
                let left_type = self.infer_expression(left, None)?;
                let right_type = self.constrain_parameter(parameter, left_type.clone())?;
                Ok((left_type, right_type))
            }
            _ => Ok((
                self.infer_expression(left, None)?,
                self.infer_expression(right, None)?,
            )),
        }
    }

    fn infer_aggregation(
        &mut self,
        aggregation: &Aggregation,
    ) -> Result<ExpressionType, QueryCatalogBindingError> {
        if aggregation.is_count_star() {
            return Ok(ExpressionType::schema(ValueSchema::Integer, false));
        }
        let first = if aggregation.function() == AggregationFunction::Count {
            let any = ExpressionType::new(QueryType::AnyValue, true);
            self.infer_expression(&aggregation.expressions()[0], Some(&any))?
        } else {
            self.infer_expression(&aggregation.expressions()[0], None)?
        };
        match aggregation.function() {
            AggregationFunction::Count => Ok(ExpressionType::schema(ValueSchema::Integer, false)),
            AggregationFunction::Sum => {
                require_numeric(&first, "SUM argument")?;
                Ok(first.with_nullable(true))
            }
            AggregationFunction::Minimum | AggregationFunction::Maximum => {
                require_ordered(&first, "MIN/MAX argument")?;
                Ok(first.with_nullable(true))
            }
            AggregationFunction::Average => {
                require_numeric(&first, "AVERAGE argument")?;
                Ok(ExpressionType::new(QueryType::Numeric, true))
            }
            AggregationFunction::CollectList => {
                Ok(ExpressionType::new(QueryType::List(Box::new(first)), false))
            }
            AggregationFunction::StandardDeviationSample
            | AggregationFunction::StandardDeviationPopulation => {
                require_numeric(&first, "standard deviation argument")?;
                Ok(ExpressionType::new(QueryType::Numeric, true))
            }
            AggregationFunction::PercentileContinuous | AggregationFunction::PercentileDiscrete => {
                require_ordered(&first, "percentile value argument")?;
                let percentile = self.infer_expression(&aggregation.expressions()[1], None)?;
                require_numeric(&percentile, "percentile fraction argument")?;
                Ok(first.with_nullable(true))
            }
        }
    }

    fn constrain_parameter(
        &mut self,
        parameter: &Parameter,
        required: ExpressionType,
    ) -> Result<ExpressionType, QueryCatalogBindingError> {
        if matches!(
            required.query_type,
            QueryType::Node(_) | QueryType::Relation(_) | QueryType::Null
        ) {
            return Err(QueryCatalogBindingError::ConflictingParameterType(
                parameter.clone(),
            ));
        }
        let name = parameter.as_str().to_owned();
        if let Some(existing) = self.parameters.get_mut(&name) {
            let mut merged = merge_compatible(
                &existing.expression_type,
                &required,
                "parameter constraints",
            )
            .map_err(|_| QueryCatalogBindingError::ConflictingParameterType(parameter.clone()))?;
            merged.nullable = existing.expression_type.nullable && required.nullable;
            existing.expression_type = merged.clone();
            Ok(merged)
        } else {
            self.parameters.insert(
                name,
                ParameterType {
                    parameter: parameter.clone(),
                    expression_type: required.clone(),
                },
            );
            Ok(required)
        }
    }
}
