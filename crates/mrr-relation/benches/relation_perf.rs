use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mrr_relation::{
    EntityId, EvidenceCompleteness, Fact, FactId, FactProvenance, FactValidity, GenerationId,
    RelationAuthority, RelationCardinality, RelationContext, RelationField, RelationId,
    RelationSchema, Value, ValueType,
};

const SCALES: &[usize] = &[1_000, 10_000, 100_000];

fn fixture(size: usize) -> (RelationSchema, Vec<Fact>) {
    let relation = RelationId::from_canonical_bytes(b"relation:ingestion").expect("relation id");
    let schema = RelationSchema::new(
        relation,
        "observed",
        vec![RelationField::new("value", ValueType::String).expect("field")],
        RelationCardinality::ManyToMany,
    )
    .expect("schema");
    let generation =
        GenerationId::from_canonical_bytes(b"relation:generation").expect("generation identity");
    let authority =
        EntityId::from_canonical_bytes(b"relation:authority").expect("authority identity");
    let facts = (0..size)
        .map(|index| {
            Fact::new(
                FactId::from_canonical_bytes(format!("relation:fact:{index}"))
                    .expect("fact identity"),
                relation,
                vec![Value::String(index.to_string())],
                RelationContext::new(
                    generation,
                    RelationAuthority::Entity(authority),
                    FactProvenance::Source(authority),
                    EvidenceCompleteness::Complete,
                    FactValidity::Valid,
                ),
            )
        })
        .collect();
    (schema, facts)
}

fn bench(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("relation_ingestion");
    for &size in SCALES {
        let (schema, facts) = fixture(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &facts,
            |bencher, facts| {
                bencher.iter(|| {
                    for fact in facts {
                        std::hint::black_box(schema.validate_fact(std::hint::black_box(fact)))
                            .expect("valid fact");
                    }
                })
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
