//! Internal IR surface shared by analysis and compiler passes.
#![forbid(unsafe_code)]

mod api;

pub use api::{
    Binding, EdgeDirection, EdgePattern, GraphPattern, GraphPatternElement, NodePattern,
    PathPattern, Predicate, QueryBlock,
};
