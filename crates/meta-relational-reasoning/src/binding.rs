//! Storage-neutral query binding to semantic catalog and source snapshot identity.

use std::fmt;

use mrr_bundle::{ReasoningBundle, RelationCatalogDigest};
use mrr_identity::{FactId, GenerationId, QueryId, RelationId};
use mrr_query::{MetaQueryIr, QueryIrError};
use mrr_revision::SemanticSnapshot;
use sha2::{Digest, Sha256};

const QUERY_CATALOG_BINDING_SCHEMA: &[u8] = b"mrr.query-catalog-binding.v1";

/// A query bound to one semantic relation catalog and source snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogBoundQuery {
    query: MetaQueryIr,
    generation: GenerationId,
    catalog: RelationCatalogDigest,
    snapshot_digest: [u8; 32],
    query_digest: [u8; 32],
    digest: [u8; 32],
}

/// Reasons a query cannot be bound to a catalog and semantic snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QueryCatalogBindingError {
    /// The bundle does not contain the requested query template.
    UnknownQuery(QueryId),
    /// Dependent templates need a separately admitted query-plan graph.
    DependentQueryTemplateUnsupported(QueryId),
    /// A graph relation reference is absent from the exact catalog.
    UnknownRelation(RelationId),
    /// Bundle facts cannot be mixed with a snapshot from another generation.
    FactGenerationMismatch {
        fact: FactId,
        expected: GenerationId,
        actual: GenerationId,
    },
    /// The already-admitted query could not be canonically encoded.
    QueryEncoding(QueryIrError),
}

impl fmt::Display for QueryCatalogBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for QueryCatalogBindingError {}

impl CatalogBoundQuery {
    /// Returns the exact canonical query.
    #[must_use]
    pub const fn query(&self) -> &MetaQueryIr {
        &self.query
    }

    /// Returns the semantic generation shared with the snapshot.
    #[must_use]
    pub const fn generation(&self) -> GenerationId {
        self.generation
    }

    /// Returns the exact relation catalog digest.
    #[must_use]
    pub const fn catalog_digest(&self) -> RelationCatalogDigest {
        self.catalog
    }

    /// Returns the exact source snapshot digest.
    #[must_use]
    pub const fn snapshot_digest(&self) -> &[u8; 32] {
        &self.snapshot_digest
    }

    /// Returns the canonical query digest.
    #[must_use]
    pub const fn query_digest(&self) -> &[u8; 32] {
        &self.query_digest
    }

    /// Returns the complete V1 query-catalog binding digest.
    #[must_use]
    pub const fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
}

/// Binds an admitted query to an exact relation catalog and semantic snapshot.
pub fn bind_query_to_catalog(
    bundle: &ReasoningBundle,
    query_id: QueryId,
    snapshot: &SemanticSnapshot,
) -> Result<CatalogBoundQuery, QueryCatalogBindingError> {
    let template = bundle
        .query_templates()
        .iter()
        .find(|template| template.id() == query_id)
        .ok_or(QueryCatalogBindingError::UnknownQuery(query_id))?;
    if !template.dependencies().is_empty() {
        return Err(QueryCatalogBindingError::DependentQueryTemplateUnsupported(
            query_id,
        ));
    }
    let catalog = bundle.relation_catalog();
    for relation in template.query().referenced_relations() {
        if catalog.relation(relation).is_none() {
            return Err(QueryCatalogBindingError::UnknownRelation(relation));
        }
    }
    for fact in bundle.facts() {
        let actual = fact.context().generation();
        if actual != snapshot.generation() {
            return Err(QueryCatalogBindingError::FactGenerationMismatch {
                fact: fact.id(),
                expected: snapshot.generation(),
                actual,
            });
        }
    }

    let query = template.query().clone();
    let canonical = query
        .encode_canonical()
        .map_err(QueryCatalogBindingError::QueryEncoding)?;
    let query_digest: [u8; 32] = Sha256::digest(&canonical).into();
    let catalog_digest = catalog.digest();
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, QUERY_CATALOG_BINDING_SCHEMA);
    hash_field(&mut hasher, &query_digest);
    hash_field(&mut hasher, catalog_digest.as_bytes());
    hash_field(&mut hasher, snapshot.generation().digest_bytes());
    hash_field(&mut hasher, snapshot.digest());
    Ok(CatalogBoundQuery {
        query,
        generation: snapshot.generation(),
        catalog: catalog_digest,
        snapshot_digest: *snapshot.digest(),
        query_digest,
        digest: hasher.finalize().into(),
    })
}

fn hash_field(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}
