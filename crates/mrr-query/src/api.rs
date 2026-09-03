//! Language-neutral relational goals and canonical meta-query IR.
#![forbid(unsafe_code)]

use std::io::Cursor;

pub use mrr_identity::{EntityId, QueryId, QueryOperatorId, RelationId};
pub use mrr_relation::Value;
use serde::{Deserialize, Serialize};

pub const META_QUERY_SCHEMA: &str = "mrr.meta-query.v1";
const META_QUERY_PREFIX: &[u8] = b"mrr.meta-query.v1\0";

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Variable(String);

impl Variable {
    pub fn new(name: impl Into<String>) -> Option<Self> {
        let name = name.into();
        valid_name(&name).then_some(Self(name))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Term {
    Variable(Variable),
    Value(Value),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Atom {
    pub relation: RelationId,
    pub terms: Vec<Term>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationalGoal {
    outputs: Vec<Variable>,
    body: Vec<Atom>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RelationalGoalError {
    EmptyBody,
    UnboundOutput(Variable),
}

impl RelationalGoal {
    pub fn new(outputs: Vec<Variable>, body: Vec<Atom>) -> Result<Self, RelationalGoalError> {
        if body.is_empty() {
            return Err(RelationalGoalError::EmptyBody);
        }
        for output in &outputs {
            let bound = body
                .iter()
                .flat_map(|atom| &atom.terms)
                .any(|term| matches!(term, Term::Variable(variable) if variable == output));
            if !bound {
                return Err(RelationalGoalError::UnboundOutput(output.clone()));
            }
        }
        Ok(Self { outputs, body })
    }

    #[must_use]
    pub fn outputs(&self) -> &[Variable] {
        &self.outputs
    }

    #[must_use]
    pub fn body(&self) -> &[Atom] {
        &self.body
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Binding(String);

impl Binding {
    pub fn new(name: impl Into<String>) -> Result<Self, QueryIrError> {
        let name = name.into();
        if !valid_name(&name) {
            return Err(QueryIrError::InvalidName(name));
        }
        Ok(Self(name))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Parameter(String);

impl Parameter {
    pub fn new(name: impl Into<String>) -> Result<Self, QueryIrError> {
        let name = name.into();
        if !valid_name(&name) {
            return Err(QueryIrError::InvalidName(name));
        }
        Ok(Self(name))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct PropertyKey(String);

impl PropertyKey {
    pub fn new(name: impl Into<String>) -> Result<Self, QueryIrError> {
        let name = name.into();
        if !valid_name(&name) {
            return Err(QueryIrError::InvalidName(name));
        }
        Ok(Self(name))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Direction {
    Outgoing,
    Incoming,
    Undirected,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NodePattern {
    binding: Binding,
    types: Vec<EntityId>,
}

impl NodePattern {
    #[must_use]
    pub fn new(binding: Binding, types: Vec<EntityId>) -> Self {
        Self { binding, types }
    }

    #[must_use]
    pub fn binding(&self) -> &Binding {
        &self.binding
    }

    #[must_use]
    pub fn types(&self) -> &[EntityId] {
        &self.types
    }

    fn normalize(&mut self) {
        self.types.sort_unstable();
        self.types.dedup();
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RelationPattern {
    binding: Option<Binding>,
    types: Vec<RelationId>,
    direction: Direction,
    min_hops: u32,
    max_hops: Option<u32>,
}

impl RelationPattern {
    pub fn new(
        binding: Option<Binding>,
        types: Vec<RelationId>,
        direction: Direction,
        min_hops: u32,
        max_hops: Option<u32>,
    ) -> Result<Self, QueryIrError> {
        let pattern = Self {
            binding,
            types,
            direction,
            min_hops,
            max_hops,
        };
        pattern.validate()?;
        Ok(pattern)
    }

    #[must_use]
    pub fn binding(&self) -> Option<&Binding> {
        self.binding.as_ref()
    }

    #[must_use]
    pub fn types(&self) -> &[RelationId] {
        &self.types
    }

    #[must_use]
    pub const fn direction(&self) -> Direction {
        self.direction
    }

    #[must_use]
    pub const fn min_hops(&self) -> u32 {
        self.min_hops
    }

    #[must_use]
    pub const fn max_hops(&self) -> Option<u32> {
        self.max_hops
    }

    fn validate(&self) -> Result<(), QueryIrError> {
        if self.min_hops == 0 || self.max_hops.is_some_and(|max| max < self.min_hops) {
            return Err(QueryIrError::InvalidHopRange {
                min: self.min_hops,
                max: self.max_hops,
            });
        }
        validate_optional_binding(self.binding.as_ref())
    }

    fn normalize(&mut self) {
        self.types.sort_unstable();
        self.types.dedup();
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PathSegment {
    relation: RelationPattern,
    node: NodePattern,
}

impl PathSegment {
    #[must_use]
    pub const fn new(relation: RelationPattern, node: NodePattern) -> Self {
        Self { relation, node }
    }

    #[must_use]
    pub const fn relation(&self) -> &RelationPattern {
        &self.relation
    }

    #[must_use]
    pub const fn node(&self) -> &NodePattern {
        &self.node
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PathPattern {
    start: NodePattern,
    segments: Vec<PathSegment>,
}

impl PathPattern {
    #[must_use]
    pub fn new(start: NodePattern, segments: Vec<PathSegment>) -> Self {
        Self { start, segments }
    }

    #[must_use]
    pub const fn start(&self) -> &NodePattern {
        &self.start
    }

    #[must_use]
    pub fn segments(&self) -> &[PathSegment] {
        &self.segments
    }

    fn normalize(&mut self) {
        self.start.normalize();
        for segment in &mut self.segments {
            segment.relation.normalize();
            segment.node.normalize();
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GraphPattern {
    operator: QueryOperatorId,
    paths: Vec<PathPattern>,
}

impl GraphPattern {
    pub fn new(operator: QueryOperatorId, paths: Vec<PathPattern>) -> Result<Self, QueryIrError> {
        if paths.is_empty() {
            return Err(QueryIrError::EmptyGraphPattern);
        }
        Ok(Self { operator, paths })
    }

    #[must_use]
    pub const fn operator(&self) -> QueryOperatorId {
        self.operator
    }

    #[must_use]
    pub fn paths(&self) -> &[PathPattern] {
        &self.paths
    }

    fn normalize(&mut self) {
        for path in &mut self.paths {
            path.normalize();
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum UnaryOperator {
    Not,
    Negate,
    IsNull,
    IsNotNull,
    IsTrue,
    IsNotTrue,
    IsFalse,
    IsNotFalse,
    IsUnknown,
    IsNotUnknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum BinaryOperator {
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
    And,
    Or,
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Expression {
    Binding(Binding),
    Parameter(Parameter),
    Property {
        binding: Binding,
        key: PropertyKey,
    },
    Literal(Value),
    Unary {
        operator: UnaryOperator,
        operand: Box<Expression>,
    },
    Binary {
        left: Box<Expression>,
        operator: BinaryOperator,
        right: Box<Expression>,
    },
}

impl Expression {
    fn validate(&self) -> Result<(), QueryIrError> {
        match self {
            Self::Binding(binding) => validate_binding(binding),
            Self::Parameter(parameter) => validate_name(parameter.as_str()),
            Self::Property { binding, key } => {
                validate_binding(binding)?;
                validate_name(key.as_str())
            }
            Self::Literal(_) => Ok(()),
            Self::Unary { operand, .. } => operand.validate(),
            Self::Binary { left, right, .. } => {
                left.validate()?;
                right.validate()
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Filter {
    operator: QueryOperatorId,
    predicate: Expression,
}

impl Filter {
    #[must_use]
    pub const fn new(operator: QueryOperatorId, predicate: Expression) -> Self {
        Self {
            operator,
            predicate,
        }
    }

    #[must_use]
    pub const fn operator(&self) -> QueryOperatorId {
        self.operator
    }

    #[must_use]
    pub const fn predicate(&self) -> &Expression {
        &self.predicate
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Projection {
    operator: QueryOperatorId,
    expression: Expression,
    alias: Binding,
}

impl Projection {
    #[must_use]
    pub const fn new(operator: QueryOperatorId, expression: Expression, alias: Binding) -> Self {
        Self {
            operator,
            expression,
            alias,
        }
    }

    #[must_use]
    pub const fn operator(&self) -> QueryOperatorId {
        self.operator
    }

    #[must_use]
    pub const fn expression(&self) -> &Expression {
        &self.expression
    }

    #[must_use]
    pub const fn alias(&self) -> &Binding {
        &self.alias
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum AggregationFunction {
    Count,
    Sum,
    Minimum,
    Maximum,
    Average,
    CollectList,
    StandardDeviationSample,
    StandardDeviationPopulation,
    PercentileContinuous,
    PercentileDiscrete,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum SetQuantifier {
    All,
    Distinct,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Aggregation {
    operator: QueryOperatorId,
    function: AggregationFunction,
    quantifier: Option<SetQuantifier>,
    expressions: Vec<Expression>,
    count_star: bool,
    alias: Binding,
}

impl Aggregation {
    #[must_use]
    pub const fn new(
        operator: QueryOperatorId,
        function: AggregationFunction,
        quantifier: Option<SetQuantifier>,
        expressions: Vec<Expression>,
        count_star: bool,
        alias: Binding,
    ) -> Self {
        Self {
            operator,
            function,
            quantifier,
            expressions,
            count_star,
            alias,
        }
    }

    #[must_use]
    pub const fn function(&self) -> AggregationFunction {
        self.function
    }

    #[must_use]
    pub const fn quantifier(&self) -> Option<SetQuantifier> {
        self.quantifier
    }

    #[must_use]
    pub fn expressions(&self) -> &[Expression] {
        &self.expressions
    }

    #[must_use]
    pub const fn is_count_star(&self) -> bool {
        self.count_star
    }

    #[must_use]
    pub const fn alias(&self) -> &Binding {
        &self.alias
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Ordering {
    operator: QueryOperatorId,
    expression: Expression,
    direction: SortDirection,
}

impl Ordering {
    #[must_use]
    pub const fn new(
        operator: QueryOperatorId,
        expression: Expression,
        direction: SortDirection,
    ) -> Self {
        Self {
            operator,
            expression,
            direction,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MetaQueryIr {
    id: QueryId,
    graph: GraphPattern,
    filters: Vec<Filter>,
    projections: Vec<Projection>,
    aggregations: Vec<Aggregation>,
    ordering: Vec<Ordering>,
    limit: Option<u64>,
}

impl MetaQueryIr {
    pub fn new(
        id: QueryId,
        graph: GraphPattern,
        filters: Vec<Filter>,
        projections: Vec<Projection>,
        aggregations: Vec<Aggregation>,
        ordering: Vec<Ordering>,
        limit: Option<u64>,
    ) -> Result<Self, QueryIrError> {
        let query = Self {
            id,
            graph,
            filters,
            projections,
            aggregations,
            ordering,
            limit,
        };
        query.validate()?;
        Ok(query)
    }

    #[must_use]
    pub const fn id(&self) -> QueryId {
        self.id
    }

    #[must_use]
    pub const fn graph(&self) -> &GraphPattern {
        &self.graph
    }

    #[must_use]
    pub fn filters(&self) -> &[Filter] {
        &self.filters
    }

    #[must_use]
    pub fn projections(&self) -> &[Projection] {
        &self.projections
    }

    #[must_use]
    pub fn aggregations(&self) -> &[Aggregation] {
        &self.aggregations
    }

    #[must_use]
    pub fn ordering(&self) -> &[Ordering] {
        &self.ordering
    }

    #[must_use]
    pub const fn limit(&self) -> Option<u64> {
        self.limit
    }

    /// Returns the distinct relation identities referenced by graph patterns.
    #[must_use]
    pub fn referenced_relations(&self) -> Vec<RelationId> {
        let mut relations = self
            .graph
            .paths
            .iter()
            .flat_map(|path| &path.segments)
            .flat_map(|segment| segment.relation.types.iter().copied())
            .collect::<Vec<_>>();
        relations.sort_unstable();
        relations.dedup();
        relations
    }

    #[must_use]
    pub fn normalized(mut self) -> Self {
        self.graph.normalize();
        self
    }

    pub fn encode_canonical(&self) -> Result<Vec<u8>, QueryIrError> {
        let normalized = self.clone().normalized();
        normalized.validate()?;
        let mut encoded = META_QUERY_PREFIX.to_vec();
        ciborium::into_writer(&normalized, &mut encoded)
            .map_err(|error| QueryIrError::Encoding(error.to_string()))?;
        Ok(encoded)
    }

    pub fn decode_canonical(encoded: &[u8]) -> Result<Self, QueryIrError> {
        let payload = encoded
            .strip_prefix(META_QUERY_PREFIX)
            .ok_or(QueryIrError::SchemaMismatch)?;
        let mut cursor = Cursor::new(payload);
        let query: Self = ciborium::from_reader(&mut cursor)
            .map_err(|error| QueryIrError::Decoding(error.to_string()))?;
        if cursor.position() != payload.len() as u64 {
            return Err(QueryIrError::TrailingBytes);
        }
        let query = query.normalized();
        query.validate()?;
        Ok(query)
    }

    fn validate(&self) -> Result<(), QueryIrError> {
        if self.graph.paths.is_empty() {
            return Err(QueryIrError::EmptyGraphPattern);
        }
        if self.projections.is_empty() && self.aggregations.is_empty() {
            return Err(QueryIrError::EmptyOutput);
        }
        for path in &self.graph.paths {
            validate_binding(&path.start.binding)?;
            for segment in &path.segments {
                segment.relation.validate()?;
                validate_binding(&segment.node.binding)?;
            }
        }
        for filter in &self.filters {
            filter.predicate.validate()?;
        }
        for projection in &self.projections {
            projection.expression.validate()?;
            validate_binding(&projection.alias)?;
        }
        for aggregation in &self.aggregations {
            let expected_arity = if matches!(
                aggregation.function,
                AggregationFunction::PercentileContinuous | AggregationFunction::PercentileDiscrete
            ) {
                2
            } else if aggregation.count_star {
                0
            } else {
                1
            };
            if aggregation.expressions.len() != expected_arity
                || (aggregation.count_star && aggregation.function != AggregationFunction::Count)
                || (aggregation.count_star && aggregation.quantifier.is_some())
            {
                return Err(QueryIrError::AggregationRequiresExpression);
            }
            for expression in &aggregation.expressions {
                expression.validate()?;
            }
            validate_binding(&aggregation.alias)?;
        }
        for ordering in &self.ordering {
            ordering.expression.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QueryIrError {
    InvalidName(String),
    EmptyGraphPattern,
    EmptyOutput,
    InvalidHopRange { min: u32, max: Option<u32> },
    AggregationRequiresExpression,
    SchemaMismatch,
    TrailingBytes,
    Encoding(String),
    Decoding(String),
}

fn valid_name(name: &str) -> bool {
    !name.is_empty() && name.trim() == name
}

fn validate_name(name: &str) -> Result<(), QueryIrError> {
    if valid_name(name) {
        Ok(())
    } else {
        Err(QueryIrError::InvalidName(name.to_owned()))
    }
}

fn validate_binding(binding: &Binding) -> Result<(), QueryIrError> {
    validate_name(binding.as_str())
}

fn validate_optional_binding(binding: Option<&Binding>) -> Result<(), QueryIrError> {
    binding.map_or(Ok(()), validate_binding)
}
