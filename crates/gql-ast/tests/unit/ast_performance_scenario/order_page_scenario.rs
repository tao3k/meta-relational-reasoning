//! ASP Rust Project performance scenario for ISO ordering and pagination.
#![forbid(unsafe_code)]

use asp_rust_build_support::{AspRustScenario, asp_rust_scenario};

use super::MEMORY_BUDGET_BYTES;

pub const SOURCE: &str = include_str!("../scenarios/order-page-v1/inputs/order-page.gql");
pub const SOURCE_BYTES: u64 = SOURCE.len() as u64;

#[must_use]
pub fn order_page_scenario() -> AspRustScenario {
    asp_rust_scenario! {
        name: "order-page-v1",
        package: "gql-ast",
        description: "ISO ordering directions, null placement, OFFSET/SKIP synonyms, zero limits, and dynamic page parameters lower through typed Rowan and AST nodes.",
        fixture_root: "crates/gql-ast/tests/unit/scenarios/order-page-v1",
        tags: ["gql", "order-by", "pagination", "rowan", "ast", "performance"],
        commands: [
            {
                label: "scenario-budget",
                argv: [
                    "cargo", "test", "-p", "gql-ast",
                    "order_page_stays_inside_scenario_budget",
                    "--", "--nocapture"
                ]
            },
            {
                label: "criterion",
                argv: ["cargo", "bench", "-p", "gql-ast", "--bench", "ast_perf", "order-page-v1"]
            }
        ],
        benchmark: {
            harness: "libtest",
            test: "order_page_stays_inside_scenario_budget",
            snapshot: "gql_ast_order_page_v1",
            target_total: "400us",
            max_total: "5ms",
            regression_budget: "750us",
            memory_budget_bytes: MEMORY_BUDGET_BYTES,
            target_rationale: "One bounded statement exercises two sort specifications, both explicit direction forms, both null placements, SKIP, a dynamic page parameter, and zero LIMIT without semantic resolution, backend work, fallback, or runtime grammar dispatch.",
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
