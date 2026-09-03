use std::hint::black_box;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use asp_rust_build_support::{
    AspRustScenario, AspRustScenarioObservation, AspRustScenarioPackage, measure_asp_rust_scenario,
    render_asp_rust_scenario_benchmark_toml,
};

use super::ast_performance_scenario::{
    CHARACTER_SEQUENCE_ESCAPES_SOURCE, CHARACTER_SEQUENCE_ESCAPES_SOURCE_BYTES,
    DYNAMIC_PARAMETERS_SOURCE, DYNAMIC_PARAMETERS_SOURCE_BYTES, FILTER_FOR_SOURCE,
    GENERAL_LITERALS_EXPANDED_SOURCE, GENERAL_LITERALS_EXPANDED_SOURCE_BYTES,
    GENERAL_LITERALS_SOURCE, GENERAL_LITERALS_SOURCE_BYTES, GRAPH_ELEMENT_PREDICATES_SOURCE,
    ISO_AGGREGATE_FUNCTIONS_SOURCE, ISO_AGGREGATE_FUNCTIONS_SOURCE_BYTES,
    LEXICAL_IDENTIFIERS_SOURCE, LEXICAL_IDENTIFIERS_SOURCE_BYTES, LEXICAL_NUMERICS_SOURCE,
    LEXICAL_NUMERICS_SOURCE_BYTES, MAX_TOTAL_MILLIS, MEMORY_BUDGET_BYTES, ORDER_PAGE_SOURCE,
    PATH_SEARCH_PREFIXES_SOURCE, PRIMITIVE_RESULT_SOURCE, REFERENCE_AND_PREDEFINED_SOURCE,
    REFERENCE_AND_PREDEFINED_SOURCE_BYTES, SOURCE_BYTES, SOURCES, TRUTH_NULL_PREDICATES_SOURCE,
    TRUTH_NULL_PREDICATES_SOURCE_BYTES, VALUE_TYPE_LATTICE_SOURCE, VALUE_TYPE_PREDICATES_SOURCE,
    VALUE_TYPE_PREDICATES_SOURCE_BYTES, VALUE_TYPE_SOURCE_BYTES,
    gql_ast_performance_scenario_package,
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

fn run_dynamic_parameters_hot_path() -> AspRustScenarioObservation {
    run_single_source_hot_path(
        "dynamic-parameters.gql",
        DYNAMIC_PARAMETERS_SOURCE,
        DYNAMIC_PARAMETERS_SOURCE_BYTES,
    )
}

fn run_truth_null_predicates_hot_path() -> AspRustScenarioObservation {
    run_single_source_hot_path(
        "predicates.gql",
        TRUTH_NULL_PREDICATES_SOURCE,
        TRUTH_NULL_PREDICATES_SOURCE_BYTES,
    )
}

fn run_iso_aggregate_functions_hot_path() -> AspRustScenarioObservation {
    run_single_source_hot_path(
        "aggregates.gql",
        ISO_AGGREGATE_FUNCTIONS_SOURCE,
        ISO_AGGREGATE_FUNCTIONS_SOURCE_BYTES,
    )
}

fn run_value_type_predicates_hot_path() -> AspRustScenarioObservation {
    run_single_source_hot_path(
        "value-type-predicates.gql",
        VALUE_TYPE_PREDICATES_SOURCE,
        VALUE_TYPE_PREDICATES_SOURCE_BYTES,
    )
}

fn run_graph_element_predicates_hot_path() -> AspRustScenarioObservation {
    run_single_source_hot_path(
        "graph-element-predicates.gql",
        GRAPH_ELEMENT_PREDICATES_SOURCE,
        GRAPH_ELEMENT_PREDICATES_SOURCE.len() as u64,
    )
}

fn run_filter_for_hot_path() -> AspRustScenarioObservation {
    run_single_source_hot_path(
        "filter-for.gql",
        FILTER_FOR_SOURCE,
        FILTER_FOR_SOURCE.len() as u64,
    )
}

fn run_primitive_result_hot_path() -> AspRustScenarioObservation {
    run_single_source_hot_path(
        "primitive-result.gql",
        PRIMITIVE_RESULT_SOURCE,
        PRIMITIVE_RESULT_SOURCE.len() as u64,
    )
}

fn run_path_search_prefixes_hot_path() -> AspRustScenarioObservation {
    run_single_source_hot_path(
        "path-search-prefixes.gql",
        PATH_SEARCH_PREFIXES_SOURCE,
        PATH_SEARCH_PREFIXES_SOURCE.len() as u64,
    )
}

fn run_order_page_hot_path() -> AspRustScenarioObservation {
    run_single_source_hot_path(
        "order-page.gql",
        ORDER_PAGE_SOURCE,
        ORDER_PAGE_SOURCE.len() as u64,
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

#[test]
fn dynamic_parameters_stay_inside_scenario_budget() {
    let _lane = hold_performance_scenario_lane();
    let package = gql_ast_performance_scenario_package();
    assert_eq!(package.package_name, "gql-ast");
    let scenario = scenario_by_name(&package, "dynamic-parameter-specification-v1");
    let measurement = measure_asp_rust_scenario(scenario, run_dynamic_parameters_hot_path)
        .expect("measure the ISO dynamic-parameter AST Scenario");

    assert!(measurement.total_p95 <= Duration::from_millis(2));
    assert!(measurement.observed_memory_bytes <= MEMORY_BUDGET_BYTES);
    let rendered = render_asp_rust_scenario_benchmark_toml(scenario, &measurement)
        .expect("render the measured dynamic-parameter Scenario receipt");
    assert!(rendered.contains("snapshot = \"gql_ast_dynamic_parameter_specification_v1\""));
    assert!(rendered.contains("observed = 0"));
    eprintln!("measured dynamic-parameter Scenario:\n{rendered}");
}

#[test]
fn truth_null_predicates_stay_inside_scenario_budget() {
    let _lane = hold_performance_scenario_lane();
    let package = gql_ast_performance_scenario_package();
    assert_eq!(package.package_name, "gql-ast");
    let scenario = scenario_by_name(&package, "truth-null-predicates-v1");
    let measurement = measure_asp_rust_scenario(scenario, run_truth_null_predicates_hot_path)
        .expect("measure the ISO truth/null predicate AST Scenario");

    assert!(measurement.total_p95 <= Duration::from_millis(2));
    assert!(measurement.observed_memory_bytes <= MEMORY_BUDGET_BYTES);
    let rendered = render_asp_rust_scenario_benchmark_toml(scenario, &measurement)
        .expect("render the measured truth/null predicate Scenario receipt");
    assert!(rendered.contains("snapshot = \"gql_ast_truth_null_predicates_v1\""));
    assert!(rendered.contains("observed = 0"));
    eprintln!("measured truth/null predicate Scenario:\n{rendered}");
}

#[test]
fn iso_aggregate_functions_stay_inside_scenario_budget() {
    let _lane = hold_performance_scenario_lane();
    let package = gql_ast_performance_scenario_package();
    assert_eq!(package.package_name, "gql-ast");
    let scenario = scenario_by_name(&package, "iso-aggregate-functions-v1");
    let measurement = measure_asp_rust_scenario(scenario, run_iso_aggregate_functions_hot_path)
        .expect("measure the ISO aggregate function AST Scenario");

    assert!(measurement.total_p95 <= Duration::from_millis(2));
    assert!(measurement.observed_memory_bytes <= MEMORY_BUDGET_BYTES);
    let rendered = render_asp_rust_scenario_benchmark_toml(scenario, &measurement)
        .expect("render the measured aggregate function Scenario receipt");
    assert!(rendered.contains("snapshot = \"gql_ast_iso_aggregate_functions_v1\""));
    assert!(rendered.contains("observed = 0"));
    eprintln!("measured aggregate function Scenario:\n{rendered}");
}

#[test]
fn value_type_predicates_stay_inside_scenario_budget() {
    let _lane = hold_performance_scenario_lane();
    let package = gql_ast_performance_scenario_package();
    assert_eq!(package.package_name, "gql-ast");
    let scenario = scenario_by_name(&package, "value-type-predicates-v1");
    let measurement = measure_asp_rust_scenario(scenario, run_value_type_predicates_hot_path)
        .expect("measure the ISO value-type predicate AST Scenario");

    assert!(measurement.total_p95 <= Duration::from_millis(2));
    assert!(measurement.observed_memory_bytes <= MEMORY_BUDGET_BYTES);
    let rendered = render_asp_rust_scenario_benchmark_toml(scenario, &measurement)
        .expect("render the measured value-type predicate Scenario receipt");
    assert!(rendered.contains("snapshot = \"gql_ast_value_type_predicates_v1\""));
    assert!(rendered.contains("observed = 0"));
    eprintln!("measured value-type predicate Scenario:\n{rendered}");
}

#[test]
fn graph_element_predicates_stay_inside_scenario_budget() {
    let _lane = hold_performance_scenario_lane();
    let package = gql_ast_performance_scenario_package();
    assert_eq!(package.package_name, "gql-ast");
    let scenario = scenario_by_name(&package, "graph-element-predicates-v1");
    let measurement = measure_asp_rust_scenario(scenario, run_graph_element_predicates_hot_path)
        .expect("measure the ISO graph-element predicate AST Scenario");

    assert!(
        measurement.total_p95 <= Duration::from_millis(MAX_TOTAL_MILLIS),
        "graph-element predicate p95 {:?} exceeded the scenario hard ceiling",
        measurement.total_p95
    );
    assert!(measurement.observed_memory_bytes <= MEMORY_BUDGET_BYTES);
    let rendered = render_asp_rust_scenario_benchmark_toml(scenario, &measurement)
        .expect("render the measured graph-element predicate Scenario receipt");
    assert!(rendered.contains("snapshot = \"gql_ast_graph_element_predicates_v1\""));
    assert!(rendered.contains("observed = 0"));
    eprintln!("measured graph-element predicate Scenario:\n{rendered}");
}

#[test]
fn filter_for_stays_inside_scenario_budget() {
    let _lane = hold_performance_scenario_lane();
    let package = gql_ast_performance_scenario_package();
    let scenario = scenario_by_name(&package, "filter-for-v1");
    let measurement = measure_asp_rust_scenario(scenario, run_filter_for_hot_path)
        .expect("measure the ISO FILTER and FOR AST Scenario");

    assert!(
        measurement.total_p95 <= Duration::from_millis(MAX_TOTAL_MILLIS),
        "FILTER/FOR p95 {:?} exceeded the scenario hard ceiling",
        measurement.total_p95
    );
    assert!(measurement.observed_memory_bytes <= MEMORY_BUDGET_BYTES);
    let rendered = render_asp_rust_scenario_benchmark_toml(scenario, &measurement)
        .expect("render the measured FILTER/FOR Scenario receipt");
    assert!(rendered.contains("snapshot = \"gql_ast_filter_for_v1\""));
    assert!(rendered.contains("observed = 0"));
    eprintln!("measured FILTER/FOR Scenario:\n{rendered}");
}

#[test]
fn primitive_result_stays_inside_scenario_budget() {
    let _lane = hold_performance_scenario_lane();
    let package = gql_ast_performance_scenario_package();
    let scenario = scenario_by_name(&package, "primitive-result-v1");
    let measurement = measure_asp_rust_scenario(scenario, run_primitive_result_hot_path)
        .expect("measure the ISO primitive result AST Scenario");

    assert!(
        measurement.total_p95 <= Duration::from_millis(MAX_TOTAL_MILLIS),
        "primitive result p95 {:?} exceeded the scenario hard ceiling",
        measurement.total_p95
    );
    assert!(measurement.observed_memory_bytes <= MEMORY_BUDGET_BYTES);
    let rendered = render_asp_rust_scenario_benchmark_toml(scenario, &measurement)
        .expect("render the measured primitive result Scenario receipt");
    assert!(rendered.contains("snapshot = \"gql_ast_primitive_result_v1\""));
    assert!(rendered.contains("observed = 0"));
    eprintln!("measured primitive result Scenario:\n{rendered}");
}

#[test]
fn path_search_prefixes_stay_inside_scenario_budget() {
    let _lane = hold_performance_scenario_lane();
    let package = gql_ast_performance_scenario_package();
    let scenario = scenario_by_name(&package, "path-search-prefixes-v1");
    let measurement = measure_asp_rust_scenario(scenario, run_path_search_prefixes_hot_path)
        .expect("measure the ISO path-search prefix AST Scenario");

    assert!(
        measurement.total_p95 <= Duration::from_millis(MAX_TOTAL_MILLIS),
        "path-search prefix p95 {:?} exceeded the scenario hard ceiling",
        measurement.total_p95
    );
    assert!(measurement.observed_memory_bytes <= MEMORY_BUDGET_BYTES);
    let rendered = render_asp_rust_scenario_benchmark_toml(scenario, &measurement)
        .expect("render the measured path-search prefix Scenario receipt");
    assert!(rendered.contains("snapshot = \"gql_ast_path_search_prefixes_v1\""));
    assert!(rendered.contains("observed = 0"));
    eprintln!("measured path-search prefix Scenario:\n{rendered}");
}

#[test]
fn order_page_stays_inside_scenario_budget() {
    let _lane = hold_performance_scenario_lane();
    let package = gql_ast_performance_scenario_package();
    let scenario = scenario_by_name(&package, "order-page-v1");
    let measurement = measure_asp_rust_scenario(scenario, run_order_page_hot_path)
        .expect("measure the ISO ordering and pagination AST Scenario");

    assert!(
        measurement.total_p95 <= Duration::from_millis(MAX_TOTAL_MILLIS),
        "ordering and pagination p95 {:?} exceeded the scenario hard ceiling",
        measurement.total_p95
    );
    assert!(measurement.observed_memory_bytes <= MEMORY_BUDGET_BYTES);
    let rendered = render_asp_rust_scenario_benchmark_toml(scenario, &measurement)
        .expect("render the measured ordering and pagination Scenario receipt");
    assert!(rendered.contains("snapshot = \"gql_ast_order_page_v1\""));
    assert!(rendered.contains("observed = 0"));
    eprintln!("measured ordering and pagination Scenario:\n{rendered}");
}
