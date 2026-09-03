//! Canonical admission for ISO RETURN bodies and FINISH terminals.
#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};

use gql_ast::{Expression, QueryClause, SetQuantifier};
use gql_ir::{
    Expression as IrExpression, Projection, QueryBlock, SetQuantifier as IrSetQuantifier,
};
use gql_source::Diagnostic;
use gql_types::ValueType;

type InferExpression = fn(&Expression, &HashMap<String, ValueType>) -> Option<ValueType>;
type LowerExpression = fn(
    &Expression,
    &HashMap<String, ValueType>,
    &mut Vec<Diagnostic>,
) -> Option<IrExpression>;

pub(crate) fn analyze_result_clause(
    clause: &QueryClause,
    block: &mut QueryBlock,
    bindings: &mut HashMap<String, ValueType>,
    binding_order: &[String],
    diagnostics: &mut Vec<Diagnostic>,
    infer_expression: InferExpression,
    lower_expression: LowerExpression,
) -> bool {
    match clause {
        QueryClause::Return {
            quantifier,
            all_bindings,
            projections,
            span,
        } => {
            block.projection_quantifier = canonical_quantifier(*quantifier);
            if *all_bindings {
                expand_all_bindings(block, bindings, binding_order, *span, diagnostics);
            }
            lower_explicit_projections(
                projections,
                block,
                bindings,
                diagnostics,
                infer_expression,
                lower_expression,
            );
            true
        }
        QueryClause::Finish { .. } => {
            block.is_finish = true;
            true
        }
        _ => false,
    }
}

fn canonical_quantifier(quantifier: Option<SetQuantifier>) -> IrSetQuantifier {
    match quantifier {
        Some(SetQuantifier::Distinct) => IrSetQuantifier::Distinct,
        Some(SetQuantifier::All) | None => IrSetQuantifier::All,
    }
}

fn expand_all_bindings(
    block: &mut QueryBlock,
    bindings: &HashMap<String, ValueType>,
    binding_order: &[String],
    span: gql_source::Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if binding_order.is_empty() {
        diagnostics.push(Diagnostic::error(
            "GQL-SEMA-RETURN-STAR-WITHOUT-BINDINGS",
            "RETURN * requires at least one visible binding",
            span,
        ));
    }
    block
        .projection
        .extend(binding_order.iter().filter_map(|name| {
            bindings.get(name).map(|value_type| Projection {
                expression: IrExpression::Binding(name.clone()),
                alias: Some(name.clone()),
                value_type: value_type.clone(),
            })
        }));
}

fn lower_explicit_projections(
    projections: &[gql_ast::ReturnProjection],
    block: &mut QueryBlock,
    bindings: &mut HashMap<String, ValueType>,
    diagnostics: &mut Vec<Diagnostic>,
    infer_expression: InferExpression,
    lower_expression: LowerExpression,
) {
    let mut aliases = HashSet::new();
    let mut result_bindings = Vec::new();
    for projection in projections {
        if let Some(alias) = &projection.alias
            && !aliases.insert(alias.canonical_text())
        {
            diagnostics.push(Diagnostic::error(
                "GQL-SEMA-DUPLICATE-PROJECTION-ALIAS",
                format!("projection alias `{}` is declared more than once", alias.text),
                alias.span,
            ));
            continue;
        }
        if let Some(expression) = lower_expression(&projection.expression, bindings, diagnostics) {
            let value_type =
                infer_expression(&projection.expression, bindings).unwrap_or(ValueType::Any);
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
