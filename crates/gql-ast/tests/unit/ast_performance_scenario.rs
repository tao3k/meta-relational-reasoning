use asp_rust_build_support::{
    AspRustScenario, AspRustScenarioPackage, asp_rust_scenario, asp_rust_scenario_package,
};

pub const QUERY_SOURCE: &str =
    include_str!("scenarios/lossless-cst-to-ast-hot-path-v1/inputs/query.gql");
pub const CATALOG_SOURCE: &str =
    include_str!("scenarios/lossless-cst-to-ast-hot-path-v1/inputs/catalog.gql");
pub const MUTATION_SOURCE: &str =
    include_str!("scenarios/lossless-cst-to-ast-hot-path-v1/inputs/mutation.gql");
pub const NESTED_GRAPH_TYPE_SOURCE: &str =
    include_str!("scenarios/lossless-cst-to-ast-hot-path-v1/inputs/nested-graph-type.gql");
pub const VALUE_TYPE_LATTICE_SOURCE: &str =
    include_str!("scenarios/value-type-lattice-v1/inputs/value-types.gql");
pub const REFERENCE_AND_PREDEFINED_SOURCE: &str =
    include_str!("scenarios/reference-and-predefined-types-v1/inputs/reference-and-predefined.gql");
pub const LEXICAL_IDENTIFIERS_SOURCE: &str =
    include_str!("scenarios/lexical-identifiers-v1/inputs/identifiers.gql");
pub const LEXICAL_NUMERICS_SOURCE: &str =
    include_str!("scenarios/lexical-numerics-v1/inputs/numerics.gql");
pub const GENERAL_LITERALS_SOURCE: &str =
    include_str!("scenarios/general-literals-v1/inputs/general-literals.gql");
pub const GENERAL_LITERALS_EXPANDED_SOURCE: &str =
    include_str!("scenarios/general-literals-expanded-v1/inputs/general-literals.gql");
pub const CHARACTER_SEQUENCE_ESCAPES_SOURCE: &str =
    include_str!("scenarios/character-sequence-escapes-v1/inputs/character-sequences.gql");
pub const SOURCES: [&str; 4] = [
    QUERY_SOURCE,
    CATALOG_SOURCE,
    MUTATION_SOURCE,
    NESTED_GRAPH_TYPE_SOURCE,
];
pub const SOURCE_BYTES: u64 = (QUERY_SOURCE.len()
    + CATALOG_SOURCE.len()
    + MUTATION_SOURCE.len()
    + NESTED_GRAPH_TYPE_SOURCE.len()) as u64;
pub const MAX_TOTAL_MILLIS: u64 = 5;
pub const MEMORY_BUDGET_BYTES: u64 = 1_048_576;
pub const VALUE_TYPE_SOURCE_BYTES: u64 = VALUE_TYPE_LATTICE_SOURCE.len() as u64;
pub const REFERENCE_AND_PREDEFINED_SOURCE_BYTES: u64 = REFERENCE_AND_PREDEFINED_SOURCE.len() as u64;
pub const LEXICAL_IDENTIFIERS_SOURCE_BYTES: u64 = LEXICAL_IDENTIFIERS_SOURCE.len() as u64;
pub const LEXICAL_NUMERICS_SOURCE_BYTES: u64 = LEXICAL_NUMERICS_SOURCE.len() as u64;
pub const GENERAL_LITERALS_SOURCE_BYTES: u64 = GENERAL_LITERALS_SOURCE.len() as u64;
pub const GENERAL_LITERALS_EXPANDED_SOURCE_BYTES: u64 =
    GENERAL_LITERALS_EXPANDED_SOURCE.len() as u64;
pub const CHARACTER_SEQUENCE_ESCAPES_SOURCE_BYTES: u64 =
    CHARACTER_SEQUENCE_ESCAPES_SOURCE.len() as u64;

#[must_use]
pub fn lossless_cst_to_ast_hot_path_scenario() -> AspRustScenario {
    asp_rust_scenario! {
        name: "lossless-cst-to-ast-hot-path-v1",
        package: "gql-ast",
        description: "Lossless Rowan parsing and backend-neutral AST lowering stay inside one bounded frontend hot path.",
        fixture_root: "crates/gql-ast/tests/unit/scenarios/lossless-cst-to-ast-hot-path-v1",
        tags: ["gql", "parser", "rowan", "ast", "performance", "hot-path"],
        commands: [
            {
                label: "performance-contract",
                argv: [
                    "cargo",
                    "test",
                    "-p",
                    "gql-ast",
                    "lossless_cst_to_ast_hot_path_stays_inside_scenario_budget",
                    "--",
                    "--nocapture",
                ]
            },
            {
                label: "criterion-benchmark",
                argv: ["cargo", "bench", "-p", "gql-ast", "--bench", "ast_perf"]
            },
        ],
        benchmark: {
            harness: "libtest",
            test: "lossless_cst_to_ast_hot_path_stays_inside_scenario_budget",
            snapshot: "gql_ast_lossless_cst_to_ast_hot_path_v1",
            target_total: "500us",
            max_total: "5ms",
            regression_budget: "1ms",
            memory_budget_bytes: MEMORY_BUDGET_BYTES,
            target_rationale: "The frontend parses and lowers four representative profile-v1 statements, including a qualified nested graph type, without semantic analysis, backend work, or fallback.",
            warmup_iterations: 3,
            measure_iterations: 25,
            metrics: [
                { name: "source_bytes", unit: "bytes", kind: Exact, target: SOURCE_BYTES },
                { name: "statement_count", unit: "count", kind: Exact, target: 4 },
                { name: "diagnostic_count", unit: "count", kind: Maximum, target: 0 },
                { name: "fallback_count", unit: "count", kind: Exact, target: 0 },
                { name: "roundtrip_bytes", unit: "bytes", kind: Exact, target: SOURCE_BYTES }
            ]
        }
    }
}

#[must_use]
pub fn value_type_lattice_scenario() -> AspRustScenario {
    asp_rust_scenario! {
        name: "value-type-lattice-v1",
        package: "gql-ast",
        description: "Recursive ISO GQL declaration types remain lossless and bounded through Rowan parsing and AST lowering.",
        fixture_root: "crates/gql-ast/tests/unit/scenarios/value-type-lattice-v1",
        tags: ["gql", "parser", "rowan", "ast", "value-type", "performance"],
        commands: [
            {
                label: "performance-contract",
                argv: [
                    "cargo",
                    "test",
                    "-p",
                    "gql-ast",
                    "value_type_lattice_stays_inside_scenario_budget",
                    "--",
                    "--nocapture",
                ]
            },
            {
                label: "criterion-benchmark",
                argv: ["cargo", "bench", "-p", "gql-ast", "--bench", "ast_perf", "value-type-lattice-v1"]
            },
        ],
        benchmark: {
            harness: "libtest",
            test: "value_type_lattice_stays_inside_scenario_budget",
            snapshot: "gql_ast_value_type_lattice_v1",
            target_total: "500us",
            max_total: "5ms",
            regression_budget: "1ms",
            memory_budget_bytes: MEMORY_BUDGET_BYTES,
            target_rationale: "The frontend parses and lowers one recursive scalar, list, record, union, and reference-type declaration without semantic or backend work.",
            warmup_iterations: 3,
            measure_iterations: 25,
            metrics: [
                { name: "source_bytes", unit: "bytes", kind: Exact, target: VALUE_TYPE_SOURCE_BYTES },
                { name: "statement_count", unit: "count", kind: Exact, target: 1 },
                { name: "diagnostic_count", unit: "count", kind: Maximum, target: 0 },
                { name: "fallback_count", unit: "count", kind: Exact, target: 0 },
                { name: "roundtrip_bytes", unit: "bytes", kind: Exact, target: VALUE_TYPE_SOURCE_BYTES }
            ]
        }
    }
}

#[must_use]
pub fn reference_and_predefined_types_scenario() -> AspRustScenario {
    asp_rust_scenario! {
        name: "reference-and-predefined-types-v1",
        package: "gql-ast",
        description: "Closed reference descriptors and representative predefined aliases remain lossless and bounded through Rowan and AST lowering.",
        fixture_root: "crates/gql-ast/tests/unit/scenarios/reference-and-predefined-types-v1",
        tags: ["gql", "parser", "rowan", "ast", "reference-type", "predefined-type", "performance"],
        commands: [
            {
                label: "performance-contract",
                argv: [
                    "cargo",
                    "test",
                    "-p",
                    "gql-ast",
                    "reference_and_predefined_types_stay_inside_scenario_budget",
                    "--",
                    "--nocapture",
                ]
            },
            {
                label: "criterion-benchmark",
                argv: ["cargo", "bench", "-p", "gql-ast", "--bench", "ast_perf", "reference-and-predefined-types-v1"]
            },
        ],
        benchmark: {
            harness: "libtest",
            test: "reference_and_predefined_types_stay_inside_scenario_budget",
            snapshot: "gql_ast_reference_and_predefined_types_v1",
            target_total: "500us",
            max_total: "5ms",
            regression_budget: "1ms",
            memory_budget_bytes: MEMORY_BUDGET_BYTES,
            target_rationale: "One source exercises recursive closed graph, node and edge descriptors plus canonical predefined aliases without semantic or backend work.",
            warmup_iterations: 3,
            measure_iterations: 25,
            metrics: [
                { name: "source_bytes", unit: "bytes", kind: Exact, target: REFERENCE_AND_PREDEFINED_SOURCE_BYTES },
                { name: "statement_count", unit: "count", kind: Exact, target: 1 },
                { name: "diagnostic_count", unit: "count", kind: Maximum, target: 0 },
                { name: "fallback_count", unit: "count", kind: Exact, target: 0 },
                { name: "roundtrip_bytes", unit: "bytes", kind: Exact, target: REFERENCE_AND_PREDEFINED_SOURCE_BYTES }
            ]
        }
    }
}

#[must_use]
pub fn lexical_identifiers_scenario() -> AspRustScenario {
    asp_rust_scenario! {
        name: "lexical-identifiers-v1",
        package: "gql-ast",
        description: "Exact ISO non-reserved words, Unicode identifier categories, and delimited forms stay lossless and bounded through Rowan and AST lowering.",
        fixture_root: "crates/gql-ast/tests/unit/scenarios/lexical-identifiers-v1",
        tags: ["gql", "lexer", "unicode", "identifier", "rowan", "ast", "performance"],
        commands: [
            {
                label: "performance-contract",
                argv: [
                    "cargo",
                    "test",
                    "-p",
                    "gql-ast",
                    "lexical_identifiers_stay_inside_scenario_budget",
                    "--",
                    "--nocapture",
                ]
            },
            {
                label: "criterion-benchmark",
                argv: ["cargo", "bench", "-p", "gql-ast", "--bench", "ast_perf", "lexical-identifiers-v1"]
            },
        ],
        benchmark: {
            harness: "libtest",
            test: "lexical_identifiers_stay_inside_scenario_budget",
            snapshot: "gql_ast_lexical_identifiers_v1",
            target_total: "500us",
            max_total: "5ms",
            regression_budget: "1ms",
            memory_budget_bytes: MEMORY_BUDGET_BYTES,
            target_rationale: "One source exercises the complete non-reserved set and representative Unicode and delimited forms without semantic or backend work.",
            warmup_iterations: 3,
            measure_iterations: 25,
            metrics: [
                { name: "source_bytes", unit: "bytes", kind: Exact, target: LEXICAL_IDENTIFIERS_SOURCE_BYTES },
                { name: "statement_count", unit: "count", kind: Exact, target: 1 },
                { name: "diagnostic_count", unit: "count", kind: Maximum, target: 0 },
                { name: "fallback_count", unit: "count", kind: Exact, target: 0 },
                { name: "roundtrip_bytes", unit: "bytes", kind: Exact, target: LEXICAL_IDENTIFIERS_SOURCE_BYTES }
            ]
        }
    }
}

#[must_use]
pub fn lexical_numerics_scenario() -> AspRustScenario {
    asp_rust_scenario! {
        name: "lexical-numerics-v1",
        package: "gql-ast",
        description: "Every Gerbil-owned exact and approximate ISO numeric production remains lossless and bounded through Rowan and AST lowering.",
        fixture_root: "crates/gql-ast/tests/unit/scenarios/lexical-numerics-v1",
        tags: ["gql", "lexer", "numeric", "rowan", "ast", "performance"],
        commands: [
            {
                label: "performance-contract",
                argv: [
                    "cargo",
                    "test",
                    "-p",
                    "gql-ast",
                    "lexical_numerics_stay_inside_scenario_budget",
                    "--",
                    "--nocapture",
                ]
            },
            {
                label: "criterion-benchmark",
                argv: ["cargo", "bench", "-p", "gql-ast", "--bench", "ast_perf", "lexical-numerics-v1"]
            },
        ],
        benchmark: {
            harness: "libtest",
            test: "lexical_numerics_stay_inside_scenario_budget",
            snapshot: "gql_ast_lexical_numerics_v1",
            target_total: "250us",
            max_total: "2ms",
            regression_budget: "500us",
            memory_budget_bytes: MEMORY_BUDGET_BYTES,
            target_rationale: "One ASCII source exercises all numeric forms without semantic analysis, backend work, fallback, or runtime grammar dispatch.",
            warmup_iterations: 3,
            measure_iterations: 25,
            metrics: [
                { name: "source_bytes", unit: "bytes", kind: Exact, target: LEXICAL_NUMERICS_SOURCE_BYTES },
                { name: "statement_count", unit: "count", kind: Exact, target: 1 },
                { name: "diagnostic_count", unit: "count", kind: Maximum, target: 0 },
                { name: "fallback_count", unit: "count", kind: Exact, target: 0 },
                { name: "roundtrip_bytes", unit: "bytes", kind: Exact, target: LEXICAL_NUMERICS_SOURCE_BYTES }
            ]
        }
    }
}

#[must_use]
pub fn general_literals_scenario() -> AspRustScenario {
    asp_rust_scenario! {
        name: "general-literals-v1",
        package: "gql-ast",
        description: "Contextual character, byte, temporal, duration, nested-list, and ordered-record literals remain lossless and bounded through Rowan and AST lowering.",
        fixture_root: "crates/gql-ast/tests/unit/scenarios/general-literals-v1",
        tags: ["gql", "lexer", "literal", "rowan", "ast", "performance"],
        commands: [
            {
                label: "performance-contract",
                argv: [
                    "cargo",
                    "test",
                    "-p",
                    "gql-ast",
                    "general_literals_stay_inside_scenario_budget",
                    "--",
                    "--nocapture",
                ]
            },
            {
                label: "criterion-benchmark",
                argv: ["cargo", "bench", "-p", "gql-ast", "--bench", "ast_perf", "general-literals-v1"]
            },
        ],
        benchmark: {
            harness: "libtest",
            test: "general_literals_stay_inside_scenario_budget",
            snapshot: "gql_ast_general_literals_v1",
            target_total: "250us",
            max_total: "2ms",
            regression_budget: "500us",
            memory_budget_bytes: MEMORY_BUDGET_BYTES,
            target_rationale: "One bounded source exercises every admitted general-literal family without semantic analysis, backend work, fallback, or runtime grammar dispatch.",
            warmup_iterations: 3,
            measure_iterations: 25,
            metrics: [
                { name: "source_bytes", unit: "bytes", kind: Exact, target: GENERAL_LITERALS_SOURCE_BYTES },
                { name: "statement_count", unit: "count", kind: Exact, target: 1 },
                { name: "diagnostic_count", unit: "count", kind: Maximum, target: 0 },
                { name: "fallback_count", unit: "count", kind: Exact, target: 0 },
                { name: "roundtrip_bytes", unit: "bytes", kind: Exact, target: GENERAL_LITERALS_SOURCE_BYTES }
            ]
        }
    }
}

#[must_use]
pub fn general_literals_expanded_scenario() -> AspRustScenario {
    asp_rust_scenario! {
        name: "general-literals-expanded-v1",
        package: "gql-ast",
        description: "Temporal suffixes, DATETIME, nested lists, and both record spellings remain lossless and bounded through Rowan and AST lowering.",
        fixture_root: "crates/gql-ast/tests/unit/scenarios/general-literals-expanded-v1",
        tags: ["gql", "lexer", "literal", "rowan", "ast", "performance"],
        commands: [
            {
                label: "performance-contract",
                argv: [
                    "cargo",
                    "test",
                    "-p",
                    "gql-ast",
                    "general_literals_expanded_stay_inside_scenario_budget",
                    "--",
                    "--nocapture",
                ]
            },
            {
                label: "criterion-benchmark",
                argv: ["cargo", "bench", "-p", "gql-ast", "--bench", "ast_perf", "general-literals-expanded-v1"]
            },
        ],
        benchmark: {
            harness: "libtest",
            test: "general_literals_expanded_stay_inside_scenario_budget",
            snapshot: "gql_ast_general_literals_expanded_v1",
            target_total: "250us",
            max_total: "2ms",
            regression_budget: "500us",
            memory_budget_bytes: MEMORY_BUDGET_BYTES,
            target_rationale: "One bounded expanded source exercises temporal suffixes, DATETIME, and both record spellings without semantic analysis, backend work, fallback, or runtime grammar dispatch.",
            warmup_iterations: 3,
            measure_iterations: 25,
            metrics: [
                { name: "source_bytes", unit: "bytes", kind: Exact, target: GENERAL_LITERALS_EXPANDED_SOURCE_BYTES },
                { name: "statement_count", unit: "count", kind: Exact, target: 1 },
                { name: "diagnostic_count", unit: "count", kind: Maximum, target: 0 },
                { name: "fallback_count", unit: "count", kind: Exact, target: 0 },
                { name: "roundtrip_bytes", unit: "bytes", kind: Exact, target: GENERAL_LITERALS_EXPANDED_SOURCE_BYTES }
            ]
        }
    }
}

#[must_use]
pub fn character_sequence_escapes_scenario() -> AspRustScenario {
    asp_rust_scenario! {
        name: "character-sequence-escapes-v1",
        package: "gql-ast",
        description: "ISO GQL single, double, no-escape, control, delimiter, and Unicode character representations remain lossless and bounded through Rowan and AST lowering.",
        fixture_root: "crates/gql-ast/tests/unit/scenarios/character-sequence-escapes-v1",
        tags: ["gql", "lexer", "character-string", "escape", "rowan", "ast", "performance"],
        commands: [
            {
                label: "performance-contract",
                argv: [
                    "cargo",
                    "test",
                    "-p",
                    "gql-ast",
                    "character_sequence_escapes_stay_inside_scenario_budget",
                    "--",
                    "--nocapture",
                ]
            },
            {
                label: "criterion-benchmark",
                argv: ["cargo", "bench", "-p", "gql-ast", "--bench", "ast_perf", "character-sequence-escapes-v1"]
            },
        ],
        benchmark: {
            harness: "libtest",
            test: "character_sequence_escapes_stay_inside_scenario_budget",
            snapshot: "gql_ast_character_sequence_escapes_v1",
            target_total: "250us",
            max_total: "2ms",
            regression_budget: "500us",
            memory_budget_bytes: MEMORY_BUDGET_BYTES,
            target_rationale: "One bounded source exercises the complete admitted escape family, both ISO string delimiters, NO_ESCAPE, and temporal reuse without backend work, fallback, or runtime grammar dispatch.",
            warmup_iterations: 3,
            measure_iterations: 25,
            metrics: [
                { name: "source_bytes", unit: "bytes", kind: Exact, target: CHARACTER_SEQUENCE_ESCAPES_SOURCE_BYTES },
                { name: "statement_count", unit: "count", kind: Exact, target: 1 },
                { name: "diagnostic_count", unit: "count", kind: Maximum, target: 0 },
                { name: "fallback_count", unit: "count", kind: Exact, target: 0 },
                { name: "roundtrip_bytes", unit: "bytes", kind: Exact, target: CHARACTER_SEQUENCE_ESCAPES_SOURCE_BYTES }
            ]
        }
    }
}

#[must_use]
pub fn gql_ast_performance_scenario_package() -> AspRustScenarioPackage {
    asp_rust_scenario_package! {
        package: "gql-ast",
        scenarios: [
            lossless_cst_to_ast_hot_path_scenario(),
            value_type_lattice_scenario(),
            reference_and_predefined_types_scenario(),
            lexical_identifiers_scenario(),
            lexical_numerics_scenario(),
            general_literals_scenario(),
            general_literals_expanded_scenario(),
            character_sequence_escapes_scenario()
        ],
    }
}
