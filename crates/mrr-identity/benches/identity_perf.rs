use criterion::{Criterion, criterion_group, criterion_main};
fn bench(c: &mut Criterion) {
    c.bench_function("mrr_identity_new", |b| {
        b.iter(|| {
            mrr_identity::RelationId::from_canonical_bytes(std::hint::black_box(
                b"relation:benchmark",
            ))
        })
    });
}
criterion_group!(benches, bench);
criterion_main!(benches);
