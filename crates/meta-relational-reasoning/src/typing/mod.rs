//! Catalog-bound expression, parameter, aggregation, and result typing.

mod catalog;
mod checker;
mod model;
mod rules;

pub(crate) use checker::type_query;
pub use model::{ExpressionType, ParameterType, QueryType, ResultField, StaticQueryTyping};
