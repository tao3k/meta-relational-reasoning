//! Canonical storage-neutral identity for an admitted relation schema set.

use std::{collections::BTreeSet, fmt};

use mrr_identity::RelationId;
use mrr_relation::{RelationError, RelationSchema};
use sha2::{Digest, Sha256};

const RELATION_CATALOG_SCHEMA: &[u8] = b"mrr.relation-catalog.v1";

/// Digest of one canonically ordered and validated relation catalog.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RelationCatalogDigest([u8; 32]);

impl RelationCatalogDigest {
    /// Returns the exact V1 digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// A validated, canonically ordered set of semantic relation schemas.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationCatalog {
    relations: Vec<RelationSchema>,
    digest: RelationCatalogDigest,
}

/// Reasons a relation collection cannot define one catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RelationCatalogError {
    /// A catalog must contain at least one semantic relation.
    Empty,
    /// A relation identity may occur only once.
    DuplicateRelation(RelationId),
    /// A relation schema failed its own semantic validation.
    InvalidSchema {
        relation: RelationId,
        error: RelationError,
    },
    /// Canonical CBOR encoding failed.
    Encoding(String),
}

impl fmt::Display for RelationCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RelationCatalogError {}

impl RelationCatalog {
    /// Validates, orders, and identifies a storage-neutral schema catalog.
    pub fn admit(mut relations: Vec<RelationSchema>) -> Result<Self, RelationCatalogError> {
        if relations.is_empty() {
            return Err(RelationCatalogError::Empty);
        }
        relations.sort_by_key(RelationSchema::id);
        let mut ids = BTreeSet::new();
        for relation in &relations {
            relation
                .validate()
                .map_err(|error| RelationCatalogError::InvalidSchema {
                    relation: relation.id(),
                    error,
                })?;
            if !ids.insert(relation.id()) {
                return Err(RelationCatalogError::DuplicateRelation(relation.id()));
            }
        }
        Self::encode(relations)
    }

    pub(crate) fn from_validated(
        relations: Vec<RelationSchema>,
    ) -> Result<Self, RelationCatalogError> {
        Self::encode(relations)
    }

    fn encode(relations: Vec<RelationSchema>) -> Result<Self, RelationCatalogError> {
        let mut canonical = Vec::new();
        ciborium::into_writer(&relations, &mut canonical)
            .map_err(|error| RelationCatalogError::Encoding(error.to_string()))?;
        let mut hasher = Sha256::new();
        hasher.update(RELATION_CATALOG_SCHEMA);
        hasher.update((canonical.len() as u64).to_be_bytes());
        hasher.update(canonical);
        Ok(Self {
            relations,
            digest: RelationCatalogDigest(hasher.finalize().into()),
        })
    }

    /// Returns schemas in canonical `RelationId` order.
    #[must_use]
    pub fn relations(&self) -> &[RelationSchema] {
        &self.relations
    }

    /// Returns the canonical catalog digest.
    #[must_use]
    pub const fn digest(&self) -> RelationCatalogDigest {
        self.digest
    }

    /// Resolves an exact relation identity.
    #[must_use]
    pub fn relation(&self, id: RelationId) -> Option<&RelationSchema> {
        self.relations
            .binary_search_by_key(&id, RelationSchema::id)
            .ok()
            .map(|index| &self.relations[index])
    }
}
