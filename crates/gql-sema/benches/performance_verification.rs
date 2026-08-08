use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn performance_verification_smoke(criterion: &mut Criterion) {
    criterion.bench_function("performance_verification_smoke", |bencher| {
        bencher.iter(|| black_box(1 + 1))
    });
}

criterion_group!(performance_verification, performance_verification_smoke);
criterion_main!(performance_verification);
