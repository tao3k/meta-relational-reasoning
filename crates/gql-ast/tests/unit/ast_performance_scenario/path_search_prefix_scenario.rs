//! ASP Rust Project performance scenario for ISO graph match and path search prefixes.
#![forbid(unsafe_code)]

use asp_rust_build_support::{AspRustScenario, asp_rust_scenario};

use super::MEMORY_BUDGET_BYTES;

pub const SOURCE: &str =
    include_str!("../scenarios/path-search-prefixes-v1/inputs/path-prefixes.gql");
pub const SOURCE_BYTES: u64 = SOURCE.len() as u64;

#[must_use]
pub fn path_search_prefixes_scenario() -> AspRustScenario {
    asp_rust_scenario! {
        name: "path-search-prefixes-v1",
        package: "gql-ast",
        description: "ISO graph match modes, per-path search prefixes, uniqueness modes, targets, and KEEP lower through typed Rowan and AST nodes.",
        fixture_root: "crates/gql-ast/tests/unit/scenarios/path-search-prefixes-v1",
        tags: ["gql", "match", "path-search", "rowan", "ast", "performance"],
        commands: [
            {
                label: "scenario-budget",
                argv: [
                    "cargo", "test", "-p", "gql-ast",
                    "path_search_prefixes_stay_inside_scenario_budget",
                    "--", "--nocapture"
                ]
            },
            {
                label: "criterion",
                argv: ["cargo", "bench", "-p", "gql-ast", "--bench", "ast_perf", "path-search-prefixes-v1"]
            }
        ],
        benchmark: {
            harness: "libtest",
            test: "path_search_prefixes_stay_inside_scenario_budget",
            snapshot: "gql_ast_path_search_prefixes_v1",
            target_total: "400us",
            max_total: "5ms",
            regression_budget: "750us",
            memory_budget_bytes: MEMORY_BUDGET_BYTES,
            target_rationale: "One bounded statement exercises the complete graph match, per-path search, traversal uniqueness, path target, and KEEP surface without semantic resolution, backend work, fallback, or runtime grammar dispatch.",
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
