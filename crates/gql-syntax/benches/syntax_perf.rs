use criterion::{Criterion, criterion_group, criterion_main};

use gql_syntax::parse;

fn syntax_parse_benchmark(c: &mut Criterion) {
    c.bench_function("parse_simple_match_return", |b| {
        b.iter(|| {
            let parsed = parse("match", "MATCH (n) RETURN n");
            assert!(!parsed.tree.root().children().is_empty() || !parsed.diagnostics.is_empty());
        });
    });
    c.bench_function("parse_query_composition_ordering_pagination", |b| {
        b.iter(|| {
            let parsed = parse(
                "composed-query",
                "MATCH (a)-[:CALLS]->(b) WHERE a.name = 'Ada' RETURN b AS target ORDER BY target.name DESC LIMIT 10 OFFSET 2 UNION MATCH (c) RETURN c ORDER BY c ASC LIMIT 10 OFFSET 2",
            );
            assert!(!parsed.tree.root().children().is_empty() || !parsed.diagnostics.is_empty());
        });
    });
    c.bench_function("recover_malformed_query_losslessly", |b| {
        b.iter(|| {
            let source = "MATCH (a)-[:CALLS:Person->(b) RETURN a ORDER BY LIMIT 10 OFFSET @";
            let parsed = parse("malformed-query", source);
            assert_eq!(parsed.tree.rowan_root().text().to_string(), source);
            assert!(!parsed.diagnostics.is_empty());
        });
    });
}

criterion_group!(benches, syntax_parse_benchmark);
criterion_main!(benches);
