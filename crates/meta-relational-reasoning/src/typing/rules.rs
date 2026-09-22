//! Small, deterministic type-combination rules shared by the checker.

use mrr_query::BinaryOperator;
use mrr_relation::{Value, ValueKind, ValueSchema};

use crate::QueryCatalogBindingError;

use super::model::{ExpressionType, QueryType};

pub(super) fn literal_type(value: &Value) -> ExpressionType {
    let query_type = match value {
        Value::Null => QueryType::Null,
        Value::Entity(_) => QueryType::Schema(ValueSchema::Entity),
        Value::Boolean(_) => QueryType::Schema(ValueSchema::Boolean),
        Value::Integer(_) => QueryType::Schema(ValueSchema::Integer),
        Value::String(_) => QueryType::Schema(ValueSchema::String),
        Value::ByteString(_) => QueryType::Schema(ValueSchema::ByteString),
        Value::Date(_) => QueryType::Schema(ValueSchema::Date),
        Value::Duration(_) => QueryType::Schema(ValueSchema::Duration),
        _ => QueryType::Kind(value.kind()),
    };
    ExpressionType::new(query_type, matches!(value, Value::Null))
}

pub(super) fn require_compatible(
    expected: &ExpressionType,
    actual: &ExpressionType,
    context: &'static str,
) -> Result<(), QueryCatalogBindingError> {
    merge_compatible(expected, actual, context).map(|_| ())
}

pub(super) fn merge_compatible(
    left: &ExpressionType,
    right: &ExpressionType,
    operator: &'static str,
) -> Result<ExpressionType, QueryCatalogBindingError> {
    let query_type = match (&left.query_type, &right.query_type) {
        (QueryType::Null, query_type) | (query_type, QueryType::Null) => query_type.clone(),
        (QueryType::AnyValue, query_type) | (query_type, QueryType::AnyValue)
            if !matches!(query_type, QueryType::Node(_) | QueryType::Relation(_)) =>
        {
            query_type.clone()
        }
        (QueryType::Schema(left), QueryType::Schema(right)) if left == right => {
            QueryType::Schema(left.clone())
        }
        (QueryType::Schema(schema), QueryType::Kind(kind))
        | (QueryType::Kind(kind), QueryType::Schema(schema))
            if schema.kind() == *kind =>
        {
            QueryType::Schema(schema.clone())
        }
        (QueryType::Kind(left), QueryType::Kind(right)) if left == right => QueryType::Kind(*left),
        (QueryType::Numeric, query_type) | (query_type, QueryType::Numeric)
            if is_numeric_type(query_type) =>
        {
            QueryType::Numeric
        }
        (left, right) if is_numeric_type(left) && is_numeric_type(right) => QueryType::Numeric,
        (QueryType::Node(left), QueryType::Node(right)) if left == right => {
            QueryType::Node(left.clone())
        }
        (QueryType::Relation(left), QueryType::Relation(right)) if left == right => {
            QueryType::Relation(left.clone())
        }
        (QueryType::List(left), QueryType::List(right)) if left == right => {
            QueryType::List(left.clone())
        }
        _ => {
            return Err(QueryCatalogBindingError::IncompatibleOperands {
                operator,
                left: describe(&left.query_type),
                right: describe(&right.query_type),
            });
        }
    };
    Ok(ExpressionType::new(
        query_type,
        left.nullable || right.nullable,
    ))
}

pub(super) fn require_kind(
    expression_type: &ExpressionType,
    expected: ValueKind,
    context: &'static str,
) -> Result<(), QueryCatalogBindingError> {
    let actual = scalar_kind(&expression_type.query_type);
    if actual == Some(expected) || matches!(expression_type.query_type, QueryType::Null) {
        Ok(())
    } else {
        Err(QueryCatalogBindingError::TypeMismatch {
            context,
            expected: format!("{expected:?}"),
            actual: describe(&expression_type.query_type),
        })
    }
}

pub(super) fn require_numeric(
    expression_type: &ExpressionType,
    context: &'static str,
) -> Result<(), QueryCatalogBindingError> {
    let kind = scalar_kind(&expression_type.query_type);
    if matches!(expression_type.query_type, QueryType::Numeric)
        || matches!(
            kind,
            Some(ValueKind::Integer | ValueKind::Decimal | ValueKind::Float)
        )
        || matches!(expression_type.query_type, QueryType::Null)
    {
        Ok(())
    } else {
        Err(QueryCatalogBindingError::TypeMismatch {
            context,
            expected: "numeric".into(),
            actual: describe(&expression_type.query_type),
        })
    }
}

pub(super) fn require_ordered(
    expression_type: &ExpressionType,
    context: &'static str,
) -> Result<(), QueryCatalogBindingError> {
    let kind = scalar_kind(&expression_type.query_type);
    if matches!(expression_type.query_type, QueryType::Numeric)
        || matches!(
            kind,
            Some(
                ValueKind::Integer
                    | ValueKind::Decimal
                    | ValueKind::Float
                    | ValueKind::String
                    | ValueKind::Date
                    | ValueKind::Time
                    | ValueKind::Timestamp
                    | ValueKind::Duration
            )
        )
        || matches!(expression_type.query_type, QueryType::Null)
    {
        Ok(())
    } else if let Some(kind) = kind {
        Err(QueryCatalogBindingError::UnsupportedValueKind { context, kind })
    } else {
        Err(QueryCatalogBindingError::TypeMismatch {
            context,
            expected: "ordered scalar".into(),
            actual: describe(&expression_type.query_type),
        })
    }
}

fn scalar_kind(query_type: &QueryType) -> Option<ValueKind> {
    match query_type {
        QueryType::Schema(schema) => Some(schema.kind()),
        QueryType::Kind(kind) => Some(*kind),
        QueryType::List(_) => Some(ValueKind::List),
        QueryType::Node(_) => Some(ValueKind::Entity),
        QueryType::Relation(_) | QueryType::Numeric | QueryType::AnyValue | QueryType::Null => None,
    }
}

fn is_numeric_type(query_type: &QueryType) -> bool {
    matches!(query_type, QueryType::Numeric)
        || matches!(
            scalar_kind(query_type),
            Some(ValueKind::Integer | ValueKind::Decimal | ValueKind::Float)
        )
}

pub(super) fn operator_name(operator: BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::Add => "addition",
        BinaryOperator::Subtract => "subtraction",
        BinaryOperator::Multiply => "multiplication",
        BinaryOperator::Divide => "division",
        _ => "binary operator",
    }
}

fn describe(query_type: &QueryType) -> String {
    format!("{query_type:?}")
}
