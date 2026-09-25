// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Canonical public evidence emitted by static query typing.

use mrr_identity::{EntityId, RelationId};
use mrr_query::{Binding, Parameter};
use mrr_relation::{ValueKind, ValueSchema};
use serde::{Deserialize, Serialize};

use crate::ResolvedProperty;

/// A query-level type without introducing a second scalar schema system.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum QueryType {
    Node(Vec<EntityId>),
    Relation(Vec<RelationId>),
    Schema(ValueSchema),
    Kind(ValueKind),
    Numeric,
    AnyValue,
    Null,
    List(Box<ExpressionType>),
}

/// Static type and nullability for one expression.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExpressionType {
    pub(super) query_type: QueryType,
    pub(super) nullable: bool,
}

/// A runtime parameter constraint inferred from every use in the query.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ParameterType {
    pub(super) parameter: Parameter,
    pub(super) expression_type: ExpressionType,
}

/// One ordered output field produced by a query.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResultField {
    pub(super) name: Binding,
    pub(super) expression_type: ExpressionType,
}

/// Canonical static typing evidence included in catalog-bound query identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StaticQueryTyping {
    pub(super) resolved_properties: Vec<ResolvedProperty>,
    pub(super) parameters: Vec<ParameterType>,
    pub(super) result_fields: Vec<ResultField>,
}

impl ExpressionType {
    #[must_use]
    pub const fn query_type(&self) -> &QueryType {
        &self.query_type
    }

    #[must_use]
    pub const fn nullable(&self) -> bool {
        self.nullable
    }

    pub(super) const fn new(query_type: QueryType, nullable: bool) -> Self {
        Self {
            query_type,
            nullable,
        }
    }

    pub(super) const fn schema(schema: ValueSchema, nullable: bool) -> Self {
        Self::new(QueryType::Schema(schema), nullable)
    }

    pub(super) fn with_nullable(mut self, nullable: bool) -> Self {
        self.nullable = nullable;
        self
    }
}

impl ParameterType {
    #[must_use]
    pub const fn parameter(&self) -> &Parameter {
        &self.parameter
    }

    #[must_use]
    pub const fn expression_type(&self) -> &ExpressionType {
        &self.expression_type
    }
}

impl ResultField {
    #[must_use]
    pub const fn name(&self) -> &Binding {
        &self.name
    }

    #[must_use]
    pub const fn expression_type(&self) -> &ExpressionType {
        &self.expression_type
    }
}

impl StaticQueryTyping {
    #[must_use]
    pub fn resolved_properties(&self) -> &[ResolvedProperty] {
        &self.resolved_properties
    }

    #[must_use]
    pub fn parameters(&self) -> &[ParameterType] {
        &self.parameters
    }

    #[must_use]
    pub fn result_fields(&self) -> &[ResultField] {
        &self.result_fields
    }
}
