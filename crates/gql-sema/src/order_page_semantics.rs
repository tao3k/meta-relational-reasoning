//! Pure AST-to-IR normalization for ISO ordering and pagination metadata.
#![forbid(unsafe_code)]

use gql_ir::{NonNegativeIntegerSpecification, NullOrdering, SortDirection};

pub(crate) fn lower_non_negative_integer_specification(
    value: &gql_ast::NonNegativeIntegerSpecification,
) -> NonNegativeIntegerSpecification {
    match value {
        gql_ast::NonNegativeIntegerSpecification::Literal(value) => {
            NonNegativeIntegerSpecification::Literal(*value)
        }
        gql_ast::NonNegativeIntegerSpecification::Parameter(parameter) => {
            NonNegativeIntegerSpecification::Parameter(parameter.name.clone())
        }
    }
}

pub(crate) const fn canonical_sort_direction(
    direction: Option<gql_ast::SortDirection>,
) -> SortDirection {
    match direction {
        None | Some(gql_ast::SortDirection::Ascending) => SortDirection::Ascending,
        Some(gql_ast::SortDirection::Descending) => SortDirection::Descending,
    }
}

pub(crate) const fn lower_null_ordering(
    ordering: Option<gql_ast::NullOrdering>,
) -> Option<NullOrdering> {
    match ordering {
        None => None,
        Some(gql_ast::NullOrdering::First) => Some(NullOrdering::First),
        Some(gql_ast::NullOrdering::Last) => Some(NullOrdering::Last),
    }
}
