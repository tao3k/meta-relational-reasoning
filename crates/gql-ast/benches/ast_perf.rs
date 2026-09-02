use std::hint::black_box;
use std::time::Duration;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use gql_ast::lower_from_syntax;

#[path = "../tests/unit/ast_performance_scenario.rs"]
mod ast_performance_scenario;

use ast_performance_scenario::{
    CATALOG_SOURCE, CHARACTER_SEQUENCE_ESCAPES_SOURCE, GENERAL_LITERALS_EXPANDED_SOURCE,
    GENERAL_LITERALS_SOURCE, LEXICAL_IDENTIFIERS_SOURCE, LEXICAL_NUMERICS_SOURCE, MAX_TOTAL_MILLIS,
    MUTATION_SOURCE, NESTED_GRAPH_TYPE_SOURCE, QUERY_SOURCE, REFERENCE_AND_PREDEFINED_SOURCE,
    SOURCES, VALUE_TYPE_LATTICE_SOURCE, gql_ast_performance_scenario_package,
};

const BASELINE_SOURCES: [&str; 3] = [QUERY_SOURCE, CATALOG_SOURCE, MUTATION_SOURCE];
const QUALIFIED_NESTED_GRAPH_TYPE_BASELINE: &str = "CREATE GRAPH TYPE analytics.social_type AS { NODE TYPE Person (person {name STRING, age INT64}), NODE TYPE Company (company {name STRING}), EDGE TYPE WorksAt (person)-[{since INT64}]->(company) }";
const EDGE_TYPE_SPECIFICATION_BASELINE: &str = "CREATE GRAPH TYPE analytics.social_type AS { NODE TYPE Person (person {name STRING, age INT64}), NODE TYPE Company (company {name STRING}), DIRECTED RELATIONSHIP TYPE WorksAt {since INT64} CONNECTING (person TO company), EDGE TYPE Transfer ({id INT64})-[{}]->() }";
const CRITERION_SAMPLE_SIZE: usize = 100;
const CRITERION_WARMUP_SECONDS: u64 = 2;
const CRITERION_MEASUREMENT_SECONDS: u64 = MAX_TOTAL_MILLIS + 3;

fn configure_group(group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>) {
    group.sample_size(CRITERION_SAMPLE_SIZE);
    group.warm_up_time(Duration::from_secs(CRITERION_WARMUP_SECONDS));
    group.measurement_time(Duration::from_secs(CRITERION_MEASUREMENT_SECONDS));
    group.significance_level(0.01);
    group.noise_threshold(0.03);
}

fn benchmark_single_source(
    criterion: &mut Criterion,
    group_name: &str,
    source_name: &str,
    source: &str,
) {
    let mut group = criterion.benchmark_group(group_name);
    configure_group(&mut group);
    group.throughput(Throughput::Bytes(source.len() as u64));
    group.bench_function("lossless-rowan-parse", |bencher| {
        bencher.iter(|| {
            black_box(gql_syntax::parse(source_name, black_box(source)));
        });
    });
    let parsed = gql_syntax::parse(source_name, source);
    group.bench_function("cst-to-ast-lowering", |bencher| {
        bencher.iter(|| black_box(lower_from_syntax(black_box(&parsed))));
    });
    group.bench_function("parse-and-ast-lowering", |bencher| {
        bencher.iter(|| {
            let parse = gql_syntax::parse(source_name, black_box(source));
            black_box(lower_from_syntax(black_box(&parse)));
        });
    });
    group.finish();
}

fn benchmark_character_sequence_decoder(criterion: &mut Criterion) {
    const SEQUENCES: [&str; 6] = [
        "'plain'",
        "\"plain\"",
        r"'A\nB\u0041\U01F642'",
        r#""A\nB\u0041\U01F642""#,
        r"@'A\nB'",
        r#"@"A\nB""#,
    ];
    let bytes = SEQUENCES.iter().map(|value| value.len() as u64).sum();
    let mut group = criterion.benchmark_group("character-sequence-decoder-v1");
    configure_group(&mut group);
    group.throughput(Throughput::Bytes(bytes));
    group.bench_function("validate-and-decode", |bencher| {
        bencher.iter(|| {
            for sequence in SEQUENCES {
                black_box(
                    gql_syntax::decode_character_string(black_box(sequence))
                        .expect("benchmark sequence is valid"),
                );
            }
        });
    });
    group.finish();
}

fn ast_hot_path(criterion: &mut Criterion) {
    let package = gql_ast_performance_scenario_package();
    let scenario = &package.scenarios[0];
    assert_eq!(SOURCES.len(), BASELINE_SOURCES.len() + 1);
    let source_bytes = BASELINE_SOURCES
        .iter()
        .map(|source| source.len() as u64)
        .sum();
    let mut group = criterion.benchmark_group(scenario.name);
    configure_group(&mut group);
    group.throughput(Throughput::Bytes(source_bytes));

    group.bench_function("lossless-rowan-parse", |bencher| {
        bencher.iter(|| {
            for source in BASELINE_SOURCES {
                black_box(gql_syntax::parse("ast-benchmark.gql", black_box(source)));
            }
        });
    });

    let parsed = BASELINE_SOURCES.map(|source| gql_syntax::parse("ast-benchmark.gql", source));
    group.bench_function("cst-to-ast-lowering", |bencher| {
        bencher.iter(|| {
            for parse in &parsed {
                black_box(lower_from_syntax(black_box(parse)));
            }
        });
    });

    group.bench_function("parse-and-ast-lowering", |bencher| {
        bencher.iter(|| {
            for source in BASELINE_SOURCES {
                let parse = gql_syntax::parse("ast-benchmark.gql", black_box(source));
                black_box(lower_from_syntax(black_box(&parse)));
            }
        });
    });
    group.finish();

    benchmark_single_source(
        criterion,
        "qualified-nested-graph-type-v1",
        "nested-graph-type-baseline.gql",
        QUALIFIED_NESTED_GRAPH_TYPE_BASELINE,
    );
    benchmark_single_source(
        criterion,
        "edge-type-specification-v1",
        "edge-type-specification-benchmark.gql",
        EDGE_TYPE_SPECIFICATION_BASELINE,
    );
    benchmark_single_source(
        criterion,
        "label-key-label-type-fillers-v1",
        "label-key-label-type-fillers-benchmark.gql",
        NESTED_GRAPH_TYPE_SOURCE,
    );
    benchmark_single_source(
        criterion,
        "value-type-lattice-v1",
        "value-type-lattice-benchmark.gql",
        VALUE_TYPE_LATTICE_SOURCE,
    );
    benchmark_single_source(
        criterion,
        "reference-and-predefined-types-v1",
        "reference-and-predefined-types-benchmark.gql",
        REFERENCE_AND_PREDEFINED_SOURCE,
    );
    benchmark_single_source(
        criterion,
        "lexical-identifiers-v1",
        "lexical-identifiers-benchmark.gql",
        LEXICAL_IDENTIFIERS_SOURCE,
    );
    benchmark_single_source(
        criterion,
        "lexical-numerics-v1",
        "lexical-numerics-benchmark.gql",
        LEXICAL_NUMERICS_SOURCE,
    );
    benchmark_single_source(
        criterion,
        "general-literals-v1",
        "general-literals-benchmark.gql",
        GENERAL_LITERALS_SOURCE,
    );
    benchmark_single_source(
        criterion,
        "general-literals-expanded-v1",
        "general-literals-expanded-benchmark.gql",
        GENERAL_LITERALS_EXPANDED_SOURCE,
    );
    benchmark_single_source(
        criterion,
        "character-sequence-escapes-v1",
        "character-sequence-escapes-benchmark.gql",
        CHARACTER_SEQUENCE_ESCAPES_SOURCE,
    );
    benchmark_character_sequence_decoder(criterion);
}

criterion_group!(benches, ast_hot_path);
criterion_main!(benches);
