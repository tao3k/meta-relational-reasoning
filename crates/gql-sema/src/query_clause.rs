//! Shared structural queries over typed GQL clauses.
#![forbid(unsafe_code)]

use gql_ast::QueryClause;
use gql_source::Span;

pub(crate) fn query_clause_span(clause: &QueryClause) -> Span {
    match clause {
        QueryClause::Match(found) | QueryClause::OptionalMatch(found) => found.span,
        QueryClause::Where { span, .. }
        | QueryClause::Filter { span, .. }
        | QueryClause::For { span, .. }
        | QueryClause::Let { span, .. }
        | QueryClause::Return { span, .. }
        | QueryClause::Finish { span }
        | QueryClause::Union { span }
        | QueryClause::Limit { span, .. }
        | QueryClause::OrderBy { span, .. }
        | QueryClause::Offset { span, .. }
        | QueryClause::GroupBy { span, .. }
        | QueryClause::Insert { span, .. }
        | QueryClause::Set { span, .. }
        | QueryClause::Remove { span, .. }
        | QueryClause::Delete { span, .. } => *span,
    }
}
