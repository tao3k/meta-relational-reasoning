//! Canonical graph-semantic IR for the supported ISO GQL query slice.

use gql_types::ValueType;

#[derive(Clone, Debug, Eq, PartialEq)]
/// Backend-neutral catalog command admitted by semantic analysis.
pub enum CatalogCommand {
    /// Create one schema using its canonical identifier.
    CreateSchema { name: CatalogObjectName },
    /// Drop one schema using its canonical identifier.
    DropSchema { name: CatalogObjectName },
    /// Create one graph with a canonical graph-type specification.
    CreateGraph {
        name: CatalogObjectName,
        graph_type: GraphTypeSpecification,
        policy: CatalogCreatePolicy,
    },
    /// Drop one graph using its canonical identifier.
    DropGraph {
        name: CatalogObjectName,
        policy: CatalogDropPolicy,
    },
    /// Create one graph type from a canonical source identity.
    CreateGraphType {
        name: CatalogObjectName,
        source: GraphTypeSource,
        policy: CatalogCreatePolicy,
    },
    /// Drop one graph type using its canonical identifier.
    DropGraphType {
        name: CatalogObjectName,
        policy: CatalogDropPolicy,
    },
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// Canonical source-ordered catalog identity.
pub struct CatalogObjectName {
    pub parts: Vec<String>,
}

impl CatalogObjectName {
    /// Dotted identity used by catalog lookup without losing its segments.
    #[must_use]
    pub fn dotted(&self) -> String {
        self.parts.join(".")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Conflict behavior preserved by CREATE catalog intents.
pub enum CatalogCreatePolicy {
    Error,
    IfNotExists,
    OrReplace,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Missing-object behavior preserved by DROP catalog intents.
pub enum CatalogDropPolicy {
    Error,
    IfExists,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Backend-neutral graph type specification for catalog admission.
pub enum GraphTypeSpecification {
    /// An open graph accepting any property-graph type.
    Any { typed: bool },
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Canonical source used to derive a graph type.
pub enum GraphTypeSource {
    CopyOf {
        graph_type: CatalogObjectName,
    },
    LikeGraph {
        graph: CatalogObjectName,
    },
    Nested {
        node_types: Vec<NodeTypeSpecification>,
        edge_types: Vec<EdgeTypeSpecification>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Backend-neutral node type declaration.
pub struct NodeTypeSpecification {
    pub name: Option<String>,
    pub alias: Option<String>,
    pub key_labels: Option<Vec<String>>,
    pub labels: Vec<String>,
    pub properties: Vec<PropertyType>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Backend-neutral edge type with resolved logical endpoints.
pub struct EdgeTypeSpecification {
    pub name: Option<String>,
    pub source: NodeTypeReference,
    pub destination: NodeTypeReference,
    pub direction: EdgeDirection,
    pub key_labels: Option<Vec<String>>,
    pub labels: Vec<String>,
    pub properties: Vec<PropertyType>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Canonical endpoint reference for one edge type.
pub enum NodeTypeReference {
    Alias(String),
    Inline {
        key_labels: Option<Vec<String>>,
        labels: Vec<String>,
        properties: Vec<PropertyType>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// One canonical property type in source order.
pub struct PropertyType {
    pub name: String,
    pub value_type: DeclaredValueType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Canonical backend-neutral value type declared by graph-type DDL.
pub struct DeclaredValueType {
    pub form: DeclaredValueTypeForm,
    pub non_null: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Canonical structural form of a declared value type.
pub enum DeclaredValueTypeForm {
    Named {
        name: String,
        parameters: Vec<DeclaredTypeParameter>,
    },
    List {
        element: Option<Box<DeclaredValueType>>,
        max_length: Option<u64>,
    },
    Record {
        open: bool,
        fields: Vec<PropertyType>,
    },
    DynamicUnion {
        property_values: bool,
        members: Option<Vec<DeclaredValueType>>,
    },
    Reference {
        kind: ReferenceValueTypeKind,
        open: bool,
        property_graph: bool,
        specification: Option<Box<ClosedReferenceTypeSpecification>>,
        fields: Vec<PropertyType>,
    },
    Union(Vec<DeclaredValueType>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Canonical reference-value category.
pub enum ReferenceValueTypeKind {
    Graph,
    BindingTable,
    Node,
    Edge,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Canonical closed descriptor carried by a graph, node, or edge reference type.
pub enum ClosedReferenceTypeSpecification {
    Graph {
        node_types: Vec<NodeTypeSpecification>,
        edge_types: Vec<EdgeTypeSpecification>,
    },
    Node(NodeTypeSpecification),
    Edge(EdgeTypeSpecification),
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Canonical parameter attached to a declared value type.
pub enum DeclaredTypeParameter {
    Unsigned(u64),
    DurationQualifier { from: String, to: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Backend-neutral named procedure invocation.
pub struct ProcedureCommand {
    /// Canonical procedure identity.
    pub name: String,
    /// Source-ordered canonical arguments.
    pub arguments: Vec<Expression>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Transaction access mode carried by a start command.
pub enum TransactionAccessMode {
    ReadOnly,
    ReadWrite,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Backend-neutral transaction control intent.
pub enum TransactionCommand {
    Start {
        access_mode: Option<TransactionAccessMode>,
    },
    Commit,
    Rollback,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Backend-neutral session control intent.
pub enum SessionCommand {
    SetSchema { name: String },
    ResetSchema,
    Close,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Binding identity with semantic type information.
pub struct Binding {
    /// Stable source binding name.
    pub name: String,
    /// Inferred or catalog-derived value type.
    pub value_type: ValueType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// One ordered field in a canonical record literal.
pub struct RecordField {
    pub name: String,
    pub value: Expression,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Expression in the canonical, backend-neutral query representation.
pub enum Expression {
    /// Reference to a bound graph or LET value.
    Binding(String),
    /// Dynamic value supplied through one general parameter reference.
    Parameter(String),
    /// Boolean literal.
    Boolean(bool),
    /// Null literal.
    Null,
    /// String literal.
    String(String),
    /// Byte-string literal.
    ByteString(Vec<u8>),
    /// Calendar date literal in canonical lexical form.
    Date(String),
    /// Wall-clock time literal in canonical lexical form.
    Time(String),
    /// Combined date and time literal in canonical lexical form.
    Timestamp(String),
    /// ISO duration literal in canonical lexical form.
    Duration(String),
    /// Integer literal.
    Integer(i64),
    /// Decimal literal in canonical lexical form.
    Decimal(String),
    /// Approximate numeric literal in canonical lexical form, including precision suffix.
    ApproximateNumeric(String),
    /// List value in source order.
    List(Vec<Expression>),
    /// Record value preserving source field order.
    Record(Vec<RecordField>),
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
    /// Aggregate evaluated over one grouping partition.
    Aggregate {
        /// Canonical aggregate operator.
        function: AggregateFunction,
        /// Explicit `ALL` or `DISTINCT`; absence preserves the ISO default.
        quantifier: Option<SetQuantifier>,
        /// Source-ordered aggregate arguments.
        arguments: Vec<Expression>,
        /// Whether this is the unique `COUNT(*)` row-count form.
        count_star: bool,
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
    /// Label predicate over a graph element.
    IsLabeled {
        /// Graph-element expression being tested.
        operand: Box<Expression>,
        /// Canonical label algebra.
        label: LabelExpression,
        /// Whether `IS NOT LABELED` was requested.
        negated: bool,
    },
    /// Runtime value-type predicate over one canonical declared type.
    IsTyped {
        /// Value expression being tested.
        operand: Box<Expression>,
        /// Canonical backend-neutral value-type descriptor.
        value_type: DeclaredValueType,
        /// Whether `IS NOT TYPED` / `IS NOT ::` was requested.
        negated: bool,
    },
    /// Whether an edge has a directed orientation.
    IsDirected {
        edge: Box<Expression>,
        negated: bool,
    },
    /// Whether a node is the selected endpoint of an edge.
    IsEndpoint {
        node: Box<Expression>,
        edge: Box<Expression>,
        endpoint: EndpointKind,
        negated: bool,
    },
    /// N-ary identity relation over graph elements in source order.
    ElementIdentity {
        kind: ElementIdentityKind,
        elements: Vec<Expression>,
    },
    /// Whether a graph element owns a named property.
    PropertyExists {
        element: Box<Expression>,
        property: String,
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

/// Canonical graph endpoint role.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndpointKind {
    Source,
    Destination,
}

/// Canonical n-ary graph-element identity relation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElementIdentityKind {
    AllDifferent,
    Same,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Aggregate operators admitted by the query-syntax MVP.
pub enum AggregateFunction {
    /// Count rows or non-null argument values.
    Count,
    Average,
    Maximum,
    Minimum,
    Sum,
    CollectList,
    StandardDeviationSample,
    StandardDeviationPopulation,
    PercentileContinuous,
    PercentileDiscrete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Set quantifier retained by canonical aggregate IR.
pub enum SetQuantifier {
    All,
    Distinct,
}

impl Default for SetQuantifier {
    fn default() -> Self {
        Self::All
    }
}

/// Backend-neutral boolean algebra over canonical label names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LabelExpression {
    /// One canonical label identity.
    Name(String),
    /// Any label.
    Wildcard,
    /// Complement.
    Not(Box<LabelExpression>),
    /// Intersection.
    And(Box<LabelExpression>, Box<LabelExpression>),
    /// Union.
    Or(Box<LabelExpression>, Box<LabelExpression>),
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
    /// Numeric identity.
    Plus,
    /// Numeric negation.
    Negate,
    /// Null predicate.
    IsNull,
    /// Negated null predicate.
    IsNotNull,
    /// Truth-value test for true.
    IsTrue,
    /// Negated truth-value test for true.
    IsNotTrue,
    /// Truth-value test for false.
    IsFalse,
    /// Negated truth-value test for false.
    IsNotFalse,
    /// Truth-value test for unknown.
    IsUnknown,
    /// Negated truth-value test for unknown.
    IsNotUnknown,
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
    /// String concatenation.
    Concatenate,
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
    /// Boolean exclusive disjunction.
    Xor,
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
    /// Optional inline predicate evaluated for this node pattern.
    pub predicate: Option<Expression>,
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
    /// Optional edge-local predicate evaluated in pattern scope.
    pub predicate: Option<Expression>,
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
/// One source-ordered data modification intent.
pub enum Mutation {
    /// Insert graph patterns without assigning execution authority.
    Insert { patterns: Vec<GraphPattern> },
    /// Assign one property expression.
    SetProperty {
        target: Expression,
        value: Expression,
    },
    /// Remove one property expression.
    RemoveProperty { target: Expression },
    /// Delete one bound graph value.
    Delete { target: Expression, detach: bool },
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// First-class path pattern.
pub struct PathPattern {
    /// Optional name bound to the complete path value.
    pub binding: Option<String>,
    /// Optional ISO path search and uniqueness prefix.
    pub prefix: Option<PathPrefix>,
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
/// One atomic MATCH operation preserving graph-level and per-path semantics.
pub struct GraphMatch {
    /// Optional graph-level element uniqueness mode.
    pub mode: Option<GraphMatchMode>,
    /// Source-ordered path patterns evaluated together.
    pub paths: Vec<PathPattern>,
    /// Optional preferred path prefix retained by KEEP.
    pub keep: Option<PathPrefix>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Graph-level element uniqueness mode.
pub enum GraphMatchMode {
    RepeatableElements,
    DifferentEdges,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Canonical path traversal uniqueness mode.
pub enum PathMode {
    /// Repeated vertices and edges are allowed.
    Walk,
    /// Repeated edges are forbidden.
    Trail,
    /// Repeated vertices are forbidden, except a closing endpoint.
    Acyclic,
    /// Repeated vertices are forbidden.
    Simple,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Canonical backend-neutral path prefix.
pub struct PathPrefix {
    pub search: Option<PathSearch>,
    pub mode: Option<PathMode>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Canonical path search strategy.
pub enum PathSearch {
    All,
    Any {
        count: Option<NonNegativeIntegerSpecification>,
    },
    AllShortest,
    AnyShortest,
    Shortest {
        count: NonNegativeIntegerSpecification,
    },
    ShortestGroups {
        count: Option<NonNegativeIntegerSpecification>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Canonical non-negative integer specification without backend authority.
pub enum NonNegativeIntegerSpecification {
    Literal(u64),
    Parameter(String),
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
/// One source-ordered ISO FOR expansion in canonical form.
pub struct ForBinding {
    /// Element binding introduced by FOR.
    pub binding: Binding,
    /// Canonical collection expression evaluated before the binding is introduced.
    pub source: Expression,
    /// Optional position binding introduced by WITH ORDINALITY or WITH OFFSET.
    pub position: Option<ForPositionBinding>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Canonical position binding for one FOR expansion.
pub struct ForPositionBinding {
    pub kind: ForPositionKind,
    pub binding: Binding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Position convention retained from the ISO source statement.
pub enum ForPositionKind {
    Ordinality,
    Offset,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// A projection expression in source order.
pub struct Projection {
    /// Projected expression.
    pub expression: Expression,
    /// Optional output alias.
    pub alias: Option<String>,
    /// Statically inferred output type used for set-operation reconciliation.
    pub value_type: ValueType,
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
    /// Explicit null placement, or `None` when the ISO default applies.
    pub null_ordering: Option<NullOrdering>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Canonical null placement for one ordering key.
pub enum NullOrdering {
    First,
    Last,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// One atomic null-preserving OPTIONAL MATCH operation.
pub struct OptionalMatch {
    /// Complete graph match evaluated atomically by this optional operation.
    pub graph_match: GraphMatch,
    /// Predicate scoped to the optional operation rather than the outer query filter.
    pub predicate: Option<Expression>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Typed set operators connecting complete query branches.
pub enum SetOperator {
    /// Duplicate-eliminating UNION.
    UnionDistinct,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// One source-ordered set operation and its complete right branch.
pub struct SetOperation {
    pub operator: SetOperator,
    pub right: Box<QueryBlock>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
/// Canonical graph-semantic query block before backend planning.
pub struct QueryBlock {
    /// Mandatory MATCH operations in source order.
    pub matches: Vec<GraphMatch>,
    /// Atomic OPTIONAL MATCH operations in source order.
    pub optional_matches: Vec<OptionalMatch>,
    /// WHERE/filter expressions in source order.
    pub filters: Vec<Expression>,
    /// LET bindings in source order.
    pub let_bindings: Vec<LetBinding>,
    /// FOR expansions in source order.
    pub for_bindings: Vec<ForBinding>,
    /// Data modifications in source order.
    pub mutations: Vec<Mutation>,
    /// RETURN projection in source order.
    pub projection: Vec<Projection>,
    /// Set semantics applied to the complete result projection.
    pub projection_quantifier: SetQuantifier,
    /// Whether the query terminates with FINISH and therefore returns no binding table.
    pub is_finish: bool,
    /// GROUP BY expressions in source order.
    pub group_by: Vec<Expression>,
    /// Typed set operations and complete right branches in source order.
    pub set_operations: Vec<SetOperation>,
    /// Maximum number of rows preserved by this query block.
    pub limit: Option<NonNegativeIntegerSpecification>,
    /// Ordering keys evaluated before pagination.
    pub order_by: Vec<SortKey>,
    /// Number of rows skipped before applying the limit.
    pub offset: Option<NonNegativeIntegerSpecification>,
}
