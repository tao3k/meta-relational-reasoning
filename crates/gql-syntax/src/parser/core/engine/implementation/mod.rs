//! Lossless parser implementation branch.

mod data_modification;
mod graph_type;
mod integer_specification;
mod literals;
mod parser;
mod path_prefix;
mod postfix_expression;
mod predicate_expression;
mod primitive_query;
mod statements;

pub use parser::parse;
pub(in crate::parser) use parser::{Event, Parser, node};
