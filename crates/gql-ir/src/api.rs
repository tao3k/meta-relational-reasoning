//! Internal IR shape for resolved query plans and bindings.

use gql_catalog::RelationName;
use gql_types::{Value, ValueType};

#[derive(Clone, Debug, Eq, PartialEq)]
/// Bound identifier with its inferred type.
pub struct Binding {
    /// Identifier name.
    pub name: String,
    /// Inferred runtime value type.
    pub value_type: ValueType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Scan over a relation with its projected bindings.
pub struct RelationScan {
    /// Target relation.
    pub relation: RelationName,
    /// Bound names emitted by the scan.
    pub bindings: Vec<Binding>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct QueryBlock {
    /// Table-like scans in this query block.
    pub scans: Vec<RelationScan>,
    /// Predicates applied to the block.
    pub predicates: Vec<Predicate>,
    /// Projection bindings in final shape.
    pub projection: Vec<Binding>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Predicate {
    /// Equality predicate over a relation binding.
    Equals(Binding, Value),
    /// Null-check predicate over a relation binding.
    IsNull(Binding),
}
