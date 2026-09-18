use crate::{
    DerivationId, EntityId, EntitySchema, EvidenceCompleteness, Fact, FactId, FactProvenance,
    FactValidity, FloatWidth, GenerationId, RelationAuthority, RelationConstraint, RelationContext,
    RelationContextError, RelationError, RelationField, RelationId, RelationSchema, RuleId,
    TemporalUnit, TimezonePolicy, Value, ValueKind, ValueSchema,
};

fn id<T>(label: &str, derive: impl FnOnce(&[u8]) -> T) -> T {
    derive(format!("mrr-relation-test:{label}").as_bytes())
}

#[test]
fn entity_schema_reuses_recursive_value_contracts_without_requiring_properties() {
    let entity = id("entity-type", |bytes| {
        EntityId::from_canonical_bytes(bytes).expect("entity")
    });
    assert!(EntitySchema::new(entity, "marker", Vec::new()).is_ok());
    let duplicate = vec![
        RelationField::new("name", ValueSchema::String, false).unwrap(),
        RelationField::new("name", ValueSchema::Integer, false).unwrap(),
    ];
    assert_eq!(
        EntitySchema::new(entity, "artifact", duplicate),
        Err(RelationError::DuplicateFieldName("name".into()))
    );
    assert_eq!(
        EntitySchema::new(entity, " ", Vec::new()),
        Err(RelationError::EmptyEntityName)
    );
}

fn source_context(domain: &str) -> RelationContext {
    let authority = id(domain, |bytes| {
        EntityId::from_canonical_bytes(bytes).expect("authority")
    });
    RelationContext::new(
        id("generation", |bytes| {
            GenerationId::from_canonical_bytes(bytes).expect("generation")
        }),
        RelationAuthority::Entity(authority),
        FactProvenance::Source(authority),
        EvidenceCompleteness::Complete,
        FactValidity::Valid,
    )
    .expect("source context")
}

fn binary_schema(predicate: &str) -> RelationSchema {
    RelationSchema::new(
        id(predicate, |bytes| {
            RelationId::from_canonical_bytes(bytes).expect("relation")
        }),
        predicate,
        vec![
            RelationField::new("subject", ValueSchema::Entity, false).expect("subject field"),
            RelationField::new("object", ValueSchema::Entity, false).expect("object field"),
        ],
        Vec::new(),
    )
    .expect("binary schema")
}

#[test]
fn schema_is_the_single_fact_shape_and_type_authority() {
    let schema = binary_schema("calls");
    let subject = id("subject", |bytes| {
        EntityId::from_canonical_bytes(bytes).expect("subject")
    });
    let fact = Fact::new(
        id("fact", |bytes| {
            FactId::from_canonical_bytes(bytes).expect("fact")
        }),
        schema.id(),
        vec![
            Value::Entity(subject),
            Value::String("not-an-entity".into()),
        ],
        source_context("software-authority"),
    );
    assert_eq!(
        schema.validate_fact(&fact),
        Err(RelationError::TypeMismatch {
            field: "object".into(),
            expected: ValueSchema::Entity,
            actual: ValueKind::String,
        })
    );
}

#[test]
fn software_knowledge_and_workflow_share_one_relation_core() {
    for (predicate, authority) in [
        ("calls", "software-authority"),
        ("supports", "knowledge-authority"),
        ("permits", "workflow-authority"),
    ] {
        let schema = binary_schema(predicate);
        let subject = id(&format!("{predicate}:subject"), |bytes| {
            EntityId::from_canonical_bytes(bytes).expect("subject")
        });
        let object = id(&format!("{predicate}:object"), |bytes| {
            EntityId::from_canonical_bytes(bytes).expect("object")
        });
        let fact = Fact::new(
            id(&format!("{predicate}:fact"), |bytes| {
                FactId::from_canonical_bytes(bytes).expect("fact")
            }),
            schema.id(),
            vec![Value::Entity(subject), Value::Entity(object)],
            source_context(authority),
        );

        assert_eq!(schema.validate_fact(&fact), Ok(()));
        assert_eq!(
            fact.context().completeness(),
            EvidenceCompleteness::Complete
        );
        assert_eq!(fact.context().validity(), FactValidity::Valid);
    }
}

#[test]
fn malformed_schema_fails_closed() {
    let relation = id("duplicate", |bytes| {
        RelationId::from_canonical_bytes(bytes).expect("relation")
    });
    let duplicate = vec![
        RelationField::new("entity", ValueSchema::Entity, false).expect("field"),
        RelationField::new("entity", ValueSchema::Entity, false).expect("field"),
    ];
    assert_eq!(
        RelationSchema::new(relation, "duplicate", duplicate, Vec::new(),),
        Err(RelationError::DuplicateFieldName("entity".into()))
    );
    assert_eq!(
        RelationSchema::new(relation, "empty", Vec::new(), Vec::new(),),
        Err(RelationError::EmptyFields)
    );
}

#[test]
fn wrong_relation_and_arity_fail_closed() {
    let schema = binary_schema("supports");
    let authority = id("authority", |bytes| {
        EntityId::from_canonical_bytes(bytes).expect("authority")
    });
    let context = source_context("knowledge-authority");
    let wrong_relation = Fact::new(
        id("wrong-relation-fact", |bytes| {
            FactId::from_canonical_bytes(bytes).expect("fact")
        }),
        id("another-relation", |bytes| {
            RelationId::from_canonical_bytes(bytes).expect("relation")
        }),
        vec![Value::Entity(authority), Value::Entity(authority)],
        context,
    );
    assert_eq!(
        schema.validate_fact(&wrong_relation),
        Err(RelationError::WrongRelation)
    );

    let wrong_arity = Fact::new(
        id("wrong-arity-fact", |bytes| {
            FactId::from_canonical_bytes(bytes).expect("fact")
        }),
        schema.id(),
        vec![Value::Entity(authority)],
        context,
    );
    assert_eq!(
        schema.validate_fact(&wrong_arity),
        Err(RelationError::ArityMismatch {
            expected: 2,
            actual: 1,
        })
    );
}

#[test]
fn recursive_schema_validates_nullability_decimal_and_nested_records() {
    let relation = id("typed-document", |bytes| {
        RelationId::from_canonical_bytes(bytes).expect("relation")
    });
    let schema = RelationSchema::new(
        relation,
        "typed-document",
        vec![
            RelationField::new(
                "amount",
                ValueSchema::Decimal {
                    precision: 8,
                    scale: 2,
                },
                false,
            )
            .expect("amount"),
            RelationField::new(
                "metadata",
                ValueSchema::Record {
                    fields: vec![
                        RelationField::new("label", ValueSchema::String, false).expect("label"),
                        RelationField::new(
                            "samples",
                            ValueSchema::List {
                                element: Box::new(ValueSchema::Float {
                                    width: FloatWidth::Binary64,
                                }),
                                element_nullable: true,
                            },
                            false,
                        )
                        .expect("samples"),
                    ],
                },
                true,
            )
            .expect("metadata"),
        ],
        vec![RelationConstraint::Key(vec!["amount".into()])],
    )
    .expect("recursive schema");
    let valid = Fact::new(
        id("typed-document-valid", |bytes| {
            FactId::from_canonical_bytes(bytes).expect("fact")
        }),
        relation,
        vec![
            Value::Decimal("1234.50".into()),
            Value::Record(vec![
                ("label".into(), Value::String("observed".into())),
                (
                    "samples".into(),
                    Value::List(vec![Value::Float("1.5".into()), Value::Null]),
                ),
            ]),
        ],
        source_context("typed-authority"),
    );
    assert_eq!(schema.validate_fact(&valid), Ok(()));

    let noncanonical = Fact::new(
        id("typed-document-invalid", |bytes| {
            FactId::from_canonical_bytes(bytes).expect("fact")
        }),
        relation,
        vec![Value::Decimal("01234.50".into()), Value::Null],
        source_context("typed-authority"),
    );
    assert!(matches!(
        schema.validate_fact(&noncanonical),
        Err(RelationError::InvalidValue { field, .. }) if field == "amount"
    ));
}

#[test]
fn nullability_and_constraint_references_fail_closed() {
    let relation = id("nullable", |bytes| {
        RelationId::from_canonical_bytes(bytes).expect("relation")
    });
    let schema = RelationSchema::new(
        relation,
        "nullable",
        vec![
            RelationField::new(
                "observed-at",
                ValueSchema::Timestamp {
                    unit: TemporalUnit::Microsecond,
                    timezone: TimezonePolicy::Utc,
                },
                false,
            )
            .expect("field"),
        ],
        Vec::new(),
    )
    .expect("schema");
    let null = Fact::new(
        id("null", |bytes| {
            FactId::from_canonical_bytes(bytes).expect("fact")
        }),
        relation,
        vec![Value::Null],
        source_context("authority"),
    );
    assert_eq!(
        schema.validate_fact(&null),
        Err(RelationError::NullNotAllowed("observed-at".into()))
    );

    let invalid = RelationSchema::new(
        relation,
        "invalid-constraint",
        vec![RelationField::new("known", ValueSchema::String, false).expect("field")],
        vec![RelationConstraint::Unique(vec!["missing".into()])],
    );
    assert!(matches!(invalid, Err(RelationError::InvalidConstraint(_))));
}

#[test]
fn authority_and_provenance_must_describe_one_origin() {
    let authority = id("context-authority", |bytes| {
        EntityId::from_canonical_bytes(bytes).expect("authority")
    });
    let other = id("context-source", |bytes| {
        EntityId::from_canonical_bytes(bytes).expect("source")
    });
    let generation = id("context-generation", |bytes| {
        GenerationId::from_canonical_bytes(bytes).expect("generation")
    });
    assert_eq!(
        RelationContext::new(
            generation,
            RelationAuthority::Entity(authority),
            FactProvenance::Source(other),
            EvidenceCompleteness::Complete,
            FactValidity::Valid,
        ),
        Err(RelationContextError::AuthorityProvenanceMismatch)
    );

    let rule = id("context-rule", |bytes| {
        RuleId::from_canonical_bytes(bytes).expect("rule")
    });
    assert_eq!(
        RelationContext::new(
            generation,
            RelationAuthority::Rule(rule),
            FactProvenance::Source(authority),
            EvidenceCompleteness::Complete,
            FactValidity::Valid,
        ),
        Err(RelationContextError::AuthorityProvenanceMismatch)
    );

    let derivation = id("context-derivation", |bytes| {
        DerivationId::from_canonical_bytes(bytes).expect("derivation")
    });
    assert!(
        RelationContext::new(
            generation,
            RelationAuthority::Rule(rule),
            FactProvenance::Derivation(derivation),
            EvidenceCompleteness::Complete,
            FactValidity::Valid,
        )
        .is_ok()
    );
}

#[test]
fn fact_cannot_invalidate_itself() {
    let schema = binary_schema("self-invalidating");
    let fact_id = id("self-invalidating-fact", |bytes| {
        FactId::from_canonical_bytes(bytes).expect("fact")
    });
    let authority = id("self-invalidating-authority", |bytes| {
        EntityId::from_canonical_bytes(bytes).expect("authority")
    });
    let fact = Fact::new(
        fact_id,
        schema.id(),
        vec![Value::Entity(authority), Value::Entity(authority)],
        RelationContext::new(
            id("self-invalidating-generation", |bytes| {
                GenerationId::from_canonical_bytes(bytes).expect("generation")
            }),
            RelationAuthority::Entity(authority),
            FactProvenance::Source(authority),
            EvidenceCompleteness::Complete,
            FactValidity::InvalidatedBy(fact_id),
        )
        .expect("coherent context"),
    );
    assert_eq!(
        schema.validate_fact(&fact),
        Err(RelationError::SelfInvalidation(fact_id))
    );
}

#[test]
fn temporal_and_duration_values_use_the_canonical_v1_profile() {
    let relation = id("temporal-profile", |bytes| {
        RelationId::from_canonical_bytes(bytes).expect("relation")
    });
    let schema = RelationSchema::new(
        relation,
        "temporal-profile",
        vec![
            RelationField::new("date", ValueSchema::Date, false).unwrap(),
            RelationField::new(
                "timestamp",
                ValueSchema::Timestamp {
                    unit: TemporalUnit::Microsecond,
                    timezone: TimezonePolicy::Utc,
                },
                false,
            )
            .unwrap(),
            RelationField::new("duration", ValueSchema::Duration, false).unwrap(),
        ],
        Vec::new(),
    )
    .unwrap();
    let fact = |label: &str, date: &str, timestamp: &str, duration: &str| {
        Fact::new(
            id(label, |bytes| {
                FactId::from_canonical_bytes(bytes).expect("fact")
            }),
            relation,
            vec![
                Value::Date(date.into()),
                Value::Timestamp(timestamp.into()),
                Value::Duration(duration.into()),
            ],
            source_context("temporal-authority"),
        )
    };

    assert_eq!(
        schema.validate_fact(&fact(
            "valid-temporal",
            "2024-02-29",
            "2024-02-29T23:59:59.123456Z",
            "P1DT2H3M4.5S",
        )),
        Ok(())
    );
    for (label, date, timestamp, duration) in [
        (
            "invalid-date",
            "2023-02-29",
            "2024-02-29T23:59:59.123456Z",
            "P1D",
        ),
        (
            "invalid-time",
            "2024-02-29",
            "2024-02-29T24:00:00.000000Z",
            "P1D",
        ),
        (
            "repeated-duration-unit",
            "2024-02-29",
            "2024-02-29T23:59:59.123456Z",
            "P1Y2Y",
        ),
        (
            "ambiguous-duration",
            "2024-02-29",
            "2024-02-29T23:59:59.123456Z",
            "P1W2D",
        ),
    ] {
        assert!(matches!(
            schema.validate_fact(&fact(label, date, timestamp, duration)),
            Err(RelationError::InvalidValue { .. })
        ));
    }
}

#[test]
fn binary_fact_inline_values_preserve_the_v1_sequence_contract() {
    let subject = id("inline-subject", |bytes| {
        EntityId::from_canonical_bytes(bytes).expect("subject")
    });
    let object = id("inline-object", |bytes| {
        EntityId::from_canonical_bytes(bytes).expect("object")
    });
    let fact = Fact::new_binary(
        id("inline-fact", |bytes| {
            FactId::from_canonical_bytes(bytes).expect("fact")
        }),
        id("inline-relation", |bytes| {
            RelationId::from_canonical_bytes(bytes).expect("relation")
        }),
        [Value::Entity(subject), Value::Entity(object)],
        source_context("inline-authority"),
    );

    let encoded = serde_json::to_value(&fact).expect("serialize inline fact");
    assert_eq!(encoded["values"].as_array().map(Vec::len), Some(2));
    let decoded: Fact = serde_json::from_value(encoded).expect("deserialize inline fact");
    assert_eq!(decoded, fact);
}
