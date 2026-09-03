//! Semantic owner for ISO null and truth-value predicate tests.

use gql_ast::{ElementIdentityKind, EndpointKind, LabelExpression, TruthValue};
use gql_ir::{
    ElementIdentityKind as IrElementIdentityKind, EndpointKind as IrEndpointKind, Expression,
    LabelExpression as IrLabelExpression, UnaryOperator,
};
use gql_source::{Diagnostic, Span};
use gql_types::ValueType;

pub(crate) fn lower_null_predicate(operand: Expression, negated: bool) -> Expression {
    Expression::Unary {
        operator: if negated {
            UnaryOperator::IsNotNull
        } else {
            UnaryOperator::IsNull
        },
        operand: Box::new(operand),
    }
}

pub(crate) fn lower_label_expression(expression: &LabelExpression) -> IrLabelExpression {
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

pub(crate) fn lower_directed_predicate(
    edge: Expression,
    edge_type: ValueType,
    negated: bool,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Expression> {
    if edge_type != ValueType::Edge {
        diagnostics.push(Diagnostic::error(
            "GQL-SEMA-DIRECTED-PREDICATE-NON-EDGE",
            "IS DIRECTED requires an edge variable reference",
            span,
        ));
        return None;
    }
    Some(Expression::IsDirected {
        edge: Box::new(edge),
        negated,
    })
}

pub(crate) fn lower_endpoint_predicate(
    node: (Expression, ValueType),
    edge: (Expression, ValueType),
    endpoint: EndpointKind,
    negated: bool,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Expression> {
    if node.1 != ValueType::Node || edge.1 != ValueType::Edge {
        diagnostics.push(Diagnostic::error(
            "GQL-SEMA-ENDPOINT-PREDICATE-KIND",
            "SOURCE/DESTINATION requires a node reference followed by an edge reference",
            span,
        ));
        return None;
    }
    Some(Expression::IsEndpoint {
        node: Box::new(node.0),
        edge: Box::new(edge.0),
        endpoint: match endpoint {
            EndpointKind::Source => IrEndpointKind::Source,
            EndpointKind::Destination => IrEndpointKind::Destination,
        },
        negated,
    })
}

pub(crate) fn lower_element_identity_predicate(
    kind: ElementIdentityKind,
    elements: Vec<(Expression, ValueType)>,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Expression> {
    if elements
        .iter()
        .any(|(_, value_type)| !matches!(value_type, ValueType::Node | ValueType::Edge))
    {
        diagnostics.push(Diagnostic::error(
            "GQL-SEMA-ELEMENT-IDENTITY-PREDICATE-NON-ELEMENT",
            "ALL_DIFFERENT and SAME require graph-element variable references",
            span,
        ));
        return None;
    }
    Some(Expression::ElementIdentity {
        kind: match kind {
            ElementIdentityKind::AllDifferent => IrElementIdentityKind::AllDifferent,
            ElementIdentityKind::Same => IrElementIdentityKind::Same,
        },
        elements: elements
            .into_iter()
            .map(|(expression, _)| expression)
            .collect(),
    })
}

pub(crate) fn lower_property_exists_predicate(
    element: Expression,
    element_type: ValueType,
    property: String,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Expression> {
    if !matches!(element_type, ValueType::Node | ValueType::Edge) {
        diagnostics.push(Diagnostic::error(
            "GQL-SEMA-PROPERTY-EXISTS-NON-ELEMENT",
            "PROPERTY_EXISTS requires a graph-element variable reference",
            span,
        ));
        return None;
    }
    Some(Expression::PropertyExists {
        element: Box::new(element),
        property,
    })
}

pub(crate) fn label_predicate_operand_is_valid(
    operand_type: ValueType,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if matches!(operand_type, ValueType::Node | ValueType::Edge) {
        return true;
    }
    diagnostics.push(Diagnostic::error(
        "GQL-SEMA-LABEL-PREDICATE-NON-ELEMENT",
        "IS LABELED requires a node or edge expression",
        span,
    ));
    false
}

pub(crate) fn lower_truth_predicate(
    operand: Expression,
    operand_type: ValueType,
    value: TruthValue,
    negated: bool,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Expression> {
    if !matches!(
        operand_type,
        ValueType::Boolean | ValueType::Null | ValueType::Any
    ) {
        diagnostics.push(Diagnostic::error(
            "GQL-SEMA-TRUTH-PREDICATE-OPERAND",
            "truth-value predicates require a boolean or null operand",
            span,
        ));
        return None;
    }
    let operator = match (value, negated) {
        (TruthValue::True, false) => UnaryOperator::IsTrue,
        (TruthValue::True, true) => UnaryOperator::IsNotTrue,
        (TruthValue::False, false) => UnaryOperator::IsFalse,
        (TruthValue::False, true) => UnaryOperator::IsNotFalse,
        (TruthValue::Unknown, false) => UnaryOperator::IsUnknown,
        (TruthValue::Unknown, true) => UnaryOperator::IsNotUnknown,
    };
    Some(Expression::Unary {
        operator,
        operand: Box::new(operand),
    })
}
