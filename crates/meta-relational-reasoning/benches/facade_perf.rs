use core::num::NonZeroUsize;
use std::time::Duration;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use meta_relational_reasoning::{
    Binding, CandidateQueryResult, Direction, Expression, ExternalRevisionIdentity, GenerationId,
    GraphPattern, MrrEngine, NodePattern, PathPattern, PathSegment, Projection, QueryId,
    QueryOperatorId, QueryResult, QueryResultBinding, QueryResultLimits, QueryResultValue,
    QueryTemplate, ReasoningBundle, ReasoningBundleDeclaration, RelationField, RelationId,
    RelationPattern, RelationSchema, RevisionBinding, SemanticSnapshot, SetQuantifier, Value,
    ValueSchema, admit_query_result_candidate,
};

fn id<T>(label: &str, derive: impl FnOnce(&[u8]) -> T) -> T {
    derive(format!("query-result-benchmark:{label}").as_bytes())
}

fn bound_query() -> meta_relational_reasoning::CatalogBoundQuery {
    let relation = id("relation", |bytes| {
        RelationId::from_canonical_bytes(bytes).unwrap()
    });
    let query_id = id("query", |bytes| {
        QueryId::from_canonical_bytes(bytes).unwrap()
    });
    let generation = id("generation", |bytes| {
        GenerationId::from_canonical_bytes(bytes).unwrap()
    });
    let binding = |name| Binding::new(name).unwrap();
    let query = meta_relational_reasoning::MetaQueryIr::new(
        query_id,
        GraphPattern::new(
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
        .unwrap(),
        Vec::new(),
        QueryResult::returning(SetQuantifier::All).with_projections(vec![Projection::new(
            id("projection", |bytes| {
                QueryOperatorId::from_canonical_bytes(bytes).unwrap()
            }),
            Expression::Literal(Value::Integer(1)),
            binding("value"),
        )]),
    )
    .unwrap();
    let bundle = ReasoningBundle::admit(ReasoningBundleDeclaration {
        relations: vec![
            RelationSchema::new(
                relation,
                "depends_on",
                vec![
                    RelationField::new("subject", ValueSchema::Entity, false).unwrap(),
                    RelationField::new("object", ValueSchema::Entity, false).unwrap(),
                ],
                Vec::new(),
            )
            .unwrap(),
        ],
        query_templates: vec![QueryTemplate::new(query, Vec::new())],
        ..ReasoningBundleDeclaration::default()
    })
    .unwrap();
    let snapshot = SemanticSnapshot::admit(
        generation,
        vec![
            RevisionBinding::admit(
                ExternalRevisionIdentity::new("git", "repository", "benchmark").unwrap(),
                generation,
            )
            .unwrap(),
        ],
    )
    .unwrap();
    MrrEngine::builder()
        .with_bundle(bundle)
        .build()
        .unwrap()
        .bind_query(query_id, &snapshot)
        .unwrap()
}

fn candidate(
    query: &meta_relational_reasoning::CatalogBoundQuery,
    rows: usize,
) -> CandidateQueryResult {
    CandidateQueryResult::new(
        QueryResultBinding::for_query(query),
        vec![Binding::new("value").unwrap()],
        (0..rows)
            .map(|value| {
                vec![QueryResultValue::scalar(
                    ValueSchema::Integer,
                    Value::Integer(value as i64),
                )]
            })
            .collect(),
    )
}

fn bench(c: &mut Criterion) {
    c.bench_function("mrr_facade_identity", |b| {
        b.iter(|| RelationId::from_canonical_bytes(std::hint::black_box(b"relation:benchmark")))
    });

    let query = bound_query();
    let mut group = c.benchmark_group("query_result_admission");
    for rows in [1_000, 10_000, 100_000] {
        let candidate = candidate(&query, rows);
        let limits = QueryResultLimits::new(
            NonZeroUsize::new(rows).unwrap(),
            NonZeroUsize::new(rows).unwrap(),
        );
        group.throughput(Throughput::Elements(rows as u64));
        group.bench_with_input(BenchmarkId::from_parameter(rows), &rows, |b, _| {
            b.iter(|| {
                admit_query_result_candidate(
                    std::hint::black_box(&query),
                    std::hint::black_box(&candidate),
                    limits,
                )
                .unwrap()
            });
        });
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .measurement_time(Duration::from_secs(5));
    targets = bench
}
criterion_main!(benches);
