//! Storage-neutral query binding to semantic catalog and source snapshot identity.

use std::fmt;

use mrr_bundle::{EntityCatalogDigest, ReasoningBundle, RelationCatalogDigest};
use mrr_identity::{EntityId, FactId, GenerationId, QueryId, RelationId};
use mrr_query::{Binding, MetaQueryIr, Parameter, PropertyKey, QueryIrError};
use mrr_relation::{ValueKind, ValueSchema};
use mrr_revision::SemanticSnapshot;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const QUERY_CATALOG_BINDING_SCHEMA: &[u8] = b"mrr.query-catalog-binding.v1";

/// A query bound to one semantic relation catalog and source snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogBoundQuery {
    query: MetaQueryIr,
    generation: GenerationId,
    catalog: RelationCatalogDigest,
    entity_catalog: EntityCatalogDigest,
    typing: crate::StaticQueryTyping,
    snapshot_digest: [u8; 32],
    query_digest: [u8; 32],
    digest: [u8; 32],
}

/// A property access resolved to the exact schema admitted by the bundle.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResolvedProperty {
    pub(crate) binding: Binding,
    pub(crate) key: PropertyKey,
    pub(crate) schema: ValueSchema,
    pub(crate) nullable: bool,
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
    /// A graph node type is absent from the exact entity catalog.
    UnknownEntity(EntityId),
    /// An expression references a binding not introduced by the graph or result scope.
    UnknownBinding(Binding),
    /// One binding was used for incompatible graph element kinds or result aliases.
    ConflictingBinding(Binding),
    /// A property access has no exact node or relation type constraint.
    UnconstrainedPropertyBinding(Binding),
    /// The property does not exist on every possible type of its binding.
    UnknownProperty { binding: Binding, key: PropertyKey },
    /// Possible types disagree about the property's value schema or nullability.
    ConflictingPropertySchema { binding: Binding, key: PropertyKey },
    /// A parameter has no catalog- or context-derived type constraint.
    UnconstrainedParameter(Parameter),
    /// Repeated uses impose incompatible schemas on one parameter.
    ConflictingParameterType(Parameter),
    /// An operator, filter, aggregation, or result expression has the wrong type.
    TypeMismatch {
        context: &'static str,
        expected: String,
        actual: String,
    },
    /// Two expression operands do not have a compatible common type.
    IncompatibleOperands {
        operator: &'static str,
        left: String,
        right: String,
    },
    /// A value category has no admitted semantics for the selected operation.
    UnsupportedValueKind {
        context: &'static str,
        kind: ValueKind,
    },
    /// A non-aggregate output is not present in the explicit grouping keys.
    UngroupedProjection(Binding),
    /// Bundle facts cannot be mixed with a snapshot from another generation.
    FactGenerationMismatch {
        fact: FactId,
        expected: GenerationId,
        actual: GenerationId,
    },
    /// The already-admitted query could not be canonically encoded.
    QueryEncoding(QueryIrError),
    /// Canonical static-typing receipt encoding failed.
    StaticTypingEncoding(String),
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

    /// Returns the exact entity/property catalog digest.
    #[must_use]
    pub const fn entity_catalog_digest(&self) -> EntityCatalogDigest {
        self.entity_catalog
    }

    /// Returns property accesses resolved during static admission.
    #[must_use]
    pub fn resolved_properties(&self) -> &[ResolvedProperty] {
        self.typing.resolved_properties()
    }

    /// Returns the canonical static typing proof for parameters and output rows.
    #[must_use]
    pub const fn static_typing(&self) -> &crate::StaticQueryTyping {
        &self.typing
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

impl ResolvedProperty {
    #[must_use]
    pub const fn binding(&self) -> &Binding {
        &self.binding
    }

    #[must_use]
    pub const fn key(&self) -> &PropertyKey {
        &self.key
    }

    #[must_use]
    pub const fn schema(&self) -> &ValueSchema {
        &self.schema
    }

    #[must_use]
    pub const fn nullable(&self) -> bool {
        self.nullable
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
    let entity_catalog = bundle.entity_catalog();
    for relation in template.query().referenced_relations() {
        if catalog.relation(relation).is_none() {
            return Err(QueryCatalogBindingError::UnknownRelation(relation));
        }
    }
    for entity in template.query().referenced_entities() {
        if entity_catalog.entity(entity).is_none() {
            return Err(QueryCatalogBindingError::UnknownEntity(entity));
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
    let typing = crate::typing::type_query(&query, bundle)?;
    let mut typing_proof = Vec::new();
    ciborium::into_writer(&typing, &mut typing_proof)
        .map_err(|error| QueryCatalogBindingError::StaticTypingEncoding(error.to_string()))?;
    let canonical = query
        .encode_canonical()
        .map_err(QueryCatalogBindingError::QueryEncoding)?;
    let query_digest: [u8; 32] = Sha256::digest(&canonical).into();
    let catalog_digest = catalog.digest();
    let entity_catalog_digest = entity_catalog.digest();
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, QUERY_CATALOG_BINDING_SCHEMA);
    hash_field(&mut hasher, &query_digest);
    hash_field(&mut hasher, catalog_digest.as_bytes());
    hash_field(&mut hasher, entity_catalog_digest.as_bytes());
    hash_field(&mut hasher, &typing_proof);
    hash_field(&mut hasher, snapshot.generation().digest_bytes());
    hash_field(&mut hasher, snapshot.digest());
    Ok(CatalogBoundQuery {
        query,
        generation: snapshot.generation(),
        catalog: catalog_digest,
        entity_catalog: entity_catalog_digest,
        typing,
        snapshot_digest: *snapshot.digest(),
        query_digest,
        digest: hasher.finalize().into(),
    })
}

fn hash_field(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}
