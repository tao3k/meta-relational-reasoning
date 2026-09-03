use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mrr_query::{
    Binding, Direction, Expression, GraphPattern, MetaQueryIr, NodePattern, PathPattern,
    PathSegment, Projection, QueryId, QueryOperatorId, RelationId, RelationPattern,
};

const SCALES: &[usize] = &[1_000, 10_000, 100_000];

fn query(size: usize) -> MetaQueryIr {
    let left = Binding::new("left").expect("left binding");
    let right = Binding::new("right").expect("right binding");
    let relation_types = (0..size)
        .rev()
        .map(|index| {
            RelationId::from_canonical_bytes(format!("query:relation:{index}"))
                .expect("relation identity")
        })
        .collect();
    let graph = GraphPattern::new(
        QueryOperatorId::from_canonical_bytes(b"query:graph").expect("graph identity"),
        vec![PathPattern::new(
            NodePattern::new(left, vec![]),
            vec![PathSegment::new(
                RelationPattern::new(None, relation_types, Direction::Outgoing, 1, Some(1))
                    .expect("relation pattern"),
                NodePattern::new(right.clone(), vec![]),
            )],
        )],
    )
    .expect("graph pattern");
    MetaQueryIr::new(
        QueryId::from_canonical_bytes(b"query:normalization").expect("query identity"),
        graph,
        vec![],
        vec![Projection::new(
            QueryOperatorId::from_canonical_bytes(b"query:projection")
                .expect("projection identity"),
            Expression::Binding(right.clone()),
            right,
        )],
        vec![],
        vec![],
        None,
    )
    .expect("query IR")
}

fn bench(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("query_normalization");
    for &size in SCALES {
        let fixture = query(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &fixture,
            |bencher, query| bencher.iter(|| std::hint::black_box(query.clone()).normalized()),
        );
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
