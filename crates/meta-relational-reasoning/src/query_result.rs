//! Admission of bounded physical query results into MRR candidate evidence.

use core::num::NonZeroUsize;
use std::{collections::BTreeSet, fmt};

use mrr_bundle::{EntityCatalogDigest, RelationCatalogDigest};
use mrr_identity::{EntityId, FactId, GenerationId, RelationId};
use mrr_query::{Binding, PageValue, ResultMode, SetQuantifier};
use mrr_relation::{RelationError, Value, ValueKind, ValueSchema};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{CatalogBoundQuery, ExpressionType, QueryType};

const QUERY_RESULT_ADMISSION_SCHEMA: &[u8] = b"mrr.query-result-admission.v1";

/// One self-describing value returned by a physical query engine.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum QueryResultValue {
    Null,
    Scalar {
        schema: ValueSchema,
        value: Value,
    },
    Node {
        id: EntityId,
        entity_type: EntityId,
    },
    Relation {
        id: FactId,
        relation_type: RelationId,
    },
    List(Vec<QueryResultValue>),
}

/// Coarse result category used only in typed admission failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryResultValueKind {
    Null,
    Scalar,
    Node,
    Relation,
    List,
}

/// The exact upstream semantic identities a downstream result must retain.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QueryResultBinding {
    query_binding_digest: [u8; 32],
    generation: GenerationId,
    relation_catalog_digest: RelationCatalogDigest,
    entity_catalog_digest: EntityCatalogDigest,
    snapshot_digest: [u8; 32],
}

/// A downstream result candidate carrying all upstream semantic identities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateQueryResult {
    binding: QueryResultBinding,
    columns: Vec<Binding>,
    rows: Vec<Vec<QueryResultValue>>,
}

/// Bounded resources for one in-memory candidate admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QueryResultLimits {
    max_rows: NonZeroUsize,
    max_cells: NonZeroUsize,
}

/// An identity- and type-complete candidate result admitted by MRR.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryResultAdmissionReceipt {
    binding: QueryResultBinding,
    row_count: usize,
    digest: [u8; 32],
}

/// Reasons a physical result cannot enter MRR candidate evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QueryResultAdmissionError {
    QueryBindingMismatch,
    GenerationMismatch,
    RelationCatalogMismatch,
    EntityCatalogMismatch,
    SnapshotMismatch,
    RowLimitExceeded {
        limit: usize,
        actual: usize,
    },
    CellLimitExceeded {
        limit: usize,
        actual: usize,
    },
    UnexpectedRowsForFinish {
        actual: usize,
    },
    ResultLimitExceeded {
        limit: usize,
        actual: usize,
    },
    DuplicateDistinctRow {
        row: usize,
    },
    ColumnCountMismatch {
        expected: usize,
        actual: usize,
    },
    ColumnNameMismatch {
        column: usize,
        expected: Binding,
        actual: Binding,
    },
    RowWidthMismatch {
        row: usize,
        expected: usize,
        actual: usize,
    },
    NullNotAllowed {
        row: usize,
        column: usize,
    },
    ValueTypeMismatch {
        row: usize,
        column: usize,
        expected: QueryType,
        actual: QueryResultValueKind,
    },
    ScalarSchemaMismatch {
        row: usize,
        column: usize,
        expected: ValueSchema,
        actual: ValueSchema,
    },
    ScalarKindMismatch {
        row: usize,
        column: usize,
        expected: ValueKind,
        actual: ValueKind,
    },
    InvalidScalar {
        row: usize,
        column: usize,
        error: RelationError,
    },
    Encoding(String),
}

impl fmt::Display for QueryResultAdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for QueryResultAdmissionError {}

impl QueryResultValue {
    #[must_use]
    pub const fn null() -> Self {
        Self::Null
    }

    #[must_use]
    pub const fn scalar(schema: ValueSchema, value: Value) -> Self {
        Self::Scalar { schema, value }
    }

    #[must_use]
    pub const fn node(id: EntityId, entity_type: EntityId) -> Self {
        Self::Node { id, entity_type }
    }

    #[must_use]
    pub const fn relation(id: FactId, relation_type: RelationId) -> Self {
        Self::Relation { id, relation_type }
    }

    #[must_use]
    pub const fn list(values: Vec<Self>) -> Self {
        Self::List(values)
    }

    #[must_use]
    pub const fn kind(&self) -> QueryResultValueKind {
        match self {
            Self::Null => QueryResultValueKind::Null,
            Self::Scalar { .. } => QueryResultValueKind::Scalar,
            Self::Node { .. } => QueryResultValueKind::Node,
            Self::Relation { .. } => QueryResultValueKind::Relation,
            Self::List(_) => QueryResultValueKind::List,
        }
    }
}

impl QueryResultBinding {
    #[must_use]
    pub const fn new(
        query_binding_digest: [u8; 32],
        generation: GenerationId,
        relation_catalog_digest: RelationCatalogDigest,
        entity_catalog_digest: EntityCatalogDigest,
        snapshot_digest: [u8; 32],
    ) -> Self {
        Self {
            query_binding_digest,
            generation,
            relation_catalog_digest,
            entity_catalog_digest,
            snapshot_digest,
        }
    }

    #[must_use]
    pub fn for_query(query: &CatalogBoundQuery) -> Self {
        Self::new(
            *query.digest(),
            query.generation(),
            query.catalog_digest(),
            query.entity_catalog_digest(),
            *query.snapshot_digest(),
        )
    }

    #[must_use]
    pub const fn query_binding_digest(&self) -> &[u8; 32] {
        &self.query_binding_digest
    }

    #[must_use]
    pub const fn generation(&self) -> GenerationId {
        self.generation
    }

    #[must_use]
    pub const fn relation_catalog_digest(&self) -> RelationCatalogDigest {
        self.relation_catalog_digest
    }

    #[must_use]
    pub const fn entity_catalog_digest(&self) -> EntityCatalogDigest {
        self.entity_catalog_digest
    }

    #[must_use]
    pub const fn snapshot_digest(&self) -> &[u8; 32] {
        &self.snapshot_digest
    }
}

impl CandidateQueryResult {
    #[must_use]
    pub const fn new(
        binding: QueryResultBinding,
        columns: Vec<Binding>,
        rows: Vec<Vec<QueryResultValue>>,
    ) -> Self {
        Self {
            binding,
            columns,
            rows,
        }
    }

    #[must_use]
    pub const fn binding(&self) -> &QueryResultBinding {
        &self.binding
    }

    #[must_use]
    pub fn columns(&self) -> &[Binding] {
        &self.columns
    }

    #[must_use]
    pub fn rows(&self) -> &[Vec<QueryResultValue>] {
        &self.rows
    }
}

impl QueryResultLimits {
    #[must_use]
    pub const fn new(max_rows: NonZeroUsize, max_cells: NonZeroUsize) -> Self {
        Self {
            max_rows,
            max_cells,
        }
    }
}

impl QueryResultAdmissionReceipt {
    #[must_use]
    pub const fn binding(&self) -> QueryResultBinding {
        self.binding
    }

    #[must_use]
    pub const fn row_count(&self) -> usize {
        self.row_count
    }

    #[must_use]
    pub const fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
}

/// Admits an exact, bounded result candidate for one catalog-bound query.
pub fn admit_query_result_candidate(
    query: &CatalogBoundQuery,
    candidate: &CandidateQueryResult,
    limits: QueryResultLimits,
) -> Result<QueryResultAdmissionReceipt, QueryResultAdmissionError> {
    validate_identity(query, candidate)?;
    validate_shape(query, candidate, limits)?;
    let digest = digest_candidate(candidate)?;
    Ok(QueryResultAdmissionReceipt {
        binding: candidate.binding,
        row_count: candidate.rows.len(),
        digest,
    })
}

fn validate_identity(
    query: &CatalogBoundQuery,
    candidate: &CandidateQueryResult,
) -> Result<(), QueryResultAdmissionError> {
    if candidate.binding.query_binding_digest != *query.digest() {
        return Err(QueryResultAdmissionError::QueryBindingMismatch);
    }
    if candidate.binding.generation != query.generation() {
        return Err(QueryResultAdmissionError::GenerationMismatch);
    }
    if candidate.binding.relation_catalog_digest != query.catalog_digest() {
        return Err(QueryResultAdmissionError::RelationCatalogMismatch);
    }
    if candidate.binding.entity_catalog_digest != query.entity_catalog_digest() {
        return Err(QueryResultAdmissionError::EntityCatalogMismatch);
    }
    if candidate.binding.snapshot_digest != *query.snapshot_digest() {
        return Err(QueryResultAdmissionError::SnapshotMismatch);
    }
    Ok(())
}

fn validate_shape(
    query: &CatalogBoundQuery,
    candidate: &CandidateQueryResult,
    limits: QueryResultLimits,
) -> Result<(), QueryResultAdmissionError> {
    if candidate.rows.len() > limits.max_rows.get() {
        return Err(QueryResultAdmissionError::RowLimitExceeded {
            limit: limits.max_rows.get(),
            actual: candidate.rows.len(),
        });
    }
    if matches!(query.query().result().mode(), ResultMode::Finish) && !candidate.rows.is_empty() {
        return Err(QueryResultAdmissionError::UnexpectedRowsForFinish {
            actual: candidate.rows.len(),
        });
    }
    if let Some(PageValue::Literal(limit)) = query.query().limit()
        && candidate.rows.len() > *limit as usize
    {
        return Err(QueryResultAdmissionError::ResultLimitExceeded {
            limit: *limit as usize,
            actual: candidate.rows.len(),
        });
    }
    let fields = query.static_typing().result_fields();
    if candidate.columns.len() != fields.len() {
        return Err(QueryResultAdmissionError::ColumnCountMismatch {
            expected: fields.len(),
            actual: candidate.columns.len(),
        });
    }
    for (column, (actual, expected)) in candidate.columns.iter().zip(fields).enumerate() {
        if actual != expected.name() {
            return Err(QueryResultAdmissionError::ColumnNameMismatch {
                column,
                expected: expected.name().clone(),
                actual: actual.clone(),
            });
        }
    }
    let cells = candidate.rows.len().saturating_mul(candidate.columns.len());
    if cells > limits.max_cells.get() {
        return Err(QueryResultAdmissionError::CellLimitExceeded {
            limit: limits.max_cells.get(),
            actual: cells,
        });
    }
    for (row_index, row) in candidate.rows.iter().enumerate() {
        if row.len() != fields.len() {
            return Err(QueryResultAdmissionError::RowWidthMismatch {
                row: row_index,
                expected: fields.len(),
                actual: row.len(),
            });
        }
        for (column_index, (value, field)) in row.iter().zip(fields).enumerate() {
            validate_value(field.expression_type(), value, row_index, column_index)?;
        }
    }
    if matches!(
        query.query().result().mode(),
        ResultMode::Return(SetQuantifier::Distinct)
    ) {
        let mut rows = BTreeSet::new();
        for (row_index, row) in candidate.rows.iter().enumerate() {
            let encoded = encode(row)?;
            if !rows.insert(encoded) {
                return Err(QueryResultAdmissionError::DuplicateDistinctRow { row: row_index });
            }
        }
    }
    Ok(())
}

fn validate_value(
    expected: &ExpressionType,
    value: &QueryResultValue,
    row: usize,
    column: usize,
) -> Result<(), QueryResultAdmissionError> {
    if matches!(value, QueryResultValue::Null) {
        return if expected.nullable() || matches!(expected.query_type(), QueryType::Null) {
            Ok(())
        } else {
            Err(QueryResultAdmissionError::NullNotAllowed { row, column })
        };
    }
    match (expected.query_type(), value) {
        (QueryType::Schema(expected_schema), QueryResultValue::Scalar { schema, value }) => {
            if schema != expected_schema {
                return Err(QueryResultAdmissionError::ScalarSchemaMismatch {
                    row,
                    column,
                    expected: expected_schema.clone(),
                    actual: schema.clone(),
                });
            }
            validate_scalar(schema, value, row, column)
        }
        (QueryType::Kind(expected_kind), QueryResultValue::Scalar { schema, value }) => {
            if schema.kind() != *expected_kind {
                return Err(QueryResultAdmissionError::ScalarKindMismatch {
                    row,
                    column,
                    expected: *expected_kind,
                    actual: schema.kind(),
                });
            }
            validate_scalar(schema, value, row, column)
        }
        (QueryType::Numeric, QueryResultValue::Scalar { schema, value })
            if is_numeric(schema.kind()) =>
        {
            validate_scalar(schema, value, row, column)
        }
        (QueryType::AnyValue, value) => validate_self_describing(value, row, column),
        (QueryType::Node(types), QueryResultValue::Node { entity_type, .. })
            if types.contains(entity_type) =>
        {
            Ok(())
        }
        (QueryType::Relation(types), QueryResultValue::Relation { relation_type, .. })
            if types.contains(relation_type) =>
        {
            Ok(())
        }
        (QueryType::List(element), QueryResultValue::List(values)) => {
            for value in values {
                validate_value(element, value, row, column)?;
            }
            Ok(())
        }
        (query_type, value) => Err(QueryResultAdmissionError::ValueTypeMismatch {
            row,
            column,
            expected: query_type.clone(),
            actual: value.kind(),
        }),
    }
}

fn validate_self_describing(
    value: &QueryResultValue,
    row: usize,
    column: usize,
) -> Result<(), QueryResultAdmissionError> {
    match value {
        QueryResultValue::Null
        | QueryResultValue::Node { .. }
        | QueryResultValue::Relation { .. } => Ok(()),
        QueryResultValue::Scalar { schema, value } => validate_scalar(schema, value, row, column),
        QueryResultValue::List(values) => {
            for value in values {
                validate_self_describing(value, row, column)?;
            }
            Ok(())
        }
    }
}

fn validate_scalar(
    schema: &ValueSchema,
    value: &Value,
    row: usize,
    column: usize,
) -> Result<(), QueryResultAdmissionError> {
    schema
        .validate()
        .and_then(|()| schema.validate_value(value))
        .map_err(|error| QueryResultAdmissionError::InvalidScalar { row, column, error })
}

const fn is_numeric(kind: ValueKind) -> bool {
    matches!(
        kind,
        ValueKind::Integer | ValueKind::Decimal | ValueKind::Float
    )
}

fn digest_candidate(
    candidate: &CandidateQueryResult,
) -> Result<[u8; 32], QueryResultAdmissionError> {
    let body = encode(&(&candidate.columns, &candidate.rows))?;
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, QUERY_RESULT_ADMISSION_SCHEMA);
    hash_field(&mut hasher, &candidate.binding.query_binding_digest);
    hash_field(&mut hasher, candidate.binding.generation.digest_bytes());
    hash_field(
        &mut hasher,
        candidate.binding.relation_catalog_digest.as_bytes(),
    );
    hash_field(
        &mut hasher,
        candidate.binding.entity_catalog_digest.as_bytes(),
    );
    hash_field(&mut hasher, &candidate.binding.snapshot_digest);
    hash_field(&mut hasher, &body);
    Ok(hasher.finalize().into())
}

fn encode<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, QueryResultAdmissionError> {
    let mut encoded = Vec::new();
    ciborium::into_writer(value, &mut encoded)
        .map_err(|error| QueryResultAdmissionError::Encoding(error.to_string()))?;
    Ok(encoded)
}

fn hash_field(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}
