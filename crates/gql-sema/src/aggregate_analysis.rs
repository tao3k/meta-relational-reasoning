//! Recursive aggregate detection over backend-neutral expression IR.
#![forbid(unsafe_code)]

use gql_ast::{AggregateFunction, SetQuantifier};
use gql_ir::{
    AggregateFunction as IrAggregateFunction, Expression, SetQuantifier as IrSetQuantifier,
};
use gql_source::{Diagnostic, Span};
use gql_types::ValueType;

use crate::type_inference::is_numeric_type;

pub(crate) fn lower_aggregate_call(
    function: AggregateFunction,
    quantifier: Option<SetQuantifier>,
    arguments: Vec<Expression>,
    argument_types: &[ValueType],
    count_star: bool,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Expression> {
    let expected_arity = if matches!(
        function,
        AggregateFunction::PercentileContinuous | AggregateFunction::PercentileDiscrete
    ) {
        2
    } else if count_star {
        0
    } else {
        1
    };
    if arguments.len() != expected_arity {
        diagnostics.push(Diagnostic::error(
            "GQL-SEMA-AGGREGATE-ARITY",
            "aggregate argument count does not match its ISO GQL function family",
            span,
        ));
        return None;
    }
    if count_star && function != AggregateFunction::Count {
        diagnostics.push(Diagnostic::error(
            "GQL-SEMA-AGGREGATE-STAR",
            "only COUNT admits the `*` row-count form",
            span,
        ));
        return None;
    }
    let requires_numeric = matches!(
        function,
        AggregateFunction::Average
            | AggregateFunction::Sum
            | AggregateFunction::StandardDeviationSample
            | AggregateFunction::StandardDeviationPopulation
            | AggregateFunction::PercentileContinuous
            | AggregateFunction::PercentileDiscrete
    );
    if requires_numeric
        && argument_types
            .iter()
            .any(|kind| kind != &ValueType::Any && !is_numeric_type(kind))
    {
        diagnostics.push(Diagnostic::error(
            "GQL-SEMA-AGGREGATE-NUMERIC-OPERAND",
            "numeric aggregate functions require numeric operands",
            span,
        ));
        return None;
    }
    Some(Expression::Aggregate {
        function: match function {
            AggregateFunction::Average => IrAggregateFunction::Average,
            AggregateFunction::Count => IrAggregateFunction::Count,
            AggregateFunction::Maximum => IrAggregateFunction::Maximum,
            AggregateFunction::Minimum => IrAggregateFunction::Minimum,
            AggregateFunction::Sum => IrAggregateFunction::Sum,
            AggregateFunction::CollectList => IrAggregateFunction::CollectList,
            AggregateFunction::StandardDeviationSample => {
                IrAggregateFunction::StandardDeviationSample
            }
            AggregateFunction::StandardDeviationPopulation => {
                IrAggregateFunction::StandardDeviationPopulation
            }
            AggregateFunction::PercentileContinuous => IrAggregateFunction::PercentileContinuous,
            AggregateFunction::PercentileDiscrete => IrAggregateFunction::PercentileDiscrete,
        },
        quantifier: quantifier.map(|quantifier| match quantifier {
            SetQuantifier::All => IrSetQuantifier::All,
            SetQuantifier::Distinct => IrSetQuantifier::Distinct,
        }),
        arguments,
        count_star,
    })
}

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
        | Expression::IsLabeled { operand: base, .. }
        | Expression::IsTyped { operand: base, .. }
        | Expression::IsDirected { edge: base, .. }
        | Expression::PropertyExists { element: base, .. } => contains_aggregate(base),
        Expression::IsEndpoint { node, edge, .. } => {
            contains_aggregate(node) || contains_aggregate(edge)
        }
        Expression::ElementIdentity { elements, .. } => elements.iter().any(contains_aggregate),
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
        | Expression::Parameter(_)
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
