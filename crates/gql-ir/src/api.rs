//! Canonical graph-semantic IR for the supported ISO GQL query slice.

use gql_types::ValueType;

#[derive(Clone, Debug, Eq, PartialEq)]
/// Binding identity with semantic type information.
pub struct Binding {
    /// Stable source binding name.
    pub name: String,
    /// Inferred or catalog-derived value type.
    pub value_type: ValueType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Expression in the canonical, backend-neutral query representation.
pub enum Expression {
    /// Reference to a bound graph or LET value.
    Binding(String),
    /// String literal.
    String(String),
    /// Integer literal.
    Integer(i64),
    /// Unary expression.
    Unary {
        /// Unary operator.
        operator: UnaryOperator,
        /// Operand.
        operand: Box<Expression>,
    },
    /// Binary expression.
    Binary {
        /// Binary operator.
        operator: BinaryOperator,
        /// Left operand.
        left: Box<Expression>,
        /// Right operand.
        right: Box<Expression>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Unary expression operator.
pub enum UnaryOperator {
    /// Boolean negation.
    Not,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Binary expression operator.
pub enum BinaryOperator {
    /// Equality comparison.
    Equals,
    /// Inequality comparison.
    NotEquals,
    /// Less-than comparison.
    LessThan,
    /// Less-than-or-equal comparison.
    LessThanOrEqual,
    /// Greater-than comparison.
    GreaterThan,
    /// Greater-than-or-equal comparison.
    GreaterThanOrEqual,
    /// Boolean conjunction.
    And,
    /// Boolean disjunction.
    Or,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// First-class node pattern in the canonical graph-semantic form.
pub struct NodePattern {
    /// Source binding name when present.
    pub binding: Option<String>,
    /// Label or node-type constraints.
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
/// First-class edge pattern in the canonical graph-semantic form.
pub struct EdgePattern {
    /// Edge label or edge-type constraints.
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
    /// Nested path pattern.
    Path(PathPattern),
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// First-class path pattern.
pub struct PathPattern {
    /// Ordered graph pattern elements.
    pub elements: Vec<GraphPatternElement>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Graph pattern for a query block.
pub struct GraphPattern {
    /// Ordered pattern elements in source order.
    pub elements: Vec<GraphPatternElement>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// A semantically resolved LET binding.
pub struct LetBinding {
    /// Binding identity.
    pub binding: Binding,
    /// Bound value expression.
    pub value: Expression,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// A projection expression in source order.
pub struct Projection {
    /// Projected expression.
    pub expression: Expression,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
/// Canonical graph-semantic query block before backend planning.
pub struct QueryBlock {
    /// Graph pattern, if the query contains a MATCH clause.
    pub graph: Option<GraphPattern>,
    /// WHERE/filter expressions in source order.
    pub filters: Vec<Expression>,
    /// LET bindings in source order.
    pub let_bindings: Vec<LetBinding>,
    /// RETURN projection in source order.
    pub projection: Vec<Projection>,
}
