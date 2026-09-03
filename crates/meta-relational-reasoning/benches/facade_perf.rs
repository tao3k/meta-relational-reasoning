use criterion::{Criterion, criterion_group, criterion_main};
fn bench(c: &mut Criterion) {
    c.bench_function("mrr_facade_identity", |b| {
        b.iter(|| {
            meta_relational_reasoning::RelationId::from_canonical_bytes(std::hint::black_box(
                b"relation:benchmark",
            ))
        })
    });
}
criterion_group!(benches, bench);
criterion_main!(benches);
