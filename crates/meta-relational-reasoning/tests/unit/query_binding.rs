use crate::{
    Binding, CatalogBoundQuery, Direction, EntityId, EntitySchema, ExternalRevisionIdentity, Fact,
    FactProvenance, FactValidity, GenerationId, GraphPattern, MetaQueryIr, MrrEngine, NodePattern,
    PathPattern, PathSegment, Projection, PropertyKey, QueryCatalogBindingError, QueryId,
    QueryOperatorId, QueryResult, QueryTemplate, ReasoningBundle, ReasoningBundleDeclaration,
    RelationAuthority, RelationContext, RelationField, RelationId, RelationPattern, RelationSchema,
    RevisionBinding, SemanticSnapshot, SetQuantifier, Value, ValueSchema,
};
use crate::{EvidenceCompleteness, Expression, FactId};

fn id<T>(label: &str, derive: impl FnOnce(&[u8]) -> T) -> T {
    derive(format!("query-binding:{label}").as_bytes())
}

fn relation_schema(relation: RelationId) -> RelationSchema {
    RelationSchema::new(
        relation,
        "depends_on",
        vec![
            RelationField::new("subject", ValueSchema::Entity, false).unwrap(),
            RelationField::new("object", ValueSchema::Entity, false).unwrap(),
        ],
        Vec::new(),
    )
    .unwrap()
}

fn query(query_id: QueryId, relation: RelationId) -> MetaQueryIr {
    let binding = |name| Binding::new(name).unwrap();
    let graph = GraphPattern::new(
        id("graph", |bytes| {
            QueryOperatorId::from_canonical_bytes(bytes).unwrap()
        }),
        vec![PathPattern::new(
            NodePattern::new(binding("source"), Vec::new()),
            vec![PathSegment::new(
                RelationPattern::new(None, vec![relation], Direction::Outgoing, 1, Some(1))
                    .unwrap(),
                NodePattern::new(binding("target"), Vec::new()),
            )],
        )],
    )
    .unwrap();
    MetaQueryIr::new(
        query_id,
        graph,
        Vec::new(),
        QueryResult::returning(SetQuantifier::All).with_projections(vec![Projection::new(
            id("projection", |bytes| {
                QueryOperatorId::from_canonical_bytes(bytes).unwrap()
            }),
            Expression::Binding(binding("target")),
            binding("result"),
        )]),
    )
    .unwrap()
}

fn snapshot(generation: GenerationId, revision: &str) -> SemanticSnapshot {
    let binding = RevisionBinding::admit(
        ExternalRevisionIdentity::new("git", "repository", revision).unwrap(),
        generation,
    )
    .unwrap();
    SemanticSnapshot::admit(generation, vec![binding]).unwrap()
}

fn engine(
    relation: RelationId,
    query_id: QueryId,
    facts: Vec<Fact>,
    dependencies: Vec<QueryId>,
) -> MrrEngine {
    let bundle = ReasoningBundle::admit(ReasoningBundleDeclaration {
        relations: vec![relation_schema(relation)],
        facts,
        query_templates: vec![QueryTemplate::new(query(query_id, relation), dependencies)],
        ..ReasoningBundleDeclaration::default()
    })
    .unwrap();
    MrrEngine::builder().with_bundle(bundle).build().unwrap()
}

fn assert_same_binding(left: &CatalogBoundQuery, right: &CatalogBoundQuery) {
    assert_eq!(left.query(), right.query());
    assert_eq!(left.generation(), right.generation());
    assert_eq!(left.catalog_digest(), right.catalog_digest());
    assert_eq!(left.entity_catalog_digest(), right.entity_catalog_digest());
    assert_eq!(left.resolved_properties(), right.resolved_properties());
    assert_eq!(left.snapshot_digest(), right.snapshot_digest());
    assert_eq!(left.query_digest(), right.query_digest());
    assert_eq!(left.digest(), right.digest());
}

fn property_query(
    query_id: QueryId,
    relation: RelationId,
    entity_types: Vec<EntityId>,
    key: &str,
) -> MetaQueryIr {
    let binding = |name| Binding::new(name).unwrap();
    let graph = GraphPattern::new(
        id("property-graph", |bytes| {
            QueryOperatorId::from_canonical_bytes(bytes).unwrap()
        }),
        vec![PathPattern::new(
            NodePattern::new(binding("source"), entity_types.clone()),
            vec![PathSegment::new(
                RelationPattern::new(
                    Some(binding("edge")),
                    vec![relation],
                    Direction::Outgoing,
                    1,
                    Some(1),
                )
                .unwrap(),
                NodePattern::new(binding("target"), entity_types),
            )],
        )],
    )
    .unwrap();
    MetaQueryIr::new(
        query_id,
        graph,
        Vec::new(),
        QueryResult::returning(SetQuantifier::All).with_projections(vec![Projection::new(
            id("property-projection", |bytes| {
                QueryOperatorId::from_canonical_bytes(bytes).unwrap()
            }),
            Expression::Property {
                binding: binding("source"),
                key: PropertyKey::new(key).unwrap(),
            },
            binding("result"),
        )]),
    )
    .unwrap()
}

fn entity_schema(entity: EntityId, property: ValueSchema) -> EntitySchema {
    EntitySchema::new(
        entity,
        "artifact",
        vec![RelationField::new("name", property, false).unwrap()],
    )
    .unwrap()
}

#[test]
fn query_binding_resolves_property_schema_from_exact_entity_catalog() {
    let relation = id("property-relation", |bytes| {
        RelationId::from_canonical_bytes(bytes).unwrap()
    });
    let entity = id("property-entity", |bytes| {
        EntityId::from_canonical_bytes(bytes).unwrap()
    });
    let query_id = id("property-query", |bytes| {
        QueryId::from_canonical_bytes(bytes).unwrap()
    });
    let generation = id("property-generation", |bytes| {
        GenerationId::from_canonical_bytes(bytes).unwrap()
    });
    let bundle = ReasoningBundle::admit(ReasoningBundleDeclaration {
        entities: vec![entity_schema(entity, ValueSchema::String)],
        relations: vec![relation_schema(relation)],
        query_templates: vec![QueryTemplate::new(
            property_query(query_id, relation, vec![entity], "name"),
            Vec::new(),
        )],
        ..ReasoningBundleDeclaration::default()
    })
    .unwrap();
    let bound = MrrEngine::builder()
        .with_bundle(bundle)
        .build()
        .unwrap()
        .bind_query(query_id, &snapshot(generation, "property-commit"))
        .unwrap();
    assert_eq!(bound.resolved_properties().len(), 1);
    assert_eq!(bound.resolved_properties()[0].binding().as_str(), "source");
    assert_eq!(bound.resolved_properties()[0].key().as_str(), "name");
    assert_eq!(
        bound.resolved_properties()[0].schema(),
        &ValueSchema::String
    );
    assert!(!bound.resolved_properties()[0].nullable());
}

#[test]
fn query_binding_rejects_unconstrained_unknown_and_conflicting_properties() {
    let relation = id("property-errors-relation", |bytes| {
        RelationId::from_canonical_bytes(bytes).unwrap()
    });
    let first = id("property-errors-first", |bytes| {
        EntityId::from_canonical_bytes(bytes).unwrap()
    });
    let second = id("property-errors-second", |bytes| {
        EntityId::from_canonical_bytes(bytes).unwrap()
    });
    let generation = id("property-errors-generation", |bytes| {
        GenerationId::from_canonical_bytes(bytes).unwrap()
    });
    let assert_error =
        |label: &str, types: Vec<EntityId>, key: &str, expected: QueryCatalogBindingError| {
            let query_id = id(label, |bytes| QueryId::from_canonical_bytes(bytes).unwrap());
            let bundle = ReasoningBundle::admit(ReasoningBundleDeclaration {
                entities: vec![
                    entity_schema(first, ValueSchema::String),
                    entity_schema(second, ValueSchema::Integer),
                ],
                relations: vec![relation_schema(relation)],
                query_templates: vec![QueryTemplate::new(
                    property_query(query_id, relation, types, key),
                    Vec::new(),
                )],
                ..ReasoningBundleDeclaration::default()
            })
            .unwrap();
            let engine = MrrEngine::builder().with_bundle(bundle).build().unwrap();
            assert_eq!(
                engine.bind_query(query_id, &snapshot(generation, label)),
                Err(expected)
            );
        };
    let source = Binding::new("source").unwrap();
    assert_error(
        "unconstrained-property",
        Vec::new(),
        "name",
        QueryCatalogBindingError::UnconstrainedPropertyBinding(source.clone()),
    );
    assert_error(
        "unknown-property",
        vec![first],
        "missing",
        QueryCatalogBindingError::UnknownProperty {
            binding: source.clone(),
            key: PropertyKey::new("missing").unwrap(),
        },
    );
    assert_error(
        "conflicting-property",
        vec![first, second],
        "name",
        QueryCatalogBindingError::ConflictingPropertySchema {
            binding: source,
            key: PropertyKey::new("name").unwrap(),
        },
    );
}

#[test]
fn query_binding_is_deterministic_and_binds_catalog_snapshot_and_generation() {
    let relation = id("relation", |bytes| {
        RelationId::from_canonical_bytes(bytes).unwrap()
    });
    let query_id = id("query", |bytes| {
        QueryId::from_canonical_bytes(bytes).unwrap()
    });
    let generation = id("generation", |bytes| {
        GenerationId::from_canonical_bytes(bytes).unwrap()
    });
    let engine = engine(relation, query_id, Vec::new(), Vec::new());
    let semantic_snapshot = snapshot(generation, "commit-a");
    let first = engine.bind_query(query_id, &semantic_snapshot).unwrap();
    let second = engine.bind_query(query_id, &semantic_snapshot).unwrap();
    assert_same_binding(&first, &second);
    assert_eq!(first.generation(), generation);

    let next = snapshot(generation, "commit-b");
    let rebound = engine.bind_query(query_id, &next).unwrap();
    assert_ne!(first.snapshot_digest(), rebound.snapshot_digest());
    assert_ne!(first.digest(), rebound.digest());
}

#[test]
fn query_binding_rejects_cross_generation_facts_and_unowned_dependency_graphs() {
    let relation = id("relation-errors", |bytes| {
        RelationId::from_canonical_bytes(bytes).unwrap()
    });
    let query_id = id("query-errors", |bytes| {
        QueryId::from_canonical_bytes(bytes).unwrap()
    });
    let old_generation = id("generation-old", |bytes| {
        GenerationId::from_canonical_bytes(bytes).unwrap()
    });
    let current_generation = id("generation-current", |bytes| {
        GenerationId::from_canonical_bytes(bytes).unwrap()
    });
    let authority = id("authority", |bytes| {
        EntityId::from_canonical_bytes(bytes).unwrap()
    });
    let fact_id = id("fact", |bytes| FactId::from_canonical_bytes(bytes).unwrap());
    let fact = Fact::new(
        fact_id,
        relation,
        vec![Value::Entity(authority), Value::Entity(authority)],
        RelationContext::new(
            old_generation,
            RelationAuthority::Entity(authority),
            FactProvenance::Source(authority),
            EvidenceCompleteness::Complete,
            FactValidity::Valid,
        )
        .unwrap(),
    );
    let engine = engine(relation, query_id, vec![fact], Vec::new());
    assert_eq!(
        engine.bind_query(query_id, &snapshot(current_generation, "commit-current")),
        Err(QueryCatalogBindingError::FactGenerationMismatch {
            fact: fact_id,
            expected: current_generation,
            actual: old_generation,
        })
    );

    let dependency = id("dependency", |bytes| {
        QueryId::from_canonical_bytes(bytes).unwrap()
    });
    let declaration = ReasoningBundleDeclaration {
        relations: vec![relation_schema(relation)],
        query_templates: vec![
            QueryTemplate::new(query(query_id, relation), vec![dependency]),
            QueryTemplate::new(query(dependency, relation), Vec::new()),
        ],
        ..ReasoningBundleDeclaration::default()
    };
    let engine = MrrEngine::builder()
        .with_bundle(ReasoningBundle::admit(declaration).unwrap())
        .build()
        .unwrap();
    assert_eq!(
        engine.bind_query(query_id, &snapshot(current_generation, "commit-current")),
        Err(QueryCatalogBindingError::DependentQueryTemplateUnsupported(
            query_id
        ))
    );
}
