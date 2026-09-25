// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Minimal semantic projection produced from the parser-owned GQL CST.

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Identifier {
    pub(crate) text: String,
}

impl Identifier {
    pub(crate) fn canonical_text(&self) -> String {
        self.text.to_uppercase()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Query {
    pub(crate) clauses: Vec<QueryClause>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum QueryClause {
    Match(MatchClause),
    Where(Expression),
    Filter(Expression),
    Return {
        quantifier: Option<SetQuantifier>,
        all_bindings: bool,
        projections: Vec<ReturnProjection>,
    },
    Finish,
    Limit(NonNegativeIntegerSpecification),
    OrderBy(Vec<SortKey>),
    Offset(NonNegativeIntegerSpecification),
    GroupBy(Vec<Expression>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SortKey {
    pub(crate) expression: Expression,
    pub(crate) direction: Option<SortDirection>,
    pub(crate) null_ordering: Option<NullOrdering>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NullOrdering {
    First,
    Last,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReturnProjection {
    pub(crate) expression: Expression,
    pub(crate) alias: Option<Identifier>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MatchClause {
    pub(crate) patterns: Vec<PathPattern>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PathPattern {
    pub(crate) elements: Vec<PatternElement>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PatternElement {
    Node(NodePattern),
    Edge(EdgePattern),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PropertyConstraint {
    pub(crate) key: Identifier,
    pub(crate) value: Expression,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NodePattern {
    pub(crate) binding: Option<Identifier>,
    pub(crate) labels: Vec<Identifier>,
    pub(crate) properties: Vec<PropertyConstraint>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EdgePattern {
    pub(crate) binding: Option<Identifier>,
    pub(crate) labels: Vec<Identifier>,
    pub(crate) properties: Vec<PropertyConstraint>,
    pub(crate) direction: EdgeDirection,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EdgeDirection {
    Out,
    In,
    Undirected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum NonNegativeIntegerSpecification {
    Literal(u64),
    Parameter(DynamicParameterReference),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DynamicParameterReference {
    pub(crate) name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RecordField {
    pub(crate) name: Identifier,
    pub(crate) value: Expression,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Expression {
    Name(Identifier),
    Parameter(DynamicParameterReference),
    Boolean(bool),
    Null,
    String(String),
    Integer(i64),
    Decimal(String),
    ApproximateNumeric(String),
    Date(String),
    Time(String),
    Timestamp(String),
    Duration(String),
    List(Vec<Expression>),
    Record(Vec<RecordField>),
    PropertyAccess {
        base: Box<Expression>,
        property: Identifier,
    },
    AggregateCall {
        function: AggregateFunction,
        quantifier: Option<SetQuantifier>,
        arguments: Vec<Expression>,
        count_star: bool,
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
    },
    TruthPredicate {
        operand: Box<Expression>,
        value: TruthValue,
        negated: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AggregateFunction {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SetQuantifier {
    All,
    Distinct,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TruthValue {
    True,
    False,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UnaryOperator {
    Not,
    Plus,
    Negate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equals,
    NotEquals,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    And,
    Or,
}
