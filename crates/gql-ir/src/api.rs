//! Internal IR shape for resolved query plans and bindings.

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
/// First-class node pattern in the canonical graph-syntax intermediate form.
pub struct NodePattern {
    /// Binding name when present.
    pub binding: Option<String>,
    /// Labels or type-constraint tokens attached to the node.
    pub labels: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Canonical edge direction used by graph patterns.
pub enum EdgeDirection {
    /// Outgoing edge direction.
    Out,
    /// Incoming edge direction.
    In,
    /// Unspecified direction.
    Undirected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// First-class edge pattern in the canonical graph-syntax intermediate form.
pub struct EdgePattern {
    /// Candidate relation labels bound to the edge.
    pub labels: Vec<String>,
    /// Edge direction.
    pub direction: EdgeDirection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Recursive graph pattern element.
pub enum GraphPatternElement {
    /// Node pattern element.
    Node(NodePattern),
    /// Edge pattern element.
    Edge(EdgePattern),
    /// Path pattern element.
    Path(PathPattern),
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// First-class path pattern.
pub struct PathPattern {
    /// Ordered graph pattern elements in the path.
    pub elements: Vec<GraphPatternElement>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Canonical graph pattern for a query block.
pub struct GraphPattern {
    /// Ordered pattern elements in source order.
    pub elements: Vec<GraphPatternElement>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct QueryBlock {
    /// Primary canonical graph pattern representation.
    pub graph: Option<GraphPattern>,
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
