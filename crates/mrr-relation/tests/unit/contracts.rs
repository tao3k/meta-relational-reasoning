use crate::{
    EntityId, EvidenceCompleteness, Fact, FactId, FactProvenance, FactValidity, GenerationId,
    RelationAuthority, RelationCardinality, RelationContext, RelationError, RelationField,
    RelationId, RelationSchema, Value, ValueType,
};

fn id<T>(label: &str, derive: impl FnOnce(&[u8]) -> T) -> T {
    derive(format!("mrr-relation-test:{label}").as_bytes())
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
}

fn binary_schema(predicate: &str) -> RelationSchema {
    RelationSchema::new(
        id(predicate, |bytes| {
            RelationId::from_canonical_bytes(bytes).expect("relation")
        }),
        predicate,
        vec![
            RelationField::new("subject", ValueType::Entity).expect("subject field"),
            RelationField::new("object", ValueType::Entity).expect("object field"),
        ],
        RelationCardinality::ManyToMany,
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
            expected: ValueType::Entity,
            actual: ValueType::String,
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
        RelationField::new("entity", ValueType::Entity).expect("field"),
        RelationField::new("entity", ValueType::Entity).expect("field"),
    ];
    assert_eq!(
        RelationSchema::new(
            relation,
            "duplicate",
            duplicate,
            RelationCardinality::ManyToMany,
        ),
        Err(RelationError::DuplicateFieldName("entity".into()))
    );
    assert_eq!(
        RelationSchema::new(
            relation,
            "empty",
            Vec::new(),
            RelationCardinality::ManyToMany,
        ),
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
