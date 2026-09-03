//! Backend-neutral value-type inference for semantic analysis.

use std::collections::HashMap;

use gql_ast::{AggregateFunction, BinaryOperator, Expression, UnaryOperator};
use gql_types::ValueType;

pub(super) fn expression_type(
    expression: &Expression,
    bindings: &HashMap<String, ValueType>,
) -> Option<ValueType> {
    match expression {
        Expression::Name(identifier) => bindings.get(&identifier.canonical_text()).cloned(),
        Expression::Parameter(_) => Some(ValueType::Any),
        Expression::Boolean(_, _) => Some(ValueType::Boolean),
        Expression::Null(_) => Some(ValueType::Null),
        Expression::String(_) => Some(ValueType::String),
        Expression::ByteString(_, _) => Some(ValueType::ByteString),
        Expression::Date(_, _) => Some(ValueType::Date),
        Expression::Time(_, _) => Some(ValueType::Time),
        Expression::Timestamp(_, _) => Some(ValueType::Timestamp),
        Expression::Duration(_, _) => Some(ValueType::Duration),
        Expression::Integer(_, _) => Some(ValueType::Integer),
        Expression::Decimal(_, _) => Some(ValueType::Decimal),
        Expression::ApproximateNumeric(_, _) => Some(ValueType::Float),
        Expression::List(_, _) => Some(ValueType::List),
        Expression::Record(_, _) => Some(ValueType::Record),
        Expression::Subscript { .. } | Expression::PropertyAccess { .. } => Some(ValueType::Any),
        Expression::AggregateCall { function, .. } => Some(match function {
            AggregateFunction::Count => ValueType::Integer,
            AggregateFunction::CollectList => ValueType::List,
            AggregateFunction::Average
            | AggregateFunction::StandardDeviationSample
            | AggregateFunction::StandardDeviationPopulation
            | AggregateFunction::PercentileContinuous
            | AggregateFunction::PercentileDiscrete => ValueType::Float,
            AggregateFunction::Maximum | AggregateFunction::Minimum | AggregateFunction::Sum => {
                ValueType::Any
            }
        }),
        Expression::FunctionCall { .. } => Some(ValueType::Any),
        Expression::Unary { operator, operand } => match operator {
            UnaryOperator::Not => Some(ValueType::Boolean),
            UnaryOperator::Plus | UnaryOperator::Negate => expression_type(operand, bindings),
        },
        Expression::NullPredicate { .. }
        | Expression::TruthPredicate { .. }
        | Expression::ValueTypePredicate { .. }
        | Expression::DirectedPredicate { .. }
        | Expression::EndpointPredicate { .. }
        | Expression::ElementIdentityPredicate { .. }
        | Expression::PropertyExistsPredicate { .. } => Some(ValueType::Boolean),
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
                if left_type == ValueType::Float || right_type == ValueType::Float {
                    Some(ValueType::Float)
                } else if left_type == ValueType::Decimal || right_type == ValueType::Decimal {
                    Some(ValueType::Decimal)
                } else if is_numeric_type(&left_type) && is_numeric_type(&right_type) {
                    Some(ValueType::Integer)
                } else {
                    Some(ValueType::Any)
                }
            }
            BinaryOperator::Concatenate => Some(ValueType::String),
            _ => Some(ValueType::Boolean),
        },
        Expression::IsLabeled { .. } => Some(ValueType::Boolean),
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

pub(super) fn case_types_compatible(left: &ValueType, right: &ValueType) -> bool {
    left == right
        || matches!(left, ValueType::Any | ValueType::Null)
        || matches!(right, ValueType::Any | ValueType::Null)
        || (is_numeric_type(left) && is_numeric_type(right))
}

pub(super) fn is_numeric_type(value_type: &ValueType) -> bool {
    matches!(
        value_type,
        ValueType::Integer | ValueType::Decimal | ValueType::Float
    )
}
