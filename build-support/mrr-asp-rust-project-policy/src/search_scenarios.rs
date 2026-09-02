//! Search-owned GQL Rust harness scenarios.

use crate::{
    AspRustProjectScenarioPackage, asp_rust_project_scenario, asp_rust_project_scenario_package,
};

/// Package name for GQL search scenario gates.
pub const GQL_SEARCH_SCENARIO_PACKAGE_NAME: &str = "gql";

/// Package-local coverage monitor scenario for search-like surfaces.
pub const SEARCH_PACKAGE_LINEAR_PERFORMANCE_SCENARIO_ID: &str =
    "search-package-linear-performance-monitoring";

/// Warm lexical GQL AST and parser performance scenario.
pub const LEXICAL_SEARCH_FRAME_GRAPH_ROUTER_WARM_PATH_SCENARIO_ID: &str =
    "lexical-search-frame-graph-router-warm-path";

/// Source-index evidence chain from owner evidence to graph nodes.
pub const SEARCH_SOURCE_INDEX_OWNER_ITEM_GRAPH_CHAIN_SCENARIO_ID: &str =
    "search-source-index-owner-item-graph-chain";

/// Busy source-index miss route must not fall through to fallback matcher.
pub const SEARCH_SOURCE_INDEX_BUSY_MISS_SCENARIO_ID: &str =
    "search-source-index-busy-miss-overlay-skipped";

/// Cold source-index schemas require an explicit rebuild and must not fall through.
pub const SEARCH_SOURCE_INDEX_COLD_REQUIRED_SCENARIO_ID: &str =
    "search-source-index-cold-required-overlay-skipped";

/// Read-only client-like lookup must work while source path writes are disallowed.
pub const SEARCH_SOURCE_INDEX_READ_ONLY_CLIENT_DB_SCENARIO_ID: &str =
    "search-source-index-read-only-client-db-zero-write";

/// Merkle-qualified live-memory path warm-path check.
pub const CODE_SEARCH_MERKLE_MEMORY_WARM_PATH_SCENARIO_ID: &str =
    "code-search-merkle-memory-warm-path";

/// Merkle-qualified resident session warm-path check.
pub const CODE_SEARCH_TURSO_RESIDENT_SESSION_WARM_PATH_SCENARIO_ID: &str =
    "code-search-turso-resident-session-warm-path";

/// GraphRouter next-action policy for selector-ready evidence.
pub const SEARCH_GRAPH_ROUTER_NEXT_EXACT_ACTION_SCENARIO_ID: &str =
    "search-graph-router-next-exact-action";

/// Compact graph-route receipt contract.
pub const SEARCH_SUBAGENT_COMPACT_RECEIPT_SCENARIO_ID: &str = "search-subagent-compact-receipt";

/// Bounded route contract for recovery/fallback.
pub const SEARCH_DEGRADED_ROUTE_BOUNDED_SCENARIO_ID: &str = "search-degraded-route-bounded";

/// Builds the GQL-owned search scenario package consumed by Rust harness policy.
#[must_use]
pub fn gql_search_scenario_package() -> AspRustProjectScenarioPackage {
    asp_rust_project_scenario_package!(
        package: GQL_SEARCH_SCENARIO_PACKAGE_NAME,
        scenarios: [
            asp_rust_project_scenario!(
                name: SEARCH_PACKAGE_LINEAR_PERFORMANCE_SCENARIO_ID,
                package: GQL_SEARCH_SCENARIO_PACKAGE_NAME,
                description: "Search package surfaces stay covered by package-local benchmark metadata.",
                fixture_root: "crates/gql/tests/unit/scenarios/search-package-linear-performance-monitoring",
                tags: ["search", "performance", "package-monitoring"],
                commands: [
                    {
                        label: "surface-coverage",
                        argv: [
                            "cargo",
                            "test",
                            "-p",
                            "gql",
                            "search_package_linear_performance_monitoring_covers_all_unit_surfaces",
                        ]
                    },
                ],
            ),
            asp_rust_project_scenario!(
                name: LEXICAL_SEARCH_FRAME_GRAPH_ROUTER_WARM_PATH_SCENARIO_ID,
                package: GQL_SEARCH_SCENARIO_PACKAGE_NAME,
                description: "Lexical parser warm evidence routes through parser graph without provider startup.",
                fixture_root: "crates/gql/tests/unit/scenarios/lexical_search_frame_graph_router_warm_path",
                tags: ["search", "performance", "search-frame", "graph-router"],
                commands: [
                    {
                        label: "warm-path-gate",
                        argv: [
                            "cargo",
                            "test",
                            "-p",
                            "gql",
                            "--test",
                            "unit_test",
                            "lexical_search_frame_warm_path_stays_inside_scenario_gate",
                            "--",
                            "--nocapture",
                        ]
                    },
                ],
            ),
            asp_rust_project_scenario!(
                name: SEARCH_SOURCE_INDEX_OWNER_ITEM_GRAPH_CHAIN_SCENARIO_ID,
                package: GQL_SEARCH_SCENARIO_PACKAGE_NAME,
                description:
                    "Source-index evidence projects to executable owner, item, and hot graph nodes.",
                fixture_root:
                    "crates/gql/tests/unit/scenarios/search_source_index_owner_item_graph_chain",
                tags: ["search", "source-index", "evidence-graph", "owner-item"],
                commands: [
                    {
                        label: "owner-item-graph-chain",
                        argv: [
                            "cargo",
                            "test",
                            "-p",
                            "gql",
                            "--test",
                            "unit_test",
                            "search_flow_source_index_owner_item_graph_chain_is_executable",
                            "--",
                            "--nocapture",
                        ]
                    },
                ],
            ),
            asp_rust_project_scenario!(
                name: SEARCH_SOURCE_INDEX_BUSY_MISS_SCENARIO_ID,
                package: GQL_SEARCH_SCENARIO_PACKAGE_NAME,
                description: "Busy source-index misses return immediately and skip fallback matcher.",
                fixture_root:
                    "crates/gql/tests/unit/scenarios/search_source_index_busy_miss_overlay_skipped",
                tags: ["search", "source-index", "performance", "busy", "fallback"],
                commands: [
                    {
                        label: "busy-miss-fallback-skipped",
                        argv: [
                            "cargo",
                            "test",
                            "-p",
                            "gql",
                            "--test",
                            "unit_test",
                            "search_flow_busy_source_index_miss_returns_fallback_skipped",
                            "--",
                            "--nocapture",
                        ]
                    },
                ],
            ),
            asp_rust_project_scenario!(
                name: SEARCH_SOURCE_INDEX_COLD_REQUIRED_SCENARIO_ID,
                package: GQL_SEARCH_SCENARIO_PACKAGE_NAME,
                description: "Cold source-index schemas require rebuild and skip fallback matcher.",
                fixture_root:
                    "crates/gql/tests/unit/scenarios/search_source_index_cold_required_overlay_skipped",
                tags: ["search", "source-index", "performance", "cold-required", "fallback"],
                commands: [
                    {
                        label: "cold-required-overlay-skipped",
                        argv: [
                            "cargo",
                            "test",
                            "-p",
                            "gql",
                            "--test",
                            "unit_test",
                            "search_flow_cold_required_source_index_returns_fallback_skipped",
                            "--",
                            "--nocapture",
                        ]
                    },
                ],
            ),
            asp_rust_project_scenario!(
                name: SEARCH_SOURCE_INDEX_READ_ONLY_CLIENT_DB_SCENARIO_ID,
                package: GQL_SEARCH_SCENARIO_PACKAGE_NAME,
                description: "Source-index lookup stays bounded and produces no write attempts.",
                fixture_root: "crates/gql-core/tests/unit/scenarios/search_source_index_read_only_client_db",
                tags: ["search", "source-index", "performance", "read-only", "memory"],
                commands: [
                    {
                        label: "read-only-client-db-zero-write",
                        argv: [
                            "cargo",
                            "test",
                            "-p",
                            "gql-core",
                            "--test",
                            "unit_test",
                            "search_lookup_succeeds_without_client_dir_write_permission",
                            "--",
                            "--nocapture",
                        ]
                    },
                ],
            ),
            asp_rust_project_scenario!(
                name: CODE_SEARCH_MERKLE_MEMORY_WARM_PATH_SCENARIO_ID,
                package: GQL_SEARCH_SCENARIO_PACKAGE_NAME,
                description:
                    "Merkle-qualified live-memory code search stays below warm-path budget without fallback providers.",
                fixture_root:
                    "crates/gql-core/tests/unit/scenarios/code_search_merkle_memory_warm_path",
                tags: ["search", "code-search", "performance", "merkle", "memory"],
                commands: [
                    {
                        label: "merkle-memory-warm-path-gate",
                        argv: [
                            "cargo",
                            "test",
                            "-p",
                            "gql-core",
                            "--test",
                            "performance_test",
                            "code_search_merkle_memory_warm_path_is_a_strong_gate",
                            "--",
                            "--nocapture",
                        ]
                    },
                ],
            ),
            asp_rust_project_scenario!(
                name: CODE_SEARCH_TURSO_RESIDENT_SESSION_WARM_PATH_SCENARIO_ID,
                package: GQL_SEARCH_SCENARIO_PACKAGE_NAME,
                description:
                    "Merkle-qualified code search reuses resident read sessions without reconnecting.",
                fixture_root:
                    "crates/gql-core/tests/unit/scenarios/code_search_turso_resident_session_warm_path",
                tags: ["search", "code-search", "performance", "merkle", "resident-session"],
                commands: [
                    {
                        label: "turso-resident-session-warm-path-gate",
                        argv: [
                            "cargo",
                            "test",
                            "-p",
                            "gql-core",
                            "--test",
                            "performance_test",
                            "code_search_turso_resident_session_warm_path_is_a_strong_gate",
                            "--",
                            "--nocapture",
                        ]
                    },
                ],
            ),
            asp_rust_project_scenario!(
                name: SEARCH_GRAPH_ROUTER_NEXT_EXACT_ACTION_SCENARIO_ID,
                package: GQL_SEARCH_SCENARIO_PACKAGE_NAME,
                description:
                    "GraphRouter chooses exact selector actions and rejects broad fallback after selector-ready evidence.",
                fixture_root:
                    "crates/gql/tests/unit/scenarios/search_graph_router_next_exact_action",
                tags: ["search", "graph-router", "next-action", "selector-ready"],
                commands: [
                    {
                        label: "next-exact-action",
                        argv: [
                            "cargo",
                            "test",
                            "-p",
                            "gql",
                            "--test",
                            "unit_test",
                            "search_flow_graph_router_prefers_exact_action_for_selector_ready_item",
                            "--",
                            "--nocapture",
                        ]
                    },
                ],
            ),
            asp_rust_project_scenario!(
                name: SEARCH_SUBAGENT_COMPACT_RECEIPT_SCENARIO_ID,
                package: GQL_SEARCH_SCENARIO_PACKAGE_NAME,
                description:
                    "Search subagents return compact graph-route receipts without source body or extra detail.",
                fixture_root: "crates/gql/tests/unit/scenarios/search_subagent_compact_receipt",
                tags: ["search", "subagent", "receipt", "graph-route"],
                commands: [
                    {
                        label: "compact-receipt",
                        argv: [
                            "cargo",
                            "test",
                            "-p",
                            "gql",
                            "--test",
                            "unit_test",
                            "search_flow_subagent_receipt_is_compact_graph_route",
                            "--",
                            "--nocapture",
                        ]
                    },
                ],
            ),
            asp_rust_project_scenario!(
                name: SEARCH_DEGRADED_ROUTE_BOUNDED_SCENARIO_ID,
                package: GQL_SEARCH_SCENARIO_PACKAGE_NAME,
                description:
                    "Source-index and owner-item misses return an explicit bounded receipt instead of silent fallback.",
                fixture_root: "crates/gql/tests/unit/scenarios/search_degraded_route_bounded",
                tags: ["search", "graph-router", "recovery", "bounded-fallback"],
                commands: [
                    {
                        label: "bounded-degraded-route",
                        argv: [
                            "cargo",
                            "test",
                            "-p",
                            "gql",
                            "--test",
                            "unit_test",
                            "search_flow_degraded_source_index_miss_uses_bounded_receipt_reason",
                            "--",
                            "--nocapture",
                        ]
                    },
                ],
            ),
            asp_rust_project_scenario!(
                name: "tree-sitter-querycursor-native-hot-path",
                package: GQL_SEARCH_SCENARIO_PACKAGE_NAME,
                description:
                    "Canonical Tree-sitter QueryCursor execution keeps predicate semantics and bounded native hot-path metrics visible.",
                fixture_root: "crates/gql-syntax/tests/unit/scenarios/tree-sitter-querycursor-native-hot-path",
                tags: ["search", "query", "tree-sitter", "performance", "native-runtime"],
                commands: [
                    {
                        label: "querycursor-packet-hot-path",
                        argv: [
                            "cargo",
                            "test",
                            "--manifest-path",
                            "crates/gql-syntax/Cargo.toml",
                            "--test",
                            "unit_test",
                            "tree_sitter_query_json_projects_matches_and_native_enrichment",
                            "--",
                            "--nocapture",
                        ]
                    },
                    {
                        label: "querycursor-predicate-contract",
                        argv: [
                            "cargo",
                            "test",
                            "--manifest-path",
                            "crates/gql-syntax/Cargo.toml",
                            "--test",
                            "unit_test",
                            "gql_syntax_querypredicates",
                            "--",
                            "--nocapture",
                        ]
                    },
                ],
            ),
        ],
    )
}
