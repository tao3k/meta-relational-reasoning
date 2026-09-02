//! Lossless parser engine branch boundary.

mod implementation;
mod query_result;

pub use implementation::parse;
pub(in crate::parser) use implementation::{Event, Parser, node};
