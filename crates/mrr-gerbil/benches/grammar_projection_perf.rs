use criterion::{Criterion, criterion_group, criterion_main};
use mrr_gerbil::{stamp_projection, validate_projection};

fn bench(c: &mut Criterion) {
    let input = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let projection = stamp_projection("generated grammar body\n", input);
    c.bench_function("mrr_gerbil_projection_admission", |b| {
        b.iter(|| validate_projection(std::hint::black_box(&projection), input))
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
