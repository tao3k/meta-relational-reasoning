//! Canonical result, grouping, ordering, and pagination semantics.

use serde::{Deserialize, Serialize};

use crate::{
    Aggregation, Expression, Ordering, Parameter, Projection, QueryOperatorId, SetQuantifier,
};

/// One expression defining an aggregation group.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Grouping {
    operator: QueryOperatorId,
    expression: Expression,
}

impl Grouping {
    /// Creates a grouping expression with its stable operator identity.
    #[must_use]
    pub const fn new(operator: QueryOperatorId, expression: Expression) -> Self {
        Self {
            operator,
            expression,
        }
    }

    /// Returns the operator identity assigned during lowering.
    #[must_use]
    pub const fn operator(&self) -> QueryOperatorId {
        self.operator
    }

    /// Returns the expression used as the group key.
    #[must_use]
    pub const fn expression(&self) -> &Expression {
        &self.expression
    }
}

/// A literal or runtime-bound pagination value.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PageValue {
    /// A source literal known during lowering.
    Literal(u64),
    /// A dynamic parameter resolved by the query runtime.
    Parameter(Parameter),
}

/// Terminal result behavior for a query block.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ResultMode {
    /// Produce rows using the specified set quantifier.
    Return(SetQuantifier),
    /// Execute the query without producing a result table.
    Finish,
}

/// Complete language-neutral result contract for one meta-query.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct QueryResult {
    pub(crate) mode: ResultMode,
    pub(crate) projections: Vec<Projection>,
    pub(crate) aggregations: Vec<Aggregation>,
    pub(crate) grouping: Vec<Grouping>,
    pub(crate) ordering: Vec<Ordering>,
    pub(crate) offset: Option<PageValue>,
    pub(crate) limit: Option<PageValue>,
}

impl QueryResult {
    /// Starts a row-producing result contract.
    #[must_use]
    pub const fn returning(quantifier: SetQuantifier) -> Self {
        Self {
            mode: ResultMode::Return(quantifier),
            projections: Vec::new(),
            aggregations: Vec::new(),
            grouping: Vec::new(),
            ordering: Vec::new(),
            offset: None,
            limit: None,
        }
    }

    /// Creates a terminal that intentionally produces no result table.
    #[must_use]
    pub const fn finish() -> Self {
        Self {
            mode: ResultMode::Finish,
            projections: Vec::new(),
            aggregations: Vec::new(),
            grouping: Vec::new(),
            ordering: Vec::new(),
            offset: None,
            limit: None,
        }
    }

    /// Attaches explicit row projections.
    #[must_use]
    pub fn with_projections(mut self, projections: Vec<Projection>) -> Self {
        self.projections = projections;
        self
    }

    /// Attaches aggregate outputs.
    #[must_use]
    pub fn with_aggregations(mut self, aggregations: Vec<Aggregation>) -> Self {
        self.aggregations = aggregations;
        self
    }

    /// Attaches aggregation group keys.
    #[must_use]
    pub fn with_grouping(mut self, grouping: Vec<Grouping>) -> Self {
        self.grouping = grouping;
        self
    }

    /// Attaches output ordering.
    #[must_use]
    pub fn with_ordering(mut self, ordering: Vec<Ordering>) -> Self {
        self.ordering = ordering;
        self
    }

    /// Attaches an optional row offset.
    #[must_use]
    pub fn with_offset(mut self, offset: Option<PageValue>) -> Self {
        self.offset = offset;
        self
    }

    /// Attaches an optional result limit.
    #[must_use]
    pub fn with_limit(mut self, limit: Option<PageValue>) -> Self {
        self.limit = limit;
        self
    }

    /// Returns whether the query produces rows or only side effects.
    #[must_use]
    pub const fn mode(&self) -> ResultMode {
        self.mode
    }
}
