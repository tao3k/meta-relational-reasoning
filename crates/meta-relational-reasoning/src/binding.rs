//! Storage-neutral query binding to semantic catalog and source snapshot identity.

use std::{collections::BTreeMap, collections::BTreeSet, fmt};

use mrr_bundle::{EntityCatalogDigest, ReasoningBundle, RelationCatalogDigest};
use mrr_identity::{EntityId, FactId, GenerationId, QueryId, RelationId};
use mrr_query::{Binding, Expression, MetaQueryIr, PropertyKey, QueryIrError};
use mrr_relation::{RelationField, ValueSchema};
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
    properties: Vec<ResolvedProperty>,
    snapshot_digest: [u8; 32],
    query_digest: [u8; 32],
    digest: [u8; 32],
}

/// A property access resolved to the exact schema admitted by the bundle.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResolvedProperty {
    binding: Binding,
    key: PropertyKey,
    schema: ValueSchema,
    nullable: bool,
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
    /// Bundle facts cannot be mixed with a snapshot from another generation.
    FactGenerationMismatch {
        fact: FactId,
        expected: GenerationId,
        actual: GenerationId,
    },
    /// The already-admitted query could not be canonically encoded.
    QueryEncoding(QueryIrError),
    /// Canonical property-resolution proof encoding failed.
    PropertyProofEncoding(String),
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
        &self.properties
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
    let properties = resolve_properties(&query, bundle)?;
    let mut property_proof = Vec::new();
    ciborium::into_writer(&properties, &mut property_proof)
        .map_err(|error| QueryCatalogBindingError::PropertyProofEncoding(error.to_string()))?;
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
    hash_field(&mut hasher, &property_proof);
    hash_field(&mut hasher, snapshot.generation().digest_bytes());
    hash_field(&mut hasher, snapshot.digest());
    Ok(CatalogBoundQuery {
        query,
        generation: snapshot.generation(),
        catalog: catalog_digest,
        entity_catalog: entity_catalog_digest,
        properties,
        snapshot_digest: *snapshot.digest(),
        query_digest,
        digest: hasher.finalize().into(),
    })
}

#[derive(Clone, Debug)]
enum BindingTarget {
    Node(BTreeSet<EntityId>),
    Relation(BTreeSet<RelationId>),
    Scalar,
}

fn resolve_properties(
    query: &MetaQueryIr,
    bundle: &ReasoningBundle,
) -> Result<Vec<ResolvedProperty>, QueryCatalogBindingError> {
    let mut targets = BTreeMap::<Binding, BindingTarget>::new();
    for path in query.graph().paths() {
        register_node(&mut targets, path.start().binding(), path.start().types())?;
        for segment in path.segments() {
            if let Some(binding) = segment.relation().binding() {
                register_relation(&mut targets, binding, segment.relation().types())?;
            }
            register_node(
                &mut targets,
                segment.node().binding(),
                segment.node().types(),
            )?;
        }
    }

    let mut resolved = Vec::new();
    for filter in query.filters() {
        validate_expression(filter.predicate(), &targets, bundle, &mut resolved)?;
    }
    for projection in query.projections() {
        validate_expression(projection.expression(), &targets, bundle, &mut resolved)?;
    }
    for aggregation in query.aggregations() {
        for expression in aggregation.expressions() {
            validate_expression(expression, &targets, bundle, &mut resolved)?;
        }
    }
    for grouping in query.grouping() {
        validate_expression(grouping.expression(), &targets, bundle, &mut resolved)?;
    }

    for projection in query.projections() {
        register_scalar(&mut targets, projection.alias())?;
    }
    for aggregation in query.aggregations() {
        register_scalar(&mut targets, aggregation.alias())?;
    }
    for ordering in query.ordering() {
        validate_expression(ordering.expression(), &targets, bundle, &mut resolved)?;
    }

    resolved.sort_by(|left, right| {
        (left.binding.as_str(), left.key.as_str())
            .cmp(&(right.binding.as_str(), right.key.as_str()))
    });
    resolved.dedup_by(|left, right| left.binding == right.binding && left.key == right.key);
    Ok(resolved)
}

fn register_node(
    targets: &mut BTreeMap<Binding, BindingTarget>,
    binding: &Binding,
    types: &[EntityId],
) -> Result<(), QueryCatalogBindingError> {
    register_target(
        targets,
        binding,
        BindingTarget::Node(types.iter().copied().collect()),
    )
}

fn register_relation(
    targets: &mut BTreeMap<Binding, BindingTarget>,
    binding: &Binding,
    types: &[RelationId],
) -> Result<(), QueryCatalogBindingError> {
    register_target(
        targets,
        binding,
        BindingTarget::Relation(types.iter().copied().collect()),
    )
}

fn register_scalar(
    targets: &mut BTreeMap<Binding, BindingTarget>,
    binding: &Binding,
) -> Result<(), QueryCatalogBindingError> {
    register_target(targets, binding, BindingTarget::Scalar)
}

fn register_target(
    targets: &mut BTreeMap<Binding, BindingTarget>,
    binding: &Binding,
    target: BindingTarget,
) -> Result<(), QueryCatalogBindingError> {
    match targets.get_mut(binding) {
        None => {
            targets.insert(binding.clone(), target);
            Ok(())
        }
        Some(BindingTarget::Node(existing)) => match target {
            BindingTarget::Node(types) => {
                existing.extend(types);
                Ok(())
            }
            _ => Err(QueryCatalogBindingError::ConflictingBinding(
                binding.clone(),
            )),
        },
        Some(BindingTarget::Relation(existing)) => match target {
            BindingTarget::Relation(types) => {
                existing.extend(types);
                Ok(())
            }
            _ => Err(QueryCatalogBindingError::ConflictingBinding(
                binding.clone(),
            )),
        },
        Some(BindingTarget::Scalar) => Err(QueryCatalogBindingError::ConflictingBinding(
            binding.clone(),
        )),
    }
}

fn validate_expression(
    expression: &Expression,
    targets: &BTreeMap<Binding, BindingTarget>,
    bundle: &ReasoningBundle,
    resolved: &mut Vec<ResolvedProperty>,
) -> Result<(), QueryCatalogBindingError> {
    match expression {
        Expression::Binding(binding) => {
            require_target(targets, binding)?;
        }
        Expression::Property { binding, key } => {
            let target = require_target(targets, binding)?;
            let field = resolve_property(target, binding, key, bundle)?;
            resolved.push(ResolvedProperty {
                binding: binding.clone(),
                key: key.clone(),
                schema: field.schema().clone(),
                nullable: field.nullable(),
            });
        }
        Expression::Unary { operand, .. } => {
            validate_expression(operand, targets, bundle, resolved)?;
        }
        Expression::Binary { left, right, .. } => {
            validate_expression(left, targets, bundle, resolved)?;
            validate_expression(right, targets, bundle, resolved)?;
        }
        Expression::Parameter(_) | Expression::Literal(_) => {}
    }
    Ok(())
}

fn require_target<'a>(
    targets: &'a BTreeMap<Binding, BindingTarget>,
    binding: &Binding,
) -> Result<&'a BindingTarget, QueryCatalogBindingError> {
    targets
        .get(binding)
        .ok_or_else(|| QueryCatalogBindingError::UnknownBinding(binding.clone()))
}

fn resolve_property<'a>(
    target: &BindingTarget,
    binding: &Binding,
    key: &PropertyKey,
    bundle: &'a ReasoningBundle,
) -> Result<&'a RelationField, QueryCatalogBindingError> {
    let mut fields = Vec::new();
    match target {
        BindingTarget::Node(types) => {
            if types.is_empty() {
                return Err(QueryCatalogBindingError::UnconstrainedPropertyBinding(
                    binding.clone(),
                ));
            }
            for entity in types {
                let schema = bundle
                    .entity_catalog()
                    .entity(*entity)
                    .ok_or(QueryCatalogBindingError::UnknownEntity(*entity))?;
                fields.push(schema.property(key.as_str()).ok_or_else(|| {
                    QueryCatalogBindingError::UnknownProperty {
                        binding: binding.clone(),
                        key: key.clone(),
                    }
                })?);
            }
        }
        BindingTarget::Relation(types) => {
            if types.is_empty() {
                return Err(QueryCatalogBindingError::UnconstrainedPropertyBinding(
                    binding.clone(),
                ));
            }
            for relation in types {
                let schema = bundle
                    .relation_catalog()
                    .relation(*relation)
                    .ok_or(QueryCatalogBindingError::UnknownRelation(*relation))?;
                fields.push(
                    schema
                        .fields()
                        .iter()
                        .find(|field| field.name() == key.as_str())
                        .ok_or_else(|| QueryCatalogBindingError::UnknownProperty {
                            binding: binding.clone(),
                            key: key.clone(),
                        })?,
                );
            }
        }
        BindingTarget::Scalar => {
            return Err(QueryCatalogBindingError::UnconstrainedPropertyBinding(
                binding.clone(),
            ));
        }
    }
    let first = fields[0];
    if fields
        .iter()
        .skip(1)
        .any(|field| field.schema() != first.schema() || field.nullable() != first.nullable())
    {
        return Err(QueryCatalogBindingError::ConflictingPropertySchema {
            binding: binding.clone(),
            key: key.clone(),
        });
    }
    Ok(first)
}

fn hash_field(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}
