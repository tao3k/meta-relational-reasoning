//! Canonical FILTER and FOR semantic admission.

#![forbid(unsafe_code)]

use std::collections::HashMap;

use gql_ast::{Expression, ForOrdinalityKind, QueryClause};
use gql_ir::{
    Binding as IrBinding, Expression as IrExpression, ForBinding as IrForBinding,
    ForPositionBinding as IrForPositionBinding, ForPositionKind as IrForPositionKind, QueryBlock,
};
use gql_source::Diagnostic;
use gql_types::ValueType;

type ExpressionType = fn(&Expression, &HashMap<String, ValueType>) -> Option<ValueType>;
type LowerExpression =
    fn(&Expression, &HashMap<String, ValueType>, &mut Vec<Diagnostic>) -> Option<IrExpression>;

pub(crate) fn analyze_primitive_query_clause(
    clause: &QueryClause,
    block: &mut QueryBlock,
    bindings: &mut HashMap<String, ValueType>,
    diagnostics: &mut Vec<Diagnostic>,
    expression_type: ExpressionType,
    lower_expression: LowerExpression,
) {
    match clause {
        QueryClause::Filter { expression, span } => {
            if expression_type(expression, bindings).is_some_and(|value_type| {
                !matches!(value_type, ValueType::Boolean | ValueType::Any)
            }) {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-FILTER-NOT-BOOLEAN",
                    "FILTER search condition must have Boolean type",
                    *span,
                ));
                return;
            }
            if let Some(filter) = lower_expression(expression, bindings, diagnostics) {
                block.filters.push(filter);
            }
        }
        QueryClause::For { item, .. } => {
            let source_type = expression_type(&item.source, bindings);
            if source_type
                .as_ref()
                .is_some_and(|value_type| !matches!(value_type, ValueType::List | ValueType::Any))
            {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-FOR-SOURCE-NOT-LIST",
                    "FOR source expression must have List or unknown type",
                    item.span,
                ));
            }
            let Some(source) = lower_expression(&item.source, bindings, diagnostics) else {
                return;
            };
            let canonical_binding = item.binding.canonical_text();
            if bindings.contains_key(&canonical_binding) {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-FOR-DUPLICATE-BINDING",
                    format!("FOR binding `{}` is already defined", item.binding.text),
                    item.binding.span,
                ));
                return;
            }
            let binding = IrBinding {
                name: canonical_binding.clone(),
                value_type: ValueType::Any,
            };
            bindings.insert(canonical_binding, ValueType::Any);

            let position = item.ordinality.as_ref().and_then(|position| {
                let canonical_position = position.binding.canonical_text();
                if bindings.contains_key(&canonical_position) {
                    diagnostics.push(Diagnostic::error(
                        "GQL-SEMA-FOR-DUPLICATE-POSITION-BINDING",
                        format!(
                            "FOR position binding `{}` is already defined",
                            position.binding.text
                        ),
                        position.binding.span,
                    ));
                    return None;
                }
                let binding = IrBinding {
                    name: canonical_position.clone(),
                    value_type: ValueType::Integer,
                };
                bindings.insert(canonical_position, ValueType::Integer);
                Some(IrForPositionBinding {
                    kind: match position.kind {
                        ForOrdinalityKind::Ordinality => IrForPositionKind::Ordinality,
                        ForOrdinalityKind::Offset => IrForPositionKind::Offset,
                    },
                    binding,
                })
            });
            block.for_bindings.push(IrForBinding {
                binding,
                source,
                position,
            });
        }
        _ => unreachable!("primitive-query semantic dispatch requires FILTER or FOR"),
    }
}
