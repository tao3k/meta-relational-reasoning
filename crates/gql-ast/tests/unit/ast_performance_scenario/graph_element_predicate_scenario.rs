//! ASP Rust Project performance scenario for the ISO graph-element predicate family.
#![forbid(unsafe_code)]

use asp_rust_build_support::{AspRustScenario, asp_rust_scenario};

use super::MEMORY_BUDGET_BYTES;

pub const SOURCE: &str =
    include_str!("../scenarios/graph-element-predicates-v1/inputs/predicates.gql");
pub const SOURCE_BYTES: u64 = SOURCE.len() as u64;

#[must_use]
pub fn graph_element_predicates_scenario() -> AspRustScenario {
    asp_rust_scenario! {
        name: "graph-element-predicates-v1",
        package: "gql-ast",
        description: "The complete ISO GQL graph-element predicate family lowers through typed Rowan and AST nodes.",
        fixture_root: "crates/gql-ast/tests/unit/scenarios/graph-element-predicates-v1",
        tags: ["gql", "predicate", "graph-element", "rowan", "ast", "performance"],
        commands: [
            {
                label: "scenario-budget",
                argv: [
                    "cargo", "test", "-p", "gql-ast",
                    "graph_element_predicates_stay_inside_scenario_budget",
                    "--", "--nocapture"
                ]
            },
            {
                label: "criterion",
                argv: ["cargo", "bench", "-p", "gql-ast", "--bench", "ast_perf", "graph-element-predicates-v1"]
            }
        ],
        benchmark: {
            harness: "libtest",
            test: "graph_element_predicates_stay_inside_scenario_budget",
            snapshot: "gql_ast_graph_element_predicates_v1",
            target_total: "300us",
            max_total: "5ms",
            regression_budget: "500us",
            memory_budget_bytes: MEMORY_BUDGET_BYTES,
            target_rationale: "One bounded statement exercises directed, endpoint, n-ary identity, and property-existence predicates without semantic resolution, backend work, fallback, or runtime grammar dispatch.",
            warmup_iterations: 3,
            measure_iterations: 25,
            metrics: [
                { name: "source_bytes", unit: "bytes", kind: Exact, target: SOURCE_BYTES },
                { name: "statement_count", unit: "count", kind: Exact, target: 1 },
                { name: "diagnostic_count", unit: "count", kind: Maximum, target: 0 },
                { name: "fallback_count", unit: "count", kind: Exact, target: 0 },
                { name: "roundtrip_bytes", unit: "bytes", kind: Exact, target: SOURCE_BYTES }
            ]
        }
    }
}
