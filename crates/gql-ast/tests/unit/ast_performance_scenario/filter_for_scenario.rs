//! ASP Rust Project performance scenario for ISO FILTER and FOR.
#![forbid(unsafe_code)]

use asp_rust_build_support::{AspRustScenario, asp_rust_scenario};

use super::MEMORY_BUDGET_BYTES;

pub const SOURCE: &str = include_str!("../scenarios/filter-for-v1/inputs/filter-for.gql");
pub const SOURCE_BYTES: u64 = SOURCE.len() as u64;

#[must_use]
pub fn filter_for_scenario() -> AspRustScenario {
    asp_rust_scenario! {
        name: "filter-for-v1",
        package: "gql-ast",
        description: "ISO FILTER and FOR lower through lossless Rowan nodes and typed AST bindings without runtime grammar dispatch.",
        fixture_root: "crates/gql-ast/tests/unit/scenarios/filter-for-v1",
        tags: ["gql", "filter", "for", "rowan", "ast", "performance"],
        commands: [
            {
                label: "scenario-budget",
                argv: [
                    "cargo", "test", "-p", "gql-ast",
                    "filter_for_stays_inside_scenario_budget",
                    "--", "--nocapture"
                ]
            },
            {
                label: "criterion",
                argv: ["cargo", "bench", "-p", "gql-ast", "--bench", "ast_perf", "filter-for-v1"]
            }
        ],
        benchmark: {
            harness: "libtest",
            test: "filter_for_stays_inside_scenario_budget",
            snapshot: "gql_ast_filter_for_v1",
            target_total: "400us",
            max_total: "5ms",
            regression_budget: "750us",
            memory_budget_bytes: MEMORY_BUDGET_BYTES,
            target_rationale: "One bounded statement exercises LET, FOR IN, WITH ORDINALITY, FILTER, and RETURN through the parser and AST hot path without semantic resolution, backend work, fallback, or runtime grammar dispatch.",
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
