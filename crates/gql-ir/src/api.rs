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
    /// Boolean literal.
    Boolean(bool),
    /// Null literal.
    Null,
    /// String literal.
    String(String),
    /// Integer literal.
    Integer(i64),
    /// Decimal literal in canonical lexical form.
    Decimal(String),
    /// List value in source order.
    List(Vec<Expression>),
    /// Collection subscript access.
    Subscript {
        /// Collection expression.
        base: Box<Expression>,
        /// Integer index expression.
        index: Box<Expression>,
    },
    /// Property access on a graph or value expression.
    PropertyAccess {
        /// Base expression.
        base: Box<Expression>,
        /// Property name.
        property: String,
    },
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
    /// Ordered simple or searched CASE expression.
    Case {
        /// Optional operand for simple CASE; absent for searched CASE.
        operand: Option<Box<Expression>>,
        /// Ordered WHEN/THEN branches.
        branches: Vec<CaseBranch>,
        /// Optional ELSE result.
        else_result: Option<Box<Expression>>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// One canonical CASE branch.
pub struct CaseBranch {
    /// Simple-CASE match value or searched-CASE predicate.
    pub condition: Expression,
    /// Result selected by this branch.
    pub result: Expression,
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
    /// Numeric addition.
    Add,
    /// Numeric subtraction.
    Subtract,
    /// Numeric multiplication.
    Multiply,
    /// Numeric division.
    Divide,
    /// Numeric modulo.
    Modulo,
    /// Collection membership predicate.
    In,
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
    /// Property constraints in source order.
    pub properties: Vec<PropertyConstraint>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// A property constraint attached to a graph pattern element.
pub struct PropertyConstraint {
    /// Property name.
    pub key: String,
    /// Required property value expression.
    pub value: Expression,
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
    /// Source binding name when present.
    pub binding: Option<String>,
    /// Edge label or edge-type constraints.
    pub labels: Vec<String>,
    /// Property constraints in source order.
    pub properties: Vec<PropertyConstraint>,
    /// Edge direction.
    pub direction: EdgeDirection,
    /// Optional bounded path repetition.
    pub quantifier: Option<PathQuantifier>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Bounded repetition attached to an edge pattern.
pub struct PathQuantifier {
    /// Minimum number of traversals.
    pub min: u32,
    /// Optional inclusive maximum number of traversals.
    pub max: Option<u32>,
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
    /// Optional name bound to the complete path value.
    pub binding: Option<String>,
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
    /// Optional output alias.
    pub alias: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Canonical ordering direction.
pub enum SortDirection {
    /// Ascending sort order.
    Ascending,
    /// Descending sort order.
    Descending,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Canonical sort key for one query branch.
pub struct SortKey {
    /// Sort expression.
    pub expression: Expression,
    /// Sort direction.
    pub direction: SortDirection,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
/// Canonical graph-semantic query block before backend planning.
pub struct QueryBlock {
    /// Graph pattern, if the query contains a MATCH clause.
    pub graph: Option<GraphPattern>,
    /// Optional graph patterns evaluated after the mandatory graph pattern.
    pub optional_graphs: Vec<GraphPattern>,
    /// WHERE/filter expressions in source order.
    pub filters: Vec<Expression>,
    /// LET bindings in source order.
    pub let_bindings: Vec<LetBinding>,
    /// RETURN projection in source order.
    pub projection: Vec<Projection>,
    /// Query blocks composed by ISO `UNION`, in source order.
    pub union_branches: Vec<QueryBlock>,
    /// Maximum number of rows preserved by this query block.
    pub limit: Option<u64>,
    /// Ordering keys evaluated before pagination.
    pub order_by: Vec<SortKey>,
    /// Number of rows skipped before applying the limit.
    pub offset: Option<u64>,
}
