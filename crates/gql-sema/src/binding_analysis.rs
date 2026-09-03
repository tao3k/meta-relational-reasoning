//! Graph-pattern binding registration and kind-conflict analysis.
#![forbid(unsafe_code)]

use std::collections::HashMap;

use gql_ast::{GraphPattern, Identifier, PathPattern, PatternElement};
use gql_source::Diagnostic;
use gql_types::ValueType;

pub(crate) fn register_pattern_bindings(
    pattern: &GraphPattern,
    bindings: &mut HashMap<String, ValueType>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    register_bindings(collect_pattern_bindings(pattern), bindings, diagnostics);
}

pub(crate) fn register_path_bindings(
    path: &PathPattern,
    bindings: &mut HashMap<String, ValueType>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<String> {
    let mut found = Vec::new();
    if let Some(binding) = &path.binding {
        found.push((binding.clone(), ValueType::Path));
    }
    collect_pattern_bindings_inner(&path.elements, &mut found);
    let mut admitted_order = Vec::new();
    for (binding, _) in &found {
        let canonical = binding.canonical_text();
        if !bindings.contains_key(&canonical) && !admitted_order.contains(&canonical) {
            admitted_order.push(canonical);
        }
    }
    register_bindings(found, bindings, diagnostics);
    admitted_order
}

fn register_bindings(
    found: Vec<(Identifier, ValueType)>,
    bindings: &mut HashMap<String, ValueType>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (binding, value_type) in found {
        let canonical = binding.canonical_text();
        if let Some(existing) = bindings.get(&canonical) {
            if existing != &value_type {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-BINDING-KIND-CONFLICT",
                    format!(
                        "binding `{}` cannot denote both {existing:?} and {value_type:?}",
                        binding.text
                    ),
                    binding.span,
                ));
            }
        } else {
            bindings.insert(canonical, value_type);
        }
    }
}

fn collect_pattern_bindings(pattern: &GraphPattern) -> Vec<(Identifier, ValueType)> {
    let mut bindings = Vec::new();
    collect_pattern_bindings_inner(&pattern.elements, &mut bindings);
    bindings
}

fn collect_pattern_bindings_inner(
    elements: &[PatternElement],
    bindings: &mut Vec<(Identifier, ValueType)>,
) {
    for element in elements {
        match element {
            PatternElement::Node(node) => {
                if let Some(binding) = &node.binding {
                    bindings.push((binding.clone(), ValueType::Node));
                }
            }
            PatternElement::Edge(edge) => {
                if let Some(binding) = &edge.binding {
                    bindings.push((binding.clone(), ValueType::Edge));
                }
            }
            PatternElement::Path(path) => {
                if let Some(binding) = &path.binding {
                    bindings.push((binding.clone(), ValueType::Path));
                }
                collect_pattern_bindings_inner(&path.elements, bindings);
            }
        }
    }
}
