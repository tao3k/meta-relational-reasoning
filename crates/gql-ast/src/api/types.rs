//! Abstract syntax model and lowering entrypoint for GQL source trees.
#![forbid(unsafe_code)]

use gql_source::{Diagnostic, Span};

/// ISO GQL identifier spelling class.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum IdentifierForm {
    Undelimited,
    Delimited,
}

/// Named identifier token plus source location and spelling class.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Identifier {
    pub text: String,
    pub span: Span,
    pub form: IdentifierForm,
}

impl Identifier {
    /// Return the backend-neutral identity used for name equality.
    #[must_use]
    pub fn canonical_text(&self) -> String {
        match self.form {
            IdentifierForm::Undelimited => self.text.to_uppercase(),
            IdentifierForm::Delimited => self.text.clone(),
        }
    }
}

/// Source-ordered qualified catalog identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogObjectName {
    pub parts: Vec<Identifier>,
    pub span: Span,
}

impl CatalogObjectName {
    /// Return the canonical dotted identity without discarding path segments.
    #[must_use]
    pub fn canonical_text(&self) -> String {
        self.parts
            .iter()
            .map(Identifier::canonical_text)
            .collect::<Vec<_>>()
            .join(".")
    }
}

/// A lowered statement surface for all supported entry points.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Statement {
    Query(Query),
    Catalog(CatalogStatement),
    Procedure(ProcedureCall),
    Transaction(TransactionCommand),
    Session(SessionCommand),
}

/// Session control syntax retained as frontend intent only.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionCommand {
    SetSchema { name: Identifier, span: Span },
    ResetSchema { span: Span },
    Close { span: Span },
}

/// One backend-neutral named procedure invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcedureCall {
    pub name: Vec<Identifier>,
    pub arguments: Vec<Expression>,
    pub span: Span,
}

/// Transaction control syntax retained as frontend intent only.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransactionCommand {
    Start {
        access_mode: Option<TransactionAccessMode>,
        span: Span,
    },
    Commit {
        span: Span,
    },
    Rollback {
        span: Span,
    },
}

/// ISO transaction access mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransactionAccessMode {
    ReadOnly,
    ReadWrite,
}

/// A parsed query with concrete clauses.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Query {
    pub clauses: Vec<QueryClause>,
    pub span: Span,
}

/// The public clause variants supported by the current pipeline.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QueryClause {
    Match(MatchClause),
    OptionalMatch(MatchClause),
    Where {
        expression: Expression,
        span: Span,
    },
    Let {
        bindings: Vec<LetBinding>,
        span: Span,
    },
    Filter {
        expression: Expression,
        span: Span,
    },
    For {
        item: ForItem,
        span: Span,
    },
    Return {
        quantifier: Option<SetQuantifier>,
        all_bindings: bool,
        projections: Vec<ReturnProjection>,
        span: Span,
    },
    Finish {
        span: Span,
    },
    Union {
        span: Span,
    },
    Limit {
        value: NonNegativeIntegerSpecification,
        span: Span,
    },
    OrderBy {
        keys: Vec<SortKey>,
        span: Span,
    },
    Offset {
        value: NonNegativeIntegerSpecification,
        span: Span,
    },
    GroupBy {
        keys: Vec<Expression>,
        span: Span,
    },
    Insert {
        patterns: Vec<GraphPattern>,
        span: Span,
    },
    Set {
        items: Vec<SetItem>,
        span: Span,
    },
    Remove {
        targets: Vec<Expression>,
        span: Span,
    },
    Delete {
        targets: Vec<Expression>,
        detach: bool,
        span: Span,
    },
}

/// One property assignment in a SET statement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetItem {
    pub target: Expression,
    pub value: Expression,
    pub span: Span,
}

/// One source-ordered binding in a LET clause.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LetBinding {
    pub binding: Identifier,
    pub value: Expression,
    pub span: Span,
}

/// One ISO FOR binding over a value-expression source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForItem {
    pub binding: Identifier,
    pub source: Expression,
    pub ordinality: Option<ForOrdinalityBinding>,
    pub span: Span,
}

/// Source spelling of the optional FOR position binding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ForOrdinalityKind {
    Ordinality,
    Offset,
}

/// Optional position binding introduced by WITH ORDINALITY or WITH OFFSET.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForOrdinalityBinding {
    pub kind: ForOrdinalityKind,
    pub binding: Identifier,
    pub span: Span,
}

/// An ordering expression with its source direction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SortKey {
    pub expression: Expression,
    pub direction: Option<SortDirection>,
    pub null_ordering: Option<NullOrdering>,
}

/// Source ordering direction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// Explicit source-level null placement for one sort specification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NullOrdering {
    First,
    Last,
}

/// A RETURN expression with an optional output alias.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReturnProjection {
    pub expression: Expression,
    pub alias: Option<Identifier>,
}

/// A `MATCH` clause with a compiled graph pattern.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchClause {
    pub mode: Option<GraphMatchMode>,
    pub patterns: Vec<PathPattern>,
    pub keep: Option<PathPrefix>,
    pub span: Span,
}

/// Graph-level element uniqueness requested by one MATCH operation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GraphMatchMode {
    RepeatableElements,
    DifferentEdges,
}

/// Path traversal uniqueness mode attached to a MATCH graph pattern.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PathMode {
    Walk,
    Trail,
    Acyclic,
    Simple,
}

/// A graph pattern made of nodes, edges, and path fragments.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphPattern {
    pub elements: Vec<PatternElement>,
    pub span: Span,
}

/// A graph pattern element in AST order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PatternElement {
    Node(NodePattern),
    Edge(EdgePattern),
    Path(PathPattern),
}

/// A property constraint in a node pattern.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PropertyConstraint {
    pub key: Identifier,
    pub value: Expression,
    pub span: Span,
}

/// A node pattern with optional binding, labels, and property constraints.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodePattern {
    pub binding: Option<Identifier>,
    pub labels: Vec<Identifier>,
    pub properties: Vec<PropertyConstraint>,
    pub predicate: Option<Expression>,
    pub span: Span,
}

/// An edge pattern with relation labels and direction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EdgePattern {
    pub binding: Option<Identifier>,
    pub labels: Vec<Identifier>,
    pub properties: Vec<PropertyConstraint>,
    pub predicate: Option<Expression>,
    pub direction: EdgeDirection,
    pub quantifier: Option<PathQuantifier>,
    pub span: Span,
}

/// A bounded repetition attached to an edge pattern.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathQuantifier {
    pub min: u32,
    pub max: Option<u32>,
    pub span: Span,
}

/// A path pattern that stores nested pattern elements.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathPattern {
    pub binding: Option<Identifier>,
    pub prefix: Option<PathPrefix>,
    pub elements: Vec<PatternElement>,
    pub span: Span,
}

/// Source-level path search and uniqueness prefix attached to one path only.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathPrefix {
    pub search: Option<PathSearch>,
    pub mode: Option<PathMode>,
    pub target: Option<PathTarget>,
    pub span: Span,
}

/// ISO GQL path search strategy.
#[derive(Clone, Debug, Eq, PartialEq)]
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

/// ISO non-negative integer specification shared by path search and pagination.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NonNegativeIntegerSpecification {
    Literal(u64),
    Parameter(DynamicParameterReference),
}

/// Lossless singular/plural path target spelling.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PathTarget {
    Path,
    Paths,
}

/// Relationship direction recovered from parser token neighborhoods.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EdgeDirection {
    Out,
    In,
    Undirected,
}

/// Declared direction class on an edge type specification.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EdgeKind {
    Directed,
    Undirected,
}

impl MatchClause {
    #[must_use]
    pub fn relation_candidates(&self) -> Vec<Identifier> {
        self.patterns
            .iter()
            .flat_map(|pattern| &pattern.elements)
            .filter_map(PatternElement::relation_identifier)
            .collect()
    }
}

impl GraphPattern {
    #[must_use]
    pub fn relation_candidates(&self) -> Vec<Identifier> {
        self.elements
            .iter()
            .filter_map(PatternElement::relation_identifier)
            .collect()
    }
}

impl PatternElement {
    fn relation_identifier(&self) -> Option<Identifier> {
        match self {
            PatternElement::Node(node) => node.labels.first().cloned(),
            PatternElement::Edge(edge) => edge.labels.first().cloned(),
            PatternElement::Path(path) => path
                .elements
                .iter()
                .find_map(PatternElement::relation_identifier),
        }
    }
}

/// One ordered field in a record literal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordField {
    pub name: Identifier,
    pub value: Expression,
    pub span: Span,
}

/// Source representation selected for an ISO GQL character-string literal.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CharacterStringForm {
    SingleQuoted,
    DoubleQuoted,
}

/// Decoded character-string value with its lossless source-level contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterStringLiteral {
    pub value: String,
    pub form: CharacterStringForm,
    pub no_escape: bool,
    pub span: Span,
}

/// Source representation selected for an ISO GQL parameter name.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ParameterNameForm {
    Extended,
    Delimited,
}

/// One general parameter reference retained independently from graph bindings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DynamicParameterReference {
    pub name: String,
    pub form: ParameterNameForm,
    pub span: Span,
}

/// Lowered expression form used by analysis.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expression {
    Name(Identifier),
    Parameter(DynamicParameterReference),
    Boolean(bool, Span),
    Null(Span),
    String(CharacterStringLiteral),
    Integer(i64, Span),
    Decimal(String, Span),
    ApproximateNumeric(String, Span),
    ByteString(Vec<u8>, Span),
    Date(String, Span),
    Time(String, Span),
    Timestamp(String, Span),
    Duration(String, Span),
    List(Vec<Expression>, Span),
    Record(Vec<RecordField>, Span),
    Subscript {
        base: Box<Expression>,
        index: Box<Expression>,
    },
    PropertyAccess {
        base: Box<Expression>,
        property: Identifier,
    },
    FunctionCall {
        name: Identifier,
        arguments: Vec<Expression>,
        span: Span,
    },
    AggregateCall {
        function: AggregateFunction,
        quantifier: Option<SetQuantifier>,
        arguments: Vec<Expression>,
        count_star: bool,
        span: Span,
    },
    Unary {
        operator: UnaryOperator,
        operand: Box<Expression>,
    },
    Binary {
        operator: BinaryOperator,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    NullPredicate {
        operand: Box<Expression>,
        negated: bool,
        span: Span,
    },
    TruthPredicate {
        operand: Box<Expression>,
        value: TruthValue,
        negated: bool,
        span: Span,
    },
    ValueTypePredicate {
        operand: Box<Expression>,
        value_type: PropertyValueType,
        negated: bool,
        span: Span,
    },
    DirectedPredicate {
        edge: Box<Expression>,
        negated: bool,
        span: Span,
    },
    EndpointPredicate {
        node: Box<Expression>,
        edge: Box<Expression>,
        endpoint: EndpointKind,
        negated: bool,
        span: Span,
    },
    ElementIdentityPredicate {
        kind: ElementIdentityKind,
        elements: Vec<Expression>,
        span: Span,
    },
    PropertyExistsPredicate {
        element: Box<Expression>,
        property: Identifier,
        span: Span,
    },
    IsLabeled {
        operand: Box<Expression>,
        label: LabelExpression,
        negated: bool,
        span: Span,
    },
    Case {
        operand: Option<Box<Expression>>,
        branches: Vec<CaseBranch>,
        else_result: Option<Box<Expression>>,
        span: Span,
    },
}

/// Endpoint role selected by an ISO source/destination predicate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndpointKind {
    Source,
    Destination,
}

/// N-ary graph-element identity relation selected by the source syntax.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElementIdentityKind {
    AllDifferent,
    Same,
}

/// ISO GQL aggregate function identity, independent of backend execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AggregateFunction {
    Average,
    Count,
    Maximum,
    Minimum,
    Sum,
    CollectList,
    StandardDeviationSample,
    StandardDeviationPopulation,
    PercentileContinuous,
    PercentileDiscrete,
}

/// Explicit set quantifier on an aggregate value argument.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SetQuantifier {
    All,
    Distinct,
}

/// ISO truth value named by an `IS [NOT]` predicate test.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TruthValue {
    True,
    False,
    Unknown,
}

/// Boolean algebra over graph-element labels.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LabelExpression {
    Name(Identifier),
    Wildcard,
    Not(Box<LabelExpression>),
    And(Box<LabelExpression>, Box<LabelExpression>),
    Or(Box<LabelExpression>, Box<LabelExpression>),
}

/// One ordered `WHEN ... THEN ...` branch in a CASE expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaseBranch {
    pub condition: Expression,
    pub result: Expression,
    pub span: Span,
}

/// Unary expression operators in lowered form.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnaryOperator {
    Not,
    Plus,
    Negate,
}

/// Binary expression operators in lowered form.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Concatenate,
    In,
    Equals,
    NotEquals,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    And,
    Xor,
    Or,
}

/// Catalog statement variants accepted by the broader parser surface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CatalogStatement {
    CreateSchema {
        name: CatalogObjectName,
    },
    DropSchema {
        name: CatalogObjectName,
    },
    CreateGraph {
        name: CatalogObjectName,
        graph_type: GraphTypeSpecification,
        policy: CatalogCreatePolicy,
    },
    DropGraph {
        name: CatalogObjectName,
        policy: CatalogDropPolicy,
    },
    CreateGraphType {
        name: CatalogObjectName,
        source: GraphTypeSource,
        policy: CatalogCreatePolicy,
    },
    DropGraphType {
        name: CatalogObjectName,
        policy: CatalogDropPolicy,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Conflict behavior declared by CREATE catalog statements.
pub enum CatalogCreatePolicy {
    Error,
    IfNotExists,
    OrReplace,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Missing-object behavior declared by DROP catalog statements.
pub enum CatalogDropPolicy {
    Error,
    IfExists,
}

/// Graph type syntax admitted by catalog creation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphTypeSpecification {
    Any { typed: bool, span: Span },
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Source used to construct a graph type.
pub enum GraphTypeSource {
    CopyOf {
        graph_type: CatalogObjectName,
        span: Span,
    },
    LikeGraph {
        graph: CatalogObjectName,
        span: Span,
    },
    Nested {
        specification: NestedGraphTypeSpecification,
        span: Span,
    },
}

/// Inline graph type elements in source order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NestedGraphTypeSpecification {
    pub node_types: Vec<NodeTypeSpecification>,
    pub edge_types: Vec<EdgeTypeSpecification>,
    pub span: Span,
}

/// One node type declaration with optional global name and local alias.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeTypeSpecification {
    pub name: Option<Identifier>,
    pub alias: Option<Identifier>,
    pub key_labels: Option<Vec<Identifier>>,
    pub labels: Vec<Identifier>,
    pub properties: Vec<PropertyType>,
    pub span: Span,
}

/// One edge type declaration with source-ordered endpoint aliases.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EdgeTypeSpecification {
    pub kind: Option<EdgeKind>,
    pub name: Option<Identifier>,
    pub endpoints: Vec<NodeTypeReference>,
    pub direction: EdgeDirection,
    pub key_labels: Option<Vec<Identifier>>,
    pub labels: Vec<Identifier>,
    pub properties: Vec<PropertyType>,
    pub span: Span,
}

/// One endpoint reference in an edge type pattern.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeTypeReference {
    Alias(Identifier),
    Inline {
        key_labels: Option<Vec<Identifier>>,
        labels: Vec<Identifier>,
        properties: Vec<PropertyType>,
        span: Span,
    },
}

/// One named property type in source order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PropertyType {
    pub name: Identifier,
    pub value_type: PropertyValueType,
    pub span: Span,
}

/// One source-level ISO GQL value-type specification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PropertyValueType {
    pub form: PropertyValueTypeForm,
    pub non_null: bool,
    pub span: Span,
}

/// Structural form of an ISO GQL value-type specification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PropertyValueTypeForm {
    Named {
        name: String,
        parameters: Vec<TypeParameter>,
    },
    List {
        element: Option<Box<PropertyValueType>>,
        max_length: Option<u64>,
    },
    Record {
        open: bool,
        fields: Vec<PropertyType>,
    },
    DynamicUnion {
        property_values: bool,
        members: Option<Vec<PropertyValueType>>,
    },
    Reference {
        kind: ReferenceValueTypeKind,
        open: bool,
        property_graph: bool,
        specification: Option<Box<ClosedReferenceTypeSpecification>>,
        fields: Vec<PropertyType>,
    },
    Union(Vec<PropertyValueType>),
}

/// Reference-value categories defined by the ISO GQL type lattice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferenceValueTypeKind {
    Graph,
    BindingTable,
    Node,
    Edge,
}

/// Closed structural descriptor carried by a graph, node, or edge reference type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClosedReferenceTypeSpecification {
    Graph(NestedGraphTypeSpecification),
    Node(NodeTypeSpecification),
    Edge(EdgeTypeSpecification),
}

/// Numeric or temporal parameter retained from a value-type declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TypeParameter {
    Unsigned(u64),
    DurationQualifier { from: String, to: String },
}

/// Result of syntax lowering and collected diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxParseOutput {
    pub statement: Option<Statement>,
    pub diagnostics: Vec<Diagnostic>,
}
