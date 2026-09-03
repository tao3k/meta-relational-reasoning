//! Pure AST-to-IR normalization for ISO path search prefixes.
#![forbid(unsafe_code)]

use gql_ir::{
    NonNegativeIntegerSpecification as IrNonNegativeIntegerSpecification, PathMode as IrPathMode,
    PathPrefix as IrPathPrefix, PathSearch as IrPathSearch,
};

pub(crate) fn lower_path_prefix(prefix: &gql_ast::PathPrefix) -> IrPathPrefix {
    IrPathPrefix {
        search: prefix.search.as_ref().map(|search| match search {
            gql_ast::PathSearch::All => IrPathSearch::All,
            gql_ast::PathSearch::Any { count } => IrPathSearch::Any {
                count: count.as_ref().map(lower_path_count),
            },
            gql_ast::PathSearch::AllShortest => IrPathSearch::AllShortest,
            gql_ast::PathSearch::AnyShortest => IrPathSearch::AnyShortest,
            gql_ast::PathSearch::Shortest { count } => IrPathSearch::Shortest {
                count: lower_path_count(count),
            },
            gql_ast::PathSearch::ShortestGroups { count } => IrPathSearch::ShortestGroups {
                count: count.as_ref().map(lower_path_count),
            },
        }),
        mode: prefix.mode.map(|mode| match mode {
            gql_ast::PathMode::Walk => IrPathMode::Walk,
            gql_ast::PathMode::Trail => IrPathMode::Trail,
            gql_ast::PathMode::Acyclic => IrPathMode::Acyclic,
            gql_ast::PathMode::Simple => IrPathMode::Simple,
        }),
    }
}

fn lower_path_count(
    count: &gql_ast::NonNegativeIntegerSpecification,
) -> IrNonNegativeIntegerSpecification {
    match count {
        gql_ast::NonNegativeIntegerSpecification::Literal(value) => {
            IrNonNegativeIntegerSpecification::Literal(*value)
        }
        gql_ast::NonNegativeIntegerSpecification::Parameter(parameter) => {
            IrNonNegativeIntegerSpecification::Parameter(parameter.name.clone())
        }
    }
}
