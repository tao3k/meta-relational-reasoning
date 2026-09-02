//! Canonical ordered-record lowering and duplicate-field admission.
#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};

use gql_ast::RecordField;
use gql_ir::{Expression as IrExpression, RecordField as IrRecordField};
use gql_source::Diagnostic;
use gql_types::ValueType;

use crate::api::lower_expression;

pub(crate) fn lower_record_expression(
    fields: &[RecordField],
    bindings: &HashMap<String, ValueType>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<IrExpression> {
    let mut names = HashSet::with_capacity(fields.len());
    let mut lowered = Vec::with_capacity(fields.len());
    for field in fields {
        let name = field.name.canonical_text();
        if !names.insert(name.clone()) {
            diagnostics.push(Diagnostic::error(
                "GQL-SEMA-DUPLICATE-RECORD-FIELD",
                format!(
                    "record field `{}` is declared more than once",
                    field.name.text
                ),
                field.span,
            ));
            return None;
        }
        lowered.push(IrRecordField {
            name,
            value: lower_expression(&field.value, bindings, diagnostics)?,
        });
    }
    Some(IrExpression::Record(lowered))
}
