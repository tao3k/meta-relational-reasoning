//! Internal IR surface shared by analysis and compiler passes.
#![forbid(unsafe_code)]

mod api;

pub use api::{
    BinaryOperator, Binding, CaseBranch, EdgeDirection, EdgePattern, Expression, GraphPattern,
    GraphPatternElement, LetBinding, NodePattern, PathPattern, PathQuantifier, Projection,
    PropertyConstraint, QueryBlock, SortDirection, SortKey, UnaryOperator,
};
