//! Recursive aggregate detection over backend-neutral expression IR.
#![forbid(unsafe_code)]

use gql_ir::Expression;

pub(crate) fn contains_aggregate(expression: &Expression) -> bool {
    match expression {
        Expression::Aggregate { .. } => true,
        Expression::Subscript { base, index }
        | Expression::Binary {
            left: base,
            right: index,
            ..
        } => contains_aggregate(base) || contains_aggregate(index),
        Expression::PropertyAccess { base, .. }
        | Expression::Unary { operand: base, .. }
        | Expression::IsLabeled { operand: base, .. } => contains_aggregate(base),
        Expression::List(items) => items.iter().any(contains_aggregate),
        Expression::Record(fields) => fields.iter().any(|field| contains_aggregate(&field.value)),
        Expression::Case {
            operand,
            branches,
            else_result,
        } => {
            operand.as_deref().is_some_and(contains_aggregate)
                || branches.iter().any(|branch| {
                    contains_aggregate(&branch.condition) || contains_aggregate(&branch.result)
                })
                || else_result.as_deref().is_some_and(contains_aggregate)
        }
        Expression::Binding(_)
        | Expression::Boolean(_)
        | Expression::Null
        | Expression::String(_)
        | Expression::ByteString(_)
        | Expression::Date(_)
        | Expression::Time(_)
        | Expression::Timestamp(_)
        | Expression::Duration(_)
        | Expression::Integer(_)
        | Expression::Decimal(_)
        | Expression::ApproximateNumeric(_) => false,
    }
}
