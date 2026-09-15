//! Shared result-clause lowering helpers.

use std::collections::HashSet;

use gql_ast as ast;
use mrr_query::{Binding, GraphPattern, PageValue, Parameter};

use crate::FrontendError;

pub(crate) fn lower_page_value(
    value: &ast::NonNegativeIntegerSpecification,
) -> Result<PageValue, FrontendError> {
    Ok(match value {
        ast::NonNegativeIntegerSpecification::Literal(value) => PageValue::Literal(*value),
        ast::NonNegativeIntegerSpecification::Parameter(parameter) => {
            PageValue::Parameter(Parameter::new(parameter.name.clone())?)
        }
    })
}

pub(crate) fn visible_bindings(graph: &GraphPattern) -> Vec<Binding> {
    let mut seen = HashSet::new();
    graph
        .paths()
        .iter()
        .flat_map(|path| {
            std::iter::once(path.start().binding()).chain(path.segments().iter().flat_map(
                |segment| {
                    segment
                        .relation()
                        .binding()
                        .into_iter()
                        .chain(std::iter::once(segment.node().binding()))
                },
            ))
        })
        .filter(|binding| seen.insert((*binding).clone()))
        .cloned()
        .collect()
}
