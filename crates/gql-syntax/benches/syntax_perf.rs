use criterion::{Criterion, criterion_group, criterion_main};

use gql_syntax::parse;

fn syntax_parse_benchmark(c: &mut Criterion) {
    c.bench_function("parse_simple_match_return", |b| {
        b.iter(|| {
            let parsed = parse("match", "MATCH (n) RETURN n");
            assert!(!parsed.tree.root().children().is_empty() || !parsed.diagnostics.is_empty());
        });
    });
}

criterion_group!(benches, syntax_parse_benchmark);
criterion_main!(benches);
