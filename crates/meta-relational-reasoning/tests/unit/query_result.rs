use core::num::NonZeroUsize;

use crate::{
    Aggregation, AggregationFunction, Binding, CandidateQueryResult, CatalogBoundQuery, Direction,
    EntityId, EntitySchema, Expression, ExternalRevisionIdentity, GenerationId, GraphPattern,
    MrrEngine, NodePattern, PageValue, PathPattern, PathSegment, Projection, PropertyKey, QueryId,
    QueryOperatorId, QueryResult, QueryResultAdmissionError, QueryResultBinding, QueryResultLimits,
    QueryResultTransportError, QueryResultValue, QueryTemplate, ReasoningBundle,
    ReasoningBundleDeclaration, RelationField, RelationId, RelationPattern, RelationSchema,
    RevisionBinding, SemanticSnapshot, SetQuantifier, Value, ValueSchema,
    export_query_result_transport, verify_query_result_transport,
};

enum TestResult {
    Projection {
        quantifier: SetQuantifier,
        limit: Option<PageValue>,
    },
    CollectList,
    Finish,
}

fn id<T>(label: &str, derive: impl FnOnce(&[u8]) -> T) -> T {
    derive(format!("query-result:{label}").as_bytes())
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

fn snapshot(generation: GenerationId, revision: &str) -> SemanticSnapshot {
    SemanticSnapshot::admit(
        generation,
        vec![
            RevisionBinding::admit(
                ExternalRevisionIdentity::new("git", "repository", revision).unwrap(),
                generation,
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

fn property_query(
    query_id: QueryId,
    relation: RelationId,
    entity_type: EntityId,
    result_kind: TestResult,
) -> crate::MetaQueryIr {
    let binding = |name| Binding::new(name).unwrap();
    let property = Expression::Property {
        binding: binding("source"),
        key: PropertyKey::new("name").unwrap(),
    };
    let result = match result_kind {
        TestResult::CollectList => {
            QueryResult::returning(SetQuantifier::All).with_aggregations(vec![Aggregation::new(
                id("collect", |bytes| {
                    QueryOperatorId::from_canonical_bytes(bytes).unwrap()
                }),
                AggregationFunction::CollectList,
                None,
                vec![property],
                false,
                binding("names"),
            )])
        }
        TestResult::Projection { quantifier, limit } => QueryResult::returning(quantifier)
            .with_projections(vec![Projection::new(
                id("projection", |bytes| {
                    QueryOperatorId::from_canonical_bytes(bytes).unwrap()
                }),
                property,
                binding("name"),
            )])
            .with_limit(limit),
        TestResult::Finish => QueryResult::finish(),
    };
    crate::MetaQueryIr::new(
        query_id,
        GraphPattern::new(
            id("graph", |bytes| {
                QueryOperatorId::from_canonical_bytes(bytes).unwrap()
            }),
            vec![PathPattern::new(
                NodePattern::new(binding("source"), vec![entity_type]),
                vec![PathSegment::new(
                    RelationPattern::new(
                        Some(binding("edge")),
                        vec![relation],
                        Direction::Outgoing,
                        1,
                        Some(1),
                    )
                    .unwrap(),
                    NodePattern::new(binding("target"), vec![entity_type]),
                )],
            )],
        )
        .unwrap(),
        Vec::new(),
        result,
    )
    .unwrap()
}

fn graph_value_query(
    query_id: QueryId,
    relation: RelationId,
    entity_type: EntityId,
) -> crate::MetaQueryIr {
    let binding = |name| Binding::new(name).unwrap();
    crate::MetaQueryIr::new(
        query_id,
        GraphPattern::new(
            id("graph-values", |bytes| {
                QueryOperatorId::from_canonical_bytes(bytes).unwrap()
            }),
            vec![PathPattern::new(
                NodePattern::new(binding("source"), vec![entity_type]),
                vec![PathSegment::new(
                    RelationPattern::new(
                        Some(binding("edge")),
                        vec![relation],
                        Direction::Outgoing,
                        1,
                        Some(1),
                    )
                    .unwrap(),
                    NodePattern::new(binding("target"), vec![entity_type]),
                )],
            )],
        )
        .unwrap(),
        Vec::new(),
        QueryResult::returning(SetQuantifier::All).with_projections(vec![
            Projection::new(
                id("node-projection", |bytes| {
                    QueryOperatorId::from_canonical_bytes(bytes).unwrap()
                }),
                Expression::Binding(binding("source")),
                binding("node"),
            ),
            Projection::new(
                id("edge-projection", |bytes| {
                    QueryOperatorId::from_canonical_bytes(bytes).unwrap()
                }),
                Expression::Binding(binding("edge")),
                binding("relation"),
            ),
        ]),
    )
    .unwrap()
}

fn bound_query(
    label: &str,
    schema: ValueSchema,
    nullable: bool,
    aggregate: bool,
) -> CatalogBoundQuery {
    let result = if aggregate {
        TestResult::CollectList
    } else {
        TestResult::Projection {
            quantifier: SetQuantifier::All,
            limit: None,
        }
    };
    bound_query_with_result(label, schema, nullable, result)
}

fn bound_query_with_result(
    label: &str,
    schema: ValueSchema,
    nullable: bool,
    result: TestResult,
) -> CatalogBoundQuery {
    let relation = id(&format!("{label}:relation"), |bytes| {
        RelationId::from_canonical_bytes(bytes).unwrap()
    });
    let entity = id(&format!("{label}:entity"), |bytes| {
        EntityId::from_canonical_bytes(bytes).unwrap()
    });
    let query_id = id(&format!("{label}:query"), |bytes| {
        QueryId::from_canonical_bytes(bytes).unwrap()
    });
    let generation = id(&format!("{label}:generation"), |bytes| {
        GenerationId::from_canonical_bytes(bytes).unwrap()
    });
    let bundle = ReasoningBundle::admit(ReasoningBundleDeclaration {
        relations: vec![relation_schema(relation)],
        entities: vec![
            EntitySchema::new(
                entity,
                "artifact",
                vec![RelationField::new("name", schema, nullable).unwrap()],
            )
            .unwrap(),
        ],
        query_templates: vec![QueryTemplate::new(
            property_query(query_id, relation, entity, result),
            Vec::new(),
        )],
        ..ReasoningBundleDeclaration::default()
    })
    .unwrap();
    MrrEngine::builder()
        .with_bundle(bundle)
        .build()
        .unwrap()
        .bind_query(query_id, &snapshot(generation, label))
        .unwrap()
}

fn bound_graph_query(label: &str) -> (CatalogBoundQuery, EntityId, RelationId) {
    let relation = id(&format!("{label}:relation"), |bytes| {
        RelationId::from_canonical_bytes(bytes).unwrap()
    });
    let entity = id(&format!("{label}:entity"), |bytes| {
        EntityId::from_canonical_bytes(bytes).unwrap()
    });
    let query_id = id(&format!("{label}:query"), |bytes| {
        QueryId::from_canonical_bytes(bytes).unwrap()
    });
    let generation = id(&format!("{label}:generation"), |bytes| {
        GenerationId::from_canonical_bytes(bytes).unwrap()
    });
    let bundle = ReasoningBundle::admit(ReasoningBundleDeclaration {
        relations: vec![relation_schema(relation)],
        entities: vec![EntitySchema::new(entity, "artifact", Vec::new()).unwrap()],
        query_templates: vec![QueryTemplate::new(
            graph_value_query(query_id, relation, entity),
            Vec::new(),
        )],
        ..ReasoningBundleDeclaration::default()
    })
    .unwrap();
    let bound = MrrEngine::builder()
        .with_bundle(bundle)
        .build()
        .unwrap()
        .bind_query(query_id, &snapshot(generation, label))
        .unwrap();
    (bound, entity, relation)
}

fn candidate(
    bound: &CatalogBoundQuery,
    columns: Vec<Binding>,
    rows: Vec<Vec<QueryResultValue>>,
) -> CandidateQueryResult {
    CandidateQueryResult::new(QueryResultBinding::for_query(bound), columns, rows)
}

fn limits(rows: usize, cells: usize) -> QueryResultLimits {
    QueryResultLimits::new(
        NonZeroUsize::new(rows).unwrap(),
        NonZeroUsize::new(cells).unwrap(),
    )
}

#[test]
fn query_result_admission_is_exact_bounded_and_deterministic() {
    let bound = bound_query("exact", ValueSchema::String, false, false);
    let result = candidate(
        &bound,
        vec![Binding::new("name").unwrap()],
        vec![vec![QueryResultValue::scalar(
            ValueSchema::String,
            Value::String("compiler".into()),
        )]],
    );
    let first = crate::admit_query_result_candidate(&bound, &result, limits(10, 10)).unwrap();
    let second = crate::admit_query_result_candidate(&bound, &result, limits(10, 10)).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.binding(), *result.binding());
    assert_eq!(first.row_count(), result.rows().len());
    assert_eq!(result.rows().len(), 1);
    assert_eq!(result.columns(), &[Binding::new("name").unwrap()]);
    assert_eq!(first.binding().query_binding_digest(), bound.digest());
    assert_eq!(first.binding().generation(), bound.generation());
    assert_eq!(
        first.binding().relation_catalog_digest(),
        bound.catalog_digest()
    );
    assert_eq!(
        first.binding().entity_catalog_digest(),
        bound.entity_catalog_digest()
    );
    assert_eq!(first.binding().snapshot_digest(), bound.snapshot_digest());
}

#[test]
fn transported_result_rechecks_exact_binding_rows_and_digest() {
    let bound = bound_query("transport", ValueSchema::String, false, false);
    let result = candidate(
        &bound,
        vec![Binding::new("name").unwrap()],
        vec![vec![QueryResultValue::scalar(
            ValueSchema::String,
            Value::String("compiler".into()),
        )]],
    );
    let max_bytes = NonZeroUsize::new(16_384).unwrap();
    let bytes = export_query_result_transport(&bound, &result, limits(10, 10), max_bytes).unwrap();
    let verified =
        verify_query_result_transport(&bound, &bytes, limits(10, 10), max_bytes).unwrap();
    assert_eq!(verified.candidate(), &result);
    assert_eq!(
        *verified.receipt(),
        crate::admit_query_result_candidate(&bound, &result, limits(10, 10)).unwrap()
    );

    let altered_rows = String::from_utf8(bytes.clone())
        .unwrap()
        .replace("compiler", "analyzer");
    assert_eq!(
        verify_query_result_transport(&bound, altered_rows.as_bytes(), limits(10, 10), max_bytes,),
        Err(QueryResultTransportError::DigestMismatch)
    );
    let other = bound_query("other-transport", ValueSchema::String, false, false);
    assert_eq!(
        verify_query_result_transport(&other, &bytes, limits(10, 10), max_bytes),
        Err(QueryResultTransportError::BindingMismatch)
    );
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(text.starts_with("(object "));
    let altered_count = text.replace("(\"row_count\" 1)", "(\"row_count\" 2)");
    assert_eq!(
        verify_query_result_transport(&bound, altered_count.as_bytes(), limits(10, 10), max_bytes),
        Err(QueryResultTransportError::RowCountMismatch)
    );
    let altered_schema = text.replace(
        "mrr.query-result-transport.v2",
        "mrr.query-result-transport.v1",
    );
    assert_eq!(
        verify_query_result_transport(&bound, altered_schema.as_bytes(), limits(10, 10), max_bytes),
        Err(QueryResultTransportError::SchemaMismatch)
    );
    for malformed in [
        "{}".to_owned(),
        "#.(exit)".to_owned(),
        "#0=(#0#)".to_owned(),
        format!("{text} null"),
        text.replacen("(object", "(object (\"row_count\" 1)", 1),
        text.replacen("(object", "(object (\"unexpected\" #t)", 1),
        "(list ".repeat(65) + &")".repeat(65),
    ] {
        assert!(
            matches!(
                verify_query_result_transport(
                    &bound,
                    malformed.as_bytes(),
                    limits(10, 10),
                    max_bytes
                ),
                Err(QueryResultTransportError::Encoding(_))
            ),
            "accepted {malformed}"
        );
    }
    assert!(matches!(
        verify_query_result_transport(
            &bound,
            &bytes,
            limits(10, 10),
            NonZeroUsize::new(bytes.len() - 1).unwrap(),
        ),
        Err(QueryResultTransportError::TooLarge { .. })
    ));
    assert!(matches!(
        export_query_result_transport(
            &bound,
            &result,
            limits(10, 10),
            NonZeroUsize::new(bytes.len() - 1).unwrap(),
        ),
        Err(QueryResultTransportError::TooLarge { .. })
    ));
    assert!(matches!(
        verify_query_result_transport(&bound, &bytes[..bytes.len() - 1], limits(10, 10), max_bytes,),
        Err(QueryResultTransportError::Encoding(_))
    ));
}

#[test]
fn query_result_admission_rejects_every_identity_mismatch() {
    let bound = bound_query("identity", ValueSchema::String, false, false);
    let other = bound_query("other", ValueSchema::String, false, false);
    let columns = vec![Binding::new("name").unwrap()];
    let rows = vec![vec![QueryResultValue::scalar(
        ValueSchema::String,
        Value::String("compiler".into()),
    )]];
    let make = |digest, generation, relation_catalog, entity_catalog, snapshot_digest| {
        CandidateQueryResult::new(
            QueryResultBinding::new(
                digest,
                generation,
                relation_catalog,
                entity_catalog,
                snapshot_digest,
            ),
            columns.clone(),
            rows.clone(),
        )
    };
    assert_eq!(
        crate::admit_query_result_candidate(
            &bound,
            &make(
                [9; 32],
                bound.generation(),
                bound.catalog_digest(),
                bound.entity_catalog_digest(),
                *bound.snapshot_digest(),
            ),
            limits(10, 10),
        ),
        Err(QueryResultAdmissionError::QueryBindingMismatch)
    );
    assert_eq!(
        crate::admit_query_result_candidate(
            &bound,
            &make(
                *bound.digest(),
                other.generation(),
                bound.catalog_digest(),
                bound.entity_catalog_digest(),
                *bound.snapshot_digest(),
            ),
            limits(10, 10),
        ),
        Err(QueryResultAdmissionError::GenerationMismatch)
    );
    assert_eq!(
        crate::admit_query_result_candidate(
            &bound,
            &make(
                *bound.digest(),
                bound.generation(),
                other.catalog_digest(),
                bound.entity_catalog_digest(),
                *bound.snapshot_digest(),
            ),
            limits(10, 10),
        ),
        Err(QueryResultAdmissionError::RelationCatalogMismatch)
    );
    assert_eq!(
        crate::admit_query_result_candidate(
            &bound,
            &make(
                *bound.digest(),
                bound.generation(),
                bound.catalog_digest(),
                other.entity_catalog_digest(),
                *bound.snapshot_digest(),
            ),
            limits(10, 10),
        ),
        Err(QueryResultAdmissionError::EntityCatalogMismatch)
    );
    assert_eq!(
        crate::admit_query_result_candidate(
            &bound,
            &make(
                *bound.digest(),
                bound.generation(),
                bound.catalog_digest(),
                bound.entity_catalog_digest(),
                [7; 32],
            ),
            limits(10, 10),
        ),
        Err(QueryResultAdmissionError::SnapshotMismatch)
    );
}

#[test]
fn query_result_admission_rejects_shape_type_value_and_budget_failures() {
    let bound = bound_query("shape", ValueSchema::String, false, false);
    let valid = || {
        vec![vec![QueryResultValue::scalar(
            ValueSchema::String,
            Value::String("compiler".into()),
        )]]
    };
    assert_eq!(
        crate::admit_query_result_candidate(
            &bound,
            &candidate(&bound, Vec::new(), valid()),
            limits(10, 10),
        ),
        Err(QueryResultAdmissionError::ColumnCountMismatch {
            expected: 1,
            actual: 0,
        })
    );
    assert!(matches!(
        crate::admit_query_result_candidate(
            &bound,
            &candidate(&bound, vec![Binding::new("wrong").unwrap()], valid()),
            limits(10, 10),
        ),
        Err(QueryResultAdmissionError::ColumnNameMismatch { column: 0, .. })
    ));
    assert_eq!(
        crate::admit_query_result_candidate(
            &bound,
            &candidate(
                &bound,
                vec![Binding::new("name").unwrap()],
                vec![Vec::new()]
            ),
            limits(10, 10),
        ),
        Err(QueryResultAdmissionError::RowWidthMismatch {
            row: 0,
            expected: 1,
            actual: 0,
        })
    );
    assert_eq!(
        crate::admit_query_result_candidate(
            &bound,
            &candidate(
                &bound,
                vec![Binding::new("name").unwrap()],
                vec![vec![QueryResultValue::null()]],
            ),
            limits(10, 10),
        ),
        Err(QueryResultAdmissionError::NullNotAllowed { row: 0, column: 0 })
    );
    assert!(matches!(
        crate::admit_query_result_candidate(
            &bound,
            &candidate(
                &bound,
                vec![Binding::new("name").unwrap()],
                vec![vec![QueryResultValue::scalar(
                    ValueSchema::Integer,
                    Value::Integer(1),
                )]],
            ),
            limits(10, 10),
        ),
        Err(QueryResultAdmissionError::ScalarSchemaMismatch {
            row: 0,
            column: 0,
            ..
        })
    ));
    assert_eq!(
        crate::admit_query_result_candidate(
            &bound,
            &candidate(
                &bound,
                vec![Binding::new("name").unwrap()],
                vec![vec![QueryResultValue::scalar(
                    ValueSchema::String,
                    Value::Integer(1),
                )]],
            ),
            limits(10, 10),
        ),
        Err(QueryResultAdmissionError::InvalidScalar {
            row: 0,
            column: 0,
            error: crate::RelationError::TypeMismatch {
                field: "value".into(),
                expected: ValueSchema::String,
                actual: crate::ValueKind::Integer,
            },
        })
    );
    assert_eq!(
        crate::admit_query_result_candidate(
            &bound,
            &candidate(
                &bound,
                vec![Binding::new("name").unwrap()],
                vec![valid()[0].clone(), valid()[0].clone()],
            ),
            limits(1, 10),
        ),
        Err(QueryResultAdmissionError::RowLimitExceeded {
            limit: 1,
            actual: 2,
        })
    );
    assert_eq!(
        crate::admit_query_result_candidate(
            &bound,
            &candidate(
                &bound,
                vec![Binding::new("name").unwrap()],
                vec![valid()[0].clone(), valid()[0].clone()],
            ),
            limits(10, 1),
        ),
        Err(QueryResultAdmissionError::CellLimitExceeded {
            limit: 1,
            actual: 2,
        })
    );
    assert_eq!(
        crate::admit_query_result_candidate(
            &bound,
            &candidate(&bound, vec![Binding::new("name").unwrap()], valid()),
            limits(10, 1),
        )
        .unwrap()
        .row_count(),
        1
    );
}

#[test]
fn query_result_admission_preserves_list_element_nullability() {
    let bound = bound_query("list", ValueSchema::String, true, true);
    let result = candidate(
        &bound,
        vec![Binding::new("names").unwrap()],
        vec![vec![QueryResultValue::list(vec![
            QueryResultValue::scalar(ValueSchema::String, Value::String("compiler".into())),
            QueryResultValue::null(),
        ])]],
    );
    crate::admit_query_result_candidate(&bound, &result, limits(10, 10)).unwrap();
}

#[test]
fn query_result_admission_enforces_finish_distinct_and_literal_limit() {
    let finish = bound_query_with_result("finish", ValueSchema::String, false, TestResult::Finish);
    assert_eq!(
        crate::admit_query_result_candidate(
            &finish,
            &candidate(&finish, Vec::new(), vec![Vec::new()]),
            limits(10, 10),
        ),
        Err(QueryResultAdmissionError::UnexpectedRowsForFinish { actual: 1 })
    );

    let distinct = bound_query_with_result(
        "distinct",
        ValueSchema::String,
        false,
        TestResult::Projection {
            quantifier: SetQuantifier::Distinct,
            limit: None,
        },
    );
    let value = vec![QueryResultValue::scalar(
        ValueSchema::String,
        Value::String("compiler".into()),
    )];
    assert_eq!(
        crate::admit_query_result_candidate(
            &distinct,
            &candidate(
                &distinct,
                vec![Binding::new("name").unwrap()],
                vec![value.clone(), value],
            ),
            limits(10, 10),
        ),
        Err(QueryResultAdmissionError::DuplicateDistinctRow { row: 1 })
    );

    let limited = bound_query_with_result(
        "limited",
        ValueSchema::String,
        false,
        TestResult::Projection {
            quantifier: SetQuantifier::All,
            limit: Some(PageValue::Literal(1)),
        },
    );
    let value = || {
        vec![QueryResultValue::scalar(
            ValueSchema::String,
            Value::String("compiler".into()),
        )]
    };
    assert_eq!(
        crate::admit_query_result_candidate(
            &limited,
            &candidate(
                &limited,
                vec![Binding::new("name").unwrap()],
                vec![value(), value()],
            ),
            limits(10, 10),
        ),
        Err(QueryResultAdmissionError::ResultLimitExceeded {
            limit: 1,
            actual: 2,
        })
    );
}

#[test]
fn query_result_admission_checks_graph_value_types() {
    let (bound, entity_type, relation_type) = bound_graph_query("graph");
    let node_id = id("node-instance", |bytes| {
        EntityId::from_canonical_bytes(bytes).unwrap()
    });
    let fact_id = id("relation-instance", |bytes| {
        crate::FactId::from_canonical_bytes(bytes).unwrap()
    });
    let columns = vec![
        Binding::new("node").unwrap(),
        Binding::new("relation").unwrap(),
    ];
    let row = vec![
        QueryResultValue::node(node_id, entity_type),
        QueryResultValue::relation(fact_id, relation_type),
    ];
    crate::admit_query_result_candidate(
        &bound,
        &candidate(&bound, columns.clone(), vec![row]),
        limits(10, 20),
    )
    .unwrap();

    let wrong_type = id("wrong-type", |bytes| {
        EntityId::from_canonical_bytes(bytes).unwrap()
    });
    assert!(matches!(
        crate::admit_query_result_candidate(
            &bound,
            &candidate(
                &bound,
                columns,
                vec![vec![
                    QueryResultValue::node(node_id, wrong_type),
                    QueryResultValue::relation(fact_id, relation_type),
                ]],
            ),
            limits(10, 20),
        ),
        Err(QueryResultAdmissionError::ValueTypeMismatch {
            row: 0,
            column: 0,
            ..
        })
    ));
}

#[test]
fn scheme_transport_preserves_native_scalar_schemas_and_values() {
    use crate::{FloatWidth, TemporalUnit, TimezonePolicy};
    let scalar_cases = vec![
        (
            ValueSchema::Entity,
            Value::Entity(EntityId::from_canonical_bytes(b"entity-wire").unwrap()),
        ),
        (ValueSchema::Boolean, Value::Boolean(true)),
        (ValueSchema::Integer, Value::Integer(i64::MIN)),
        (ValueSchema::Integer, Value::Integer(i64::MAX)),
        (
            ValueSchema::Decimal {
                precision: 8,
                scale: 2,
            },
            Value::Decimal("123.45".into()),
        ),
        (
            ValueSchema::Float {
                width: FloatWidth::Binary64,
            },
            Value::Float("1.25".into()),
        ),
        (
            ValueSchema::String,
            Value::String("中文 λ \" \\ \n\r\t".into()),
        ),
        (
            ValueSchema::ByteString,
            Value::ByteString(vec![0, 1, 127, 255]),
        ),
        (ValueSchema::Date, Value::Date("2026-10-04".into())),
        (
            ValueSchema::Time {
                unit: TemporalUnit::Second,
                timezone: TimezonePolicy::Naive,
            },
            Value::Time("12:00:00".into()),
        ),
        (
            ValueSchema::Timestamp {
                unit: TemporalUnit::Second,
                timezone: TimezonePolicy::Utc,
            },
            Value::Timestamp("2026-10-04T12:00:00Z".into()),
        ),
        (ValueSchema::Duration, Value::Duration("PT1S".into())),
        (
            ValueSchema::List {
                element: Box::new(ValueSchema::Integer),
                element_nullable: true,
            },
            Value::List(vec![Value::Integer(3), Value::Null]),
        ),
        (
            ValueSchema::Record {
                fields: vec![RelationField::new("message", ValueSchema::String, false).unwrap()],
            },
            Value::Record(vec![("message".into(), Value::String("event".into()))]),
        ),
    ];
    for (schema, value) in scalar_cases {
        let bound = bound_query("wire-scalars", schema.clone(), false, false);
        let original = candidate(
            &bound,
            vec![Binding::new("name").unwrap()],
            vec![vec![QueryResultValue::scalar(schema, value)]],
        );
        let bytes = export_query_result_transport(
            &bound,
            &original,
            limits(10, 10),
            NonZeroUsize::new(16_384).unwrap(),
        )
        .unwrap();
        let exact = NonZeroUsize::new(bytes.len()).unwrap();
        assert_eq!(
            export_query_result_transport(&bound, &original, limits(10, 10), exact).unwrap(),
            bytes
        );
        assert_eq!(
            verify_query_result_transport(&bound, &bytes, limits(10, 10), exact)
                .unwrap()
                .candidate(),
            &original
        );
    }
}

#[test]
fn scheme_transport_bounds_nested_values_before_recursive_admission() {
    let bound = bound_query("wire-depth", ValueSchema::Integer, false, false);
    let mut nested = QueryResultValue::Null;
    for _ in 0..80 {
        nested = QueryResultValue::List(vec![nested]);
    }
    let original = candidate(
        &bound,
        vec![Binding::new("name").unwrap()],
        vec![vec![nested]],
    );
    assert!(matches!(
        export_query_result_transport(
            &bound,
            &original,
            limits(10, 10),
            NonZeroUsize::new(16_384).unwrap()
        ),
        Err(QueryResultTransportError::Encoding(_))
    ));
}

#[test]
fn scheme_transport_preserves_empty_null_lists_and_graph_results() {
    let bound = bound_query("wire-null", ValueSchema::String, true, false);
    let list_bound = bound_query("wire-list", ValueSchema::String, true, true);
    let (graph_bound, entity_type, relation_type) = bound_graph_query("wire-graph");
    let cases = vec![
        (
            &bound,
            candidate(&bound, vec![Binding::new("name").unwrap()], vec![]),
        ),
        (
            &bound,
            candidate(
                &bound,
                vec![Binding::new("name").unwrap()],
                vec![vec![QueryResultValue::Null], vec![QueryResultValue::Null]],
            ),
        ),
        (
            &list_bound,
            candidate(
                &list_bound,
                vec![Binding::new("names").unwrap()],
                vec![vec![QueryResultValue::List(vec![
                    QueryResultValue::Null,
                    QueryResultValue::scalar(ValueSchema::String, Value::String("event".into())),
                ])]],
            ),
        ),
        (
            &graph_bound,
            candidate(
                &graph_bound,
                vec![
                    Binding::new("node").unwrap(),
                    Binding::new("relation").unwrap(),
                ],
                vec![vec![
                    QueryResultValue::node(
                        EntityId::from_canonical_bytes(b"wire-node").unwrap(),
                        entity_type,
                    ),
                    QueryResultValue::relation(
                        crate::FactId::from_canonical_bytes(b"wire-fact").unwrap(),
                        relation_type,
                    ),
                ]],
            ),
        ),
    ];
    for (query, result) in cases {
        let cap = NonZeroUsize::new(16_384).unwrap();
        let bytes = export_query_result_transport(query, &result, limits(10, 20), cap).unwrap();
        assert_eq!(
            verify_query_result_transport(query, &bytes, limits(10, 20), cap)
                .unwrap()
                .candidate(),
            &result
        );
    }
}
