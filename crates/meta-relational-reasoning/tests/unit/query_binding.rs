use crate::{
    Aggregation, AggregationFunction, BinaryOperator, Binding, CatalogBoundQuery, Direction,
    EntityId, EntitySchema, ExternalRevisionIdentity, Fact, FactProvenance, FactValidity, Filter,
    GenerationId, GraphPattern, Grouping, MetaQueryIr, MrrEngine, NodePattern, Ordering, PageValue,
    Parameter, PathPattern, PathSegment, Projection, PropertyKey, QueryCatalogBindingError,
    QueryId, QueryOperatorId, QueryResult, QueryTemplate, QueryType, ReasoningBundle,
    ReasoningBundleDeclaration, RelationAuthority, RelationContext, RelationField, RelationId,
    RelationPattern, RelationSchema, RevisionBinding, SemanticSnapshot, SetQuantifier,
    SortDirection, Value, ValueSchema,
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

struct TypedResultSpec {
    projections: Vec<Projection>,
    aggregations: Vec<Aggregation>,
    group_aggregates: bool,
    limit: Option<PageValue>,
}

fn typed_query(
    query_id: QueryId,
    relation: RelationId,
    entity: EntityId,
    filter: Expression,
    result: TypedResultSpec,
) -> MetaQueryIr {
    let binding = |name| Binding::new(name).unwrap();
    let grouping = if result.aggregations.is_empty() || !result.group_aggregates {
        Vec::new()
    } else {
        result
            .projections
            .iter()
            .enumerate()
            .map(|(index, projection)| {
                Grouping::new(
                    id(&format!("typed-grouping-{index}"), |bytes| {
                        QueryOperatorId::from_canonical_bytes(bytes).unwrap()
                    }),
                    projection.expression().clone(),
                )
            })
            .collect()
    };
    let graph = GraphPattern::new(
        id("typed-graph", |bytes| {
            QueryOperatorId::from_canonical_bytes(bytes).unwrap()
        }),
        vec![PathPattern::new(
            NodePattern::new(binding("source"), vec![entity]),
            vec![PathSegment::new(
                RelationPattern::new(
                    Some(binding("edge")),
                    vec![relation],
                    Direction::Outgoing,
                    1,
                    Some(1),
                )
                .unwrap(),
                NodePattern::new(binding("target"), vec![entity]),
            )],
        )],
    )
    .unwrap();
    MetaQueryIr::new(
        query_id,
        graph,
        vec![Filter::new(
            id("typed-filter", |bytes| {
                QueryOperatorId::from_canonical_bytes(bytes).unwrap()
            }),
            filter,
        )],
        QueryResult::returning(SetQuantifier::All)
            .with_projections(result.projections)
            .with_aggregations(result.aggregations)
            .with_grouping(grouping)
            .with_ordering(vec![Ordering::new(
                id("typed-order", |bytes| {
                    QueryOperatorId::from_canonical_bytes(bytes).unwrap()
                }),
                Expression::Binding(binding("label")),
                SortDirection::Ascending,
            )])
            .with_limit(result.limit),
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
fn query_binding_admits_parameter_types_aggregates_and_result_row_schema() {
    let relation = id("typing-relation", |bytes| {
        RelationId::from_canonical_bytes(bytes).unwrap()
    });
    let entity = id("typing-entity", |bytes| {
        EntityId::from_canonical_bytes(bytes).unwrap()
    });
    let query_id = id("typing-query", |bytes| {
        QueryId::from_canonical_bytes(bytes).unwrap()
    });
    let generation = id("typing-generation", |bytes| {
        GenerationId::from_canonical_bytes(bytes).unwrap()
    });
    let binding = |name| Binding::new(name).unwrap();
    let property = Expression::Property {
        binding: binding("source"),
        key: PropertyKey::new("name").unwrap(),
    };
    let query = typed_query(
        query_id,
        relation,
        entity,
        Expression::Binary {
            left: Box::new(property.clone()),
            operator: BinaryOperator::Equal,
            right: Box::new(Expression::Parameter(Parameter::new("name").unwrap())),
        },
        TypedResultSpec {
            projections: vec![Projection::new(
                id("typed-projection", |bytes| {
                    QueryOperatorId::from_canonical_bytes(bytes).unwrap()
                }),
                property,
                binding("label"),
            )],
            aggregations: vec![Aggregation::new(
                id("typed-count", |bytes| {
                    QueryOperatorId::from_canonical_bytes(bytes).unwrap()
                }),
                AggregationFunction::Count,
                None,
                Vec::new(),
                true,
                binding("count"),
            )],
            group_aggregates: true,
            limit: Some(PageValue::Parameter(Parameter::new("limit").unwrap())),
        },
    );
    let bundle = ReasoningBundle::admit(ReasoningBundleDeclaration {
        entities: vec![entity_schema(entity, ValueSchema::String)],
        relations: vec![relation_schema(relation)],
        query_templates: vec![QueryTemplate::new(query, Vec::new())],
        ..ReasoningBundleDeclaration::default()
    })
    .unwrap();
    let bound = MrrEngine::builder()
        .with_bundle(bundle)
        .build()
        .unwrap()
        .bind_query(query_id, &snapshot(generation, "typing-commit"))
        .unwrap();

    let typing = bound.static_typing();
    assert_eq!(typing.parameters().len(), 2);
    assert_eq!(typing.parameters()[0].parameter().as_str(), "limit");
    assert_eq!(
        typing.parameters()[0].expression_type().query_type(),
        &QueryType::Schema(ValueSchema::Integer)
    );
    assert_eq!(typing.parameters()[1].parameter().as_str(), "name");
    assert_eq!(
        typing.parameters()[1].expression_type().query_type(),
        &QueryType::Schema(ValueSchema::String)
    );
    assert_eq!(typing.result_fields().len(), 2);
    assert_eq!(typing.result_fields()[0].name().as_str(), "label");
    assert_eq!(
        typing.result_fields()[0].expression_type().query_type(),
        &QueryType::Schema(ValueSchema::String)
    );
    assert_eq!(typing.result_fields()[1].name().as_str(), "count");
    assert_eq!(
        typing.result_fields()[1].expression_type().query_type(),
        &QueryType::Schema(ValueSchema::Integer)
    );
    assert!(!typing.result_fields()[1].expression_type().nullable());
}

#[test]
fn query_binding_rejects_ungrouped_projection_with_aggregation() {
    let relation = id("typing-group-relation", |bytes| {
        RelationId::from_canonical_bytes(bytes).unwrap()
    });
    let entity = id("typing-group-entity", |bytes| {
        EntityId::from_canonical_bytes(bytes).unwrap()
    });
    let query_id = id("typing-group-query", |bytes| {
        QueryId::from_canonical_bytes(bytes).unwrap()
    });
    let generation = id("typing-group-generation", |bytes| {
        GenerationId::from_canonical_bytes(bytes).unwrap()
    });
    let binding = |name| Binding::new(name).unwrap();
    let property = Expression::Property {
        binding: binding("source"),
        key: PropertyKey::new("name").unwrap(),
    };
    let query = typed_query(
        query_id,
        relation,
        entity,
        Expression::Literal(Value::Boolean(true)),
        TypedResultSpec {
            projections: vec![Projection::new(
                id("typing-group-projection", |bytes| {
                    QueryOperatorId::from_canonical_bytes(bytes).unwrap()
                }),
                property,
                binding("label"),
            )],
            aggregations: vec![Aggregation::new(
                id("typing-group-count", |bytes| {
                    QueryOperatorId::from_canonical_bytes(bytes).unwrap()
                }),
                AggregationFunction::Count,
                None,
                Vec::new(),
                true,
                binding("count"),
            )],
            group_aggregates: false,
            limit: None,
        },
    );
    let bundle = ReasoningBundle::admit(ReasoningBundleDeclaration {
        entities: vec![entity_schema(entity, ValueSchema::String)],
        relations: vec![relation_schema(relation)],
        query_templates: vec![QueryTemplate::new(query, Vec::new())],
        ..ReasoningBundleDeclaration::default()
    })
    .unwrap();
    assert_eq!(
        MrrEngine::builder()
            .with_bundle(bundle)
            .build()
            .unwrap()
            .bind_query(query_id, &snapshot(generation, "typing-group"))
            .unwrap_err(),
        QueryCatalogBindingError::UngroupedProjection(binding("label"))
    );
}

#[test]
fn query_binding_rejects_non_boolean_filters_and_unconstrained_parameters() {
    let relation = id("typing-negative-relation", |bytes| {
        RelationId::from_canonical_bytes(bytes).unwrap()
    });
    let entity = id("typing-negative-entity", |bytes| {
        EntityId::from_canonical_bytes(bytes).unwrap()
    });
    let generation = id("typing-negative-generation", |bytes| {
        GenerationId::from_canonical_bytes(bytes).unwrap()
    });
    let binding = |name| Binding::new(name).unwrap();
    let assert_error = |label: &str, filter: Expression, projection: Expression| {
        let query_id = id(label, |bytes| QueryId::from_canonical_bytes(bytes).unwrap());
        let query = typed_query(
            query_id,
            relation,
            entity,
            filter,
            TypedResultSpec {
                projections: vec![Projection::new(
                    id(label, |bytes| {
                        QueryOperatorId::from_canonical_bytes(bytes).unwrap()
                    }),
                    projection,
                    binding("label"),
                )],
                aggregations: Vec::new(),
                group_aggregates: false,
                limit: None,
            },
        );
        let bundle = ReasoningBundle::admit(ReasoningBundleDeclaration {
            entities: vec![entity_schema(entity, ValueSchema::String)],
            relations: vec![relation_schema(relation)],
            query_templates: vec![QueryTemplate::new(query, Vec::new())],
            ..ReasoningBundleDeclaration::default()
        })
        .unwrap();
        MrrEngine::builder()
            .with_bundle(bundle)
            .build()
            .unwrap()
            .bind_query(query_id, &snapshot(generation, label))
            .unwrap_err()
    };

    let property = Expression::Property {
        binding: binding("source"),
        key: PropertyKey::new("name").unwrap(),
    };
    assert!(matches!(
        assert_error(
            "non-boolean-filter",
            property,
            Expression::Binding(binding("source"))
        ),
        QueryCatalogBindingError::IncompatibleOperands { .. }
    ));
    let free = Parameter::new("free").unwrap();
    assert_eq!(
        assert_error(
            "unconstrained-parameter",
            Expression::Literal(Value::Boolean(true)),
            Expression::Parameter(free.clone())
        ),
        QueryCatalogBindingError::UnconstrainedParameter(free)
    );
}

#[test]
fn query_binding_rejects_conflicting_parameter_and_operator_types() {
    let relation = id("typing-conflict-relation", |bytes| {
        RelationId::from_canonical_bytes(bytes).unwrap()
    });
    let entity = id("typing-conflict-entity", |bytes| {
        EntityId::from_canonical_bytes(bytes).unwrap()
    });
    let generation = id("typing-conflict-generation", |bytes| {
        GenerationId::from_canonical_bytes(bytes).unwrap()
    });
    let binding = |name| Binding::new(name).unwrap();
    let query_id = id("typing-conflict-query", |bytes| {
        QueryId::from_canonical_bytes(bytes).unwrap()
    });
    let parameter = Parameter::new("shared").unwrap();
    let property = |key| Expression::Property {
        binding: binding("source"),
        key: PropertyKey::new(key).unwrap(),
    };
    let filter = Expression::Binary {
        left: Box::new(Expression::Binary {
            left: Box::new(property("name")),
            operator: BinaryOperator::Equal,
            right: Box::new(Expression::Parameter(parameter.clone())),
        }),
        operator: BinaryOperator::And,
        right: Box::new(Expression::Binary {
            left: Box::new(property("score")),
            operator: BinaryOperator::Equal,
            right: Box::new(Expression::Parameter(parameter.clone())),
        }),
    };
    let query = typed_query(
        query_id,
        relation,
        entity,
        filter,
        TypedResultSpec {
            projections: vec![Projection::new(
                id("typing-conflict-projection", |bytes| {
                    QueryOperatorId::from_canonical_bytes(bytes).unwrap()
                }),
                Expression::Binary {
                    left: Box::new(property("name")),
                    operator: BinaryOperator::Add,
                    right: Box::new(Expression::Literal(Value::Integer(1))),
                },
                binding("label"),
            )],
            aggregations: Vec::new(),
            group_aggregates: false,
            limit: None,
        },
    );
    let bundle = ReasoningBundle::admit(ReasoningBundleDeclaration {
        entities: vec![
            EntitySchema::new(
                entity,
                "artifact",
                vec![
                    RelationField::new("name", ValueSchema::String, false).unwrap(),
                    RelationField::new("score", ValueSchema::Integer, false).unwrap(),
                ],
            )
            .unwrap(),
        ],
        relations: vec![relation_schema(relation)],
        query_templates: vec![QueryTemplate::new(query, Vec::new())],
        ..ReasoningBundleDeclaration::default()
    })
    .unwrap();
    assert_eq!(
        MrrEngine::builder()
            .with_bundle(bundle)
            .build()
            .unwrap()
            .bind_query(query_id, &snapshot(generation, "typing-conflict"))
            .unwrap_err(),
        QueryCatalogBindingError::ConflictingParameterType(parameter)
    );
}

#[test]
fn query_binding_rejects_invalid_arithmetic_types() {
    let relation = id("typing-arithmetic-relation", |bytes| {
        RelationId::from_canonical_bytes(bytes).unwrap()
    });
    let entity = id("typing-arithmetic-entity", |bytes| {
        EntityId::from_canonical_bytes(bytes).unwrap()
    });
    let query_id = id("typing-arithmetic-query", |bytes| {
        QueryId::from_canonical_bytes(bytes).unwrap()
    });
    let generation = id("typing-arithmetic-generation", |bytes| {
        GenerationId::from_canonical_bytes(bytes).unwrap()
    });
    let binding = |name| Binding::new(name).unwrap();
    let query = typed_query(
        query_id,
        relation,
        entity,
        Expression::Literal(Value::Boolean(true)),
        TypedResultSpec {
            projections: vec![Projection::new(
                id("typing-arithmetic-projection", |bytes| {
                    QueryOperatorId::from_canonical_bytes(bytes).unwrap()
                }),
                Expression::Binary {
                    left: Box::new(Expression::Property {
                        binding: binding("source"),
                        key: PropertyKey::new("name").unwrap(),
                    }),
                    operator: BinaryOperator::Add,
                    right: Box::new(Expression::Literal(Value::Integer(1))),
                },
                binding("label"),
            )],
            aggregations: Vec::new(),
            group_aggregates: false,
            limit: None,
        },
    );
    let bundle = ReasoningBundle::admit(ReasoningBundleDeclaration {
        entities: vec![entity_schema(entity, ValueSchema::String)],
        relations: vec![relation_schema(relation)],
        query_templates: vec![QueryTemplate::new(query, Vec::new())],
        ..ReasoningBundleDeclaration::default()
    })
    .unwrap();
    assert!(matches!(
        MrrEngine::builder()
            .with_bundle(bundle)
            .build()
            .unwrap()
            .bind_query(query_id, &snapshot(generation, "typing-arithmetic"))
            .unwrap_err(),
        QueryCatalogBindingError::TypeMismatch {
            context: "arithmetic operand",
            ..
        }
    ));
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
