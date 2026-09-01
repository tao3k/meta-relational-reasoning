use criterion::{Criterion, criterion_group, criterion_main};
use mrr_frontends::{QueryFrontend, QueryLanguage};

const QUERY: &str = "MATCH (a:Module)-[:DEPENDS_ON]->(b:Module) WHERE a.name = 'runtime' RETURN b";

fn bench_frontend(c: &mut Criterion) {
    let frontend = QueryFrontend::new(QueryLanguage::Gql);
    c.bench_function("mrr_frontend_gql_to_meta_query", |bencher| {
        bencher.iter(|| {
            frontend
                .compile("benchmark.gql", QUERY)
                .expect("benchmark query")
        });
    });
}

criterion_group!(benches, bench_frontend);
criterion_main!(benches);
