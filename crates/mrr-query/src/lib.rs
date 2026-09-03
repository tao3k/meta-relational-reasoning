//! Language-neutral canonical query IR and relational goals.
#![forbid(unsafe_code)]

mod api;
pub use api::{
    Aggregation, AggregationFunction, Atom, BinaryOperator, Binding, Direction, Expression, Filter,
    GraphPattern, META_QUERY_SCHEMA, MetaQueryIr, NodePattern, Ordering, Parameter, PathPattern,
    PathSegment, Projection, PropertyKey, QueryIrError, RelationPattern, RelationalGoal,
    RelationalGoalError, SetQuantifier, SortDirection, Term, UnaryOperator, Variable,
};
pub use mrr_identity::{EntityId, QueryId, QueryOperatorId, RelationId};
pub use mrr_relation::Value;

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
