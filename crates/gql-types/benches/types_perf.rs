use criterion::{criterion_group, criterion_main, Criterion};

fn bench_smoke(c: &mut Criterion) {
    c.bench_function("types_smoke", |b| b.iter(|| {
        let _ = 1u32 + 1;
    }));
}

criterion_group!(benches, bench_smoke);
criterion_main!(benches);
