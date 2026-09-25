// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Canonical identity for the entity and property schemas admitted by a bundle.

use std::{collections::BTreeSet, fmt};

use mrr_identity::EntityId;
use mrr_relation::{EntitySchema, RelationError};
use sha2::{Digest, Sha256};

const ENTITY_CATALOG_SCHEMA: &[u8] = b"mrr.entity-catalog.v1";

/// Digest of one canonically ordered entity/property catalog.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EntityCatalogDigest([u8; 32]);

impl EntityCatalogDigest {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// A validated, canonically ordered collection of entity type schemas.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityCatalog {
    entities: Vec<EntitySchema>,
    digest: EntityCatalogDigest,
}

/// Reasons an entity schema collection cannot define one catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EntityCatalogError {
    DuplicateEntity(EntityId),
    InvalidSchema {
        entity: EntityId,
        error: RelationError,
    },
    Encoding(String),
}

impl fmt::Display for EntityCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for EntityCatalogError {}

impl EntityCatalog {
    /// Validates, orders, and identifies entity/property schemas.
    pub fn admit(mut entities: Vec<EntitySchema>) -> Result<Self, EntityCatalogError> {
        entities = entities.into_iter().map(EntitySchema::normalized).collect();
        entities.sort_by_key(EntitySchema::id);
        let mut ids = BTreeSet::new();
        for entity in &entities {
            entity
                .validate()
                .map_err(|error| EntityCatalogError::InvalidSchema {
                    entity: entity.id(),
                    error,
                })?;
            if !ids.insert(entity.id()) {
                return Err(EntityCatalogError::DuplicateEntity(entity.id()));
            }
        }
        Self::encode(entities)
    }

    pub(crate) fn from_validated(entities: Vec<EntitySchema>) -> Result<Self, EntityCatalogError> {
        Self::encode(entities)
    }

    fn encode(entities: Vec<EntitySchema>) -> Result<Self, EntityCatalogError> {
        let mut canonical = Vec::new();
        ciborium::into_writer(&entities, &mut canonical)
            .map_err(|error| EntityCatalogError::Encoding(error.to_string()))?;
        let mut hasher = Sha256::new();
        hasher.update(ENTITY_CATALOG_SCHEMA);
        hasher.update((canonical.len() as u64).to_be_bytes());
        hasher.update(canonical);
        Ok(Self {
            entities,
            digest: EntityCatalogDigest(hasher.finalize().into()),
        })
    }

    #[must_use]
    pub fn entities(&self) -> &[EntitySchema] {
        &self.entities
    }

    #[must_use]
    pub const fn digest(&self) -> EntityCatalogDigest {
        self.digest
    }

    #[must_use]
    pub fn entity(&self, id: EntityId) -> Option<&EntitySchema> {
        self.entities
            .binary_search_by_key(&id, EntitySchema::id)
            .ok()
            .map(|index| &self.entities[index])
    }
}
