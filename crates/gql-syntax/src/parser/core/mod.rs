//! Parser-core branch boundary.
#![forbid(unsafe_code)]

mod boundaries;
mod diagnostic_names;
mod engine;
mod label_expression;

pub(in crate::parser) use diagnostic_names::keyword_name;
pub use engine::parse;
pub(in crate::parser) use engine::{Event, Parser, node};
