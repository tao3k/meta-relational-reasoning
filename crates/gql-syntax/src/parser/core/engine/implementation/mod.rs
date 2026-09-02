//! Lossless parser implementation branch.

mod data_modification;
mod graph_type;
mod literals;
mod parser;
mod postfix_expression;
mod statements;

pub use parser::parse;
pub(in crate::parser) use parser::{Event, Parser, node};
