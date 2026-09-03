//! ASP Rust Project performance scenario for ISO primitive result statements.
#![forbid(unsafe_code)]

use asp_rust_build_support::{AspRustScenario, asp_rust_scenario};

use super::MEMORY_BUDGET_BYTES;

pub const SOURCE: &str =
    include_str!("../scenarios/primitive-result-v1/inputs/primitive-result.gql");
pub const SOURCE_BYTES: u64 = SOURCE.len() as u64;

#[must_use]
pub fn primitive_result_scenario() -> AspRustScenario {
    asp_rust_scenario! {
        name: "primitive-result-v1",
        package: "gql-ast",
        description: "ISO RETURN set quantifiers and wildcard projection lower through lossless Rowan and typed AST nodes without runtime grammar dispatch.",
        fixture_root: "crates/gql-ast/tests/unit/scenarios/primitive-result-v1",
        tags: ["gql", "return", "finish", "rowan", "ast", "performance"],
        commands: [
            {
                label: "scenario-budget",
                argv: [
                    "cargo", "test", "-p", "gql-ast",
                    "primitive_result_stays_inside_scenario_budget",
                    "--", "--nocapture"
                ]
            },
            {
                label: "criterion",
                argv: ["cargo", "bench", "-p", "gql-ast", "--bench", "ast_perf", "primitive-result-v1"]
            }
        ],
        benchmark: {
            harness: "libtest",
            test: "primitive_result_stays_inside_scenario_budget",
            snapshot: "gql_ast_primitive_result_v1",
            target_total: "300us",
            max_total: "5ms",
            regression_budget: "600us",
            memory_budget_bytes: MEMORY_BUDGET_BYTES,
            target_rationale: "One bounded graph pattern exercises source-ordered bindings, a result set quantifier, and wildcard projection through parser and AST lowering without semantic resolution, backend work, fallback, or runtime grammar dispatch.",
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
