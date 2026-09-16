use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use mrr_frontends::QueryFrontend;
use mrr_gerbil::parse_gql_artifact;

const QUERY: &str = "MATCH (a:Module)-[:DEPENDS_ON]->(b:Module) WHERE a.name = 'runtime' RETURN b";

fn bench_frontend(c: &mut Criterion) {
    // Initialize the embedded Gambit runtime and the parser-owned kind catalog
    // before measuring steady-state request work. Cold initialization is a
    // process lifecycle concern and must not be folded into parser throughput.
    parse_gql_artifact(QUERY).expect("benchmark parser warm-up");

    let frontend = QueryFrontend::new();
    let mut group = c.benchmark_group("mrr_gql_runtime");
    group.throughput(Throughput::Bytes(QUERY.len() as u64));
    group.bench_function("parser_owned_parse_artifact", |bencher| {
        bencher.iter(|| parse_gql_artifact(std::hint::black_box(QUERY)).expect("benchmark query"));
    });
    group.bench_function("parser_owned_rowan_cst", |bencher| {
        bencher.iter(|| {
            parse_gql_artifact(std::hint::black_box(QUERY))
                .expect("benchmark query")
                .to_rowan_cst()
                .expect("accepted parser artifact")
        });
    });
    group.bench_function("frontend_meta_query", |bencher| {
        bencher.iter(|| {
            frontend
                .compile("benchmark.gql", std::hint::black_box(QUERY))
                .expect("benchmark query")
        });
    });
    group.finish();
}

criterion_group!(benches, bench_frontend);
criterion_main!(benches);
