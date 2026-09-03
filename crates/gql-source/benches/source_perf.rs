use criterion::{Criterion, criterion_group, criterion_main};

use gql_source::{SourceText, Span};

fn source_construction_benchmark(c: &mut Criterion) {
    c.bench_function("source_text_slice", |b| {
        b.iter(|| {
            let source = SourceText::new("query", "MATCH (n) RETURN n");
            let span = Span::new(0, source.text().len() as u32);
            assert_eq!(source.slice(span), Some("MATCH (n) RETURN n"));
        });
    });
}

criterion_group!(benches, source_construction_benchmark);
criterion_main!(benches);
