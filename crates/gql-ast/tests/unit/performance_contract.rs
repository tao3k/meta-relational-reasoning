use std::hint::black_box;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use asp_rust_build_support::{
    AspRustScenario, AspRustScenarioObservation, AspRustScenarioPackage, measure_asp_rust_scenario,
    render_asp_rust_scenario_benchmark_toml,
};

use super::ast_performance_scenario::{
    CHARACTER_SEQUENCE_ESCAPES_SOURCE, CHARACTER_SEQUENCE_ESCAPES_SOURCE_BYTES,
    GENERAL_LITERALS_EXPANDED_SOURCE, GENERAL_LITERALS_EXPANDED_SOURCE_BYTES,
    GENERAL_LITERALS_SOURCE, GENERAL_LITERALS_SOURCE_BYTES, LEXICAL_IDENTIFIERS_SOURCE,
    LEXICAL_IDENTIFIERS_SOURCE_BYTES, LEXICAL_NUMERICS_SOURCE, LEXICAL_NUMERICS_SOURCE_BYTES,
    MAX_TOTAL_MILLIS, MEMORY_BUDGET_BYTES, REFERENCE_AND_PREDEFINED_SOURCE,
    REFERENCE_AND_PREDEFINED_SOURCE_BYTES, SOURCE_BYTES, SOURCES, VALUE_TYPE_LATTICE_SOURCE,
    VALUE_TYPE_SOURCE_BYTES, gql_ast_performance_scenario_package,
};
use crate::lower_from_syntax;

static PERFORMANCE_SCENARIO_LANE: Mutex<()> = Mutex::new(());

fn hold_performance_scenario_lane() -> MutexGuard<'static, ()> {
    PERFORMANCE_SCENARIO_LANE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn scenario_by_name<'a>(package: &'a AspRustScenarioPackage, name: &str) -> &'a AspRustScenario {
    package
        .scenarios
        .iter()
        .find(|scenario| scenario.name == name)
        .unwrap_or_else(|| panic!("scenario package is missing stable identity `{name}`"))
}

fn run_frontend_hot_path() -> AspRustScenarioObservation {
    let parse_started_at = Instant::now();
    let parsed = SOURCES.map(|source| gql_syntax::parse("ast-hot-path.gql", black_box(source)));
    let parse_elapsed = parse_started_at.elapsed();

    let ast_started_at = Instant::now();
    let lowered = parsed
        .each_ref()
        .map(|parse| lower_from_syntax(black_box(parse)));
    let ast_elapsed = ast_started_at.elapsed();

    let parse_diagnostic_count = parsed
        .iter()
        .map(|parse| parse.diagnostics.len() as u64)
        .sum::<u64>();
    let ast_diagnostic_count = lowered
        .iter()
        .map(|output| output.diagnostics.len() as u64)
        .sum::<u64>();
    let statement_count = lowered
        .iter()
        .filter(|output| output.statement.is_some())
        .count() as u64;
    let fallback_count = lowered.len() as u64 - statement_count;
    let roundtrip_bytes = parsed
        .iter()
        .map(|parse| parse.tree.rowan_root().text().to_string().len() as u64)
        .sum::<u64>();
    let observed_memory_bytes = SOURCE_BYTES
        .saturating_add(roundtrip_bytes)
        .saturating_add((lowered.len() * std::mem::size_of::<crate::SyntaxParseOutput>()) as u64);

    black_box(&lowered);
    AspRustScenarioObservation::default()
        .with_memory_bytes(observed_memory_bytes)
        .with_timing("parse", parse_elapsed)
        .with_timing("ast_lowering", ast_elapsed)
        .with_metric("source_bytes", SOURCE_BYTES)
        .with_metric("statement_count", statement_count)
        .with_metric(
            "diagnostic_count",
            parse_diagnostic_count + ast_diagnostic_count,
        )
        .with_metric("fallback_count", fallback_count)
        .with_metric("roundtrip_bytes", roundtrip_bytes)
}

fn run_value_type_hot_path() -> AspRustScenarioObservation {
    run_single_source_hot_path(
        "value-type-lattice.gql",
        VALUE_TYPE_LATTICE_SOURCE,
        VALUE_TYPE_SOURCE_BYTES,
    )
}

fn run_reference_and_predefined_hot_path() -> AspRustScenarioObservation {
    run_single_source_hot_path(
        "reference-and-predefined.gql",
        REFERENCE_AND_PREDEFINED_SOURCE,
        REFERENCE_AND_PREDEFINED_SOURCE_BYTES,
    )
}

fn run_lexical_identifiers_hot_path() -> AspRustScenarioObservation {
    run_single_source_hot_path(
        "lexical-identifiers.gql",
        LEXICAL_IDENTIFIERS_SOURCE,
        LEXICAL_IDENTIFIERS_SOURCE_BYTES,
    )
}

fn run_lexical_numerics_hot_path() -> AspRustScenarioObservation {
    run_single_source_hot_path(
        "lexical-numerics.gql",
        LEXICAL_NUMERICS_SOURCE,
        LEXICAL_NUMERICS_SOURCE_BYTES,
    )
}

fn run_general_literals_hot_path() -> AspRustScenarioObservation {
    run_single_source_hot_path(
        "general-literals.gql",
        GENERAL_LITERALS_SOURCE,
        GENERAL_LITERALS_SOURCE_BYTES,
    )
}

fn run_general_literals_expanded_hot_path() -> AspRustScenarioObservation {
    run_single_source_hot_path(
        "general-literals-expanded.gql",
        GENERAL_LITERALS_EXPANDED_SOURCE,
        GENERAL_LITERALS_EXPANDED_SOURCE_BYTES,
    )
}

fn run_character_sequence_escapes_hot_path() -> AspRustScenarioObservation {
    run_single_source_hot_path(
        "character-sequences.gql",
        CHARACTER_SEQUENCE_ESCAPES_SOURCE,
        CHARACTER_SEQUENCE_ESCAPES_SOURCE_BYTES,
    )
}

fn run_single_source_hot_path(
    source_name: &str,
    source: &str,
    source_bytes: u64,
) -> AspRustScenarioObservation {
    let parse_started_at = Instant::now();
    let parsed = gql_syntax::parse(source_name, black_box(source));
    let parse_elapsed = parse_started_at.elapsed();

    let ast_started_at = Instant::now();
    let lowered = lower_from_syntax(black_box(&parsed));
    let ast_elapsed = ast_started_at.elapsed();
    let roundtrip_bytes = parsed.tree.rowan_root().text().to_string().len() as u64;
    let statement_count = u64::from(lowered.statement.is_some());

    black_box(&lowered);
    AspRustScenarioObservation::default()
        .with_memory_bytes(
            source_bytes
                .saturating_add(roundtrip_bytes)
                .saturating_add(std::mem::size_of::<crate::SyntaxParseOutput>() as u64),
        )
        .with_timing("parse", parse_elapsed)
        .with_timing("ast_lowering", ast_elapsed)
        .with_metric("source_bytes", source_bytes)
        .with_metric("statement_count", statement_count)
        .with_metric(
            "diagnostic_count",
            (parsed.diagnostics.len() + lowered.diagnostics.len()) as u64,
        )
        .with_metric("fallback_count", 1 - statement_count)
        .with_metric("roundtrip_bytes", roundtrip_bytes)
}

#[test]
fn lossless_cst_to_ast_hot_path_stays_inside_scenario_budget() {
    let _lane = hold_performance_scenario_lane();
    let package = gql_ast_performance_scenario_package();
    assert_eq!(package.package_name, "gql-ast");
    let scenario = scenario_by_name(&package, "lossless-cst-to-ast-hot-path-v1");
    let measurement = measure_asp_rust_scenario(scenario, run_frontend_hot_path)
        .expect("measure the real parser and CST-to-AST Scenario");

    assert!(
        measurement.total_p95 <= Duration::from_millis(MAX_TOTAL_MILLIS),
        "GQL AST hot-path p95 {:?} exceeded {}ms",
        measurement.total_p95,
        MAX_TOTAL_MILLIS
    );
    assert!(
        measurement.observed_memory_bytes <= MEMORY_BUDGET_BYTES,
        "GQL AST hot-path memory estimate {} exceeded {} bytes",
        measurement.observed_memory_bytes,
        MEMORY_BUDGET_BYTES
    );
    assert!(measurement.observed_timings.contains_key("parse"));
    assert!(measurement.observed_timings.contains_key("ast_lowering"));

    let rendered = render_asp_rust_scenario_benchmark_toml(scenario, &measurement)
        .expect("render the measured ASP Rust Scenario receipt");
    assert!(rendered.contains("statistic = \"p95\""));
    assert!(rendered.contains("[metrics.fallback_count]"));
    assert!(rendered.contains("observed = 0"));
    eprintln!("measured GQL AST Scenario:\n{rendered}");
}

#[test]
fn value_type_lattice_stays_inside_scenario_budget() {
    let _lane = hold_performance_scenario_lane();
    let package = gql_ast_performance_scenario_package();
    assert_eq!(package.package_name, "gql-ast");
    let scenario = scenario_by_name(&package, "value-type-lattice-v1");
    let measurement = measure_asp_rust_scenario(scenario, run_value_type_hot_path)
        .expect("measure the recursive value-type parser and AST Scenario");

    assert!(measurement.total_p95 <= Duration::from_millis(MAX_TOTAL_MILLIS));
    assert!(measurement.observed_memory_bytes <= MEMORY_BUDGET_BYTES);
    assert!(measurement.observed_timings.contains_key("parse"));
    assert!(measurement.observed_timings.contains_key("ast_lowering"));

    let rendered = render_asp_rust_scenario_benchmark_toml(scenario, &measurement)
        .expect("render the measured value-type Scenario receipt");
    assert!(rendered.contains("snapshot = \"gql_ast_value_type_lattice_v1\""));
    assert!(rendered.contains("observed = 0"));
    eprintln!("measured value-type lattice Scenario:\n{rendered}");
}

#[test]
fn reference_and_predefined_types_stay_inside_scenario_budget() {
    let _lane = hold_performance_scenario_lane();
    let package = gql_ast_performance_scenario_package();
    assert_eq!(package.package_name, "gql-ast");
    let scenario = scenario_by_name(&package, "reference-and-predefined-types-v1");
    let measurement = measure_asp_rust_scenario(scenario, run_reference_and_predefined_hot_path)
        .expect("measure the closed-reference and predefined-type AST Scenario");

    assert!(measurement.total_p95 <= Duration::from_millis(MAX_TOTAL_MILLIS));
    assert!(measurement.observed_memory_bytes <= MEMORY_BUDGET_BYTES);
    let rendered = render_asp_rust_scenario_benchmark_toml(scenario, &measurement)
        .expect("render the measured closed-reference Scenario receipt");
    assert!(rendered.contains("snapshot = \"gql_ast_reference_and_predefined_types_v1\""));
    assert!(rendered.contains("observed = 0"));
    eprintln!("measured reference/predefined Scenario:\n{rendered}");
}

#[test]
fn lexical_identifiers_stay_inside_scenario_budget() {
    let _lane = hold_performance_scenario_lane();
    let package = gql_ast_performance_scenario_package();
    assert_eq!(package.package_name, "gql-ast");
    let scenario = scenario_by_name(&package, "lexical-identifiers-v1");
    let measurement = measure_asp_rust_scenario(scenario, run_lexical_identifiers_hot_path)
        .expect("measure the ISO lexical identifier AST Scenario");

    assert!(measurement.total_p95 <= Duration::from_millis(MAX_TOTAL_MILLIS));
    assert!(measurement.observed_memory_bytes <= MEMORY_BUDGET_BYTES);
    let rendered = render_asp_rust_scenario_benchmark_toml(scenario, &measurement)
        .expect("render the measured lexical identifier Scenario receipt");
    assert!(rendered.contains("snapshot = \"gql_ast_lexical_identifiers_v1\""));
    assert!(rendered.contains("observed = 0"));
    eprintln!("measured lexical identifier Scenario:\n{rendered}");
}

#[test]
fn lexical_numerics_stay_inside_scenario_budget() {
    let _lane = hold_performance_scenario_lane();
    let package = gql_ast_performance_scenario_package();
    assert_eq!(package.package_name, "gql-ast");
    let scenario = scenario_by_name(&package, "lexical-numerics-v1");
    let measurement = measure_asp_rust_scenario(scenario, run_lexical_numerics_hot_path)
        .expect("measure the ISO lexical numeric AST Scenario");

    assert!(measurement.total_p95 <= Duration::from_millis(2));
    assert!(measurement.observed_memory_bytes <= MEMORY_BUDGET_BYTES);
    let rendered = render_asp_rust_scenario_benchmark_toml(scenario, &measurement)
        .expect("render the measured lexical numeric Scenario receipt");
    assert!(rendered.contains("snapshot = \"gql_ast_lexical_numerics_v1\""));
    assert!(rendered.contains("observed = 0"));
    eprintln!("measured lexical numeric Scenario:\n{rendered}");
}

#[test]
fn general_literals_stay_inside_scenario_budget() {
    let _lane = hold_performance_scenario_lane();
    let package = gql_ast_performance_scenario_package();
    assert_eq!(package.package_name, "gql-ast");
    let scenario = scenario_by_name(&package, "general-literals-v1");
    let measurement = measure_asp_rust_scenario(scenario, run_general_literals_hot_path)
        .expect("measure the ISO general literal AST Scenario");

    assert!(measurement.total_p95 <= Duration::from_millis(2));
    assert!(measurement.observed_memory_bytes <= MEMORY_BUDGET_BYTES);
    let rendered = render_asp_rust_scenario_benchmark_toml(scenario, &measurement)
        .expect("render the measured general literal Scenario receipt");
    assert!(rendered.contains("snapshot = \"gql_ast_general_literals_v1\""));
    assert!(rendered.contains("observed = 0"));
    eprintln!("measured general literal Scenario:\n{rendered}");
}

#[test]
fn general_literals_expanded_stay_inside_scenario_budget() {
    let _lane = hold_performance_scenario_lane();
    let package = gql_ast_performance_scenario_package();
    assert_eq!(package.package_name, "gql-ast");
    let scenario = scenario_by_name(&package, "general-literals-expanded-v1");
    let measurement = measure_asp_rust_scenario(scenario, run_general_literals_expanded_hot_path)
        .expect("measure the expanded ISO general literal AST Scenario");

    assert!(measurement.total_p95 <= Duration::from_millis(2));
    assert!(measurement.observed_memory_bytes <= MEMORY_BUDGET_BYTES);
    let rendered = render_asp_rust_scenario_benchmark_toml(scenario, &measurement)
        .expect("render the measured expanded general literal Scenario receipt");
    assert!(rendered.contains("snapshot = \"gql_ast_general_literals_expanded_v1\""));
    assert!(rendered.contains("observed = 0"));
    eprintln!("measured expanded general literal Scenario:\n{rendered}");
}

#[test]
fn character_sequence_escapes_stay_inside_scenario_budget() {
    let _lane = hold_performance_scenario_lane();
    let package = gql_ast_performance_scenario_package();
    assert_eq!(package.package_name, "gql-ast");
    let scenario = scenario_by_name(&package, "character-sequence-escapes-v1");
    let measurement = measure_asp_rust_scenario(scenario, run_character_sequence_escapes_hot_path)
        .expect("measure the ISO character-sequence AST Scenario");

    assert!(measurement.total_p95 <= Duration::from_millis(2));
    assert!(measurement.observed_memory_bytes <= MEMORY_BUDGET_BYTES);
    let rendered = render_asp_rust_scenario_benchmark_toml(scenario, &measurement)
        .expect("render the measured character-sequence Scenario receipt");
    assert!(rendered.contains("snapshot = \"gql_ast_character_sequence_escapes_v1\""));
    assert!(rendered.contains("observed = 0"));
    eprintln!("measured character-sequence Scenario:\n{rendered}");
}
