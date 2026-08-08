//! Registers Git-tracked hook matcher and resident-session replay scenarios for GQL.

use crate::{
    GqlRustProjectHarnessScenarioPackage, gql_rust_project_harness_scenario,
    gql_rust_project_harness_scenario_package,
};

/// Package that owns the canonical GQL hook scenarios.
pub const GQL_HOOK_SCENARIO_PACKAGE_NAME: &str = "gql-hook-v1";

/// Scenario that verifies resident dispatch for arbitrary wrappers around Cargo test.
pub const GENERIC_WRAPPER_TESTING_RESIDENT_DISPATCH_SCENARIO_ID: &str =
    "generic-wrapper-testing-resident-dispatch";

/// Builds the GQL hook scenario package.
#[must_use]
pub fn gql_hook_scenario_package() -> GqlRustProjectHarnessScenarioPackage {
    gql_rust_project_harness_scenario_package!(
        package: GQL_HOOK_SCENARIO_PACKAGE_NAME,
        scenarios: [
            gql_rust_project_harness_scenario!(
                name: GENERIC_WRAPPER_TESTING_RESIDENT_DISPATCH_SCENARIO_ID,
                package: GQL_HOOK_SCENARIO_PACKAGE_NAME,
                description: "Arbitrary wrappers around cargo test route through the configured resident dispatch loop.",
                fixture_root: "crates/gql/tests/fixtures/scenarios/generic_wrapper_testing_resident_dispatch",
                tags: ["hook", "command-match", "session", "resident-dispatch", "performance"],
                commands: [
                    {
                        label: "command-match-snapshot",
                        argv: [
                            "cargo",
                            "test",
                            "-p",
                            "gql",
                            "--test",
                            "integration_test",
                            "wrapper_match_accepts_arbitrary_wrapper_names",
                            "--",
                            "--nocapture",
                        ]
                    },
                    {
                        label: "hook-session-snapshot",
                        argv: [
                            "cargo",
                            "test",
                            "-p",
                            "gql",
                            "--test",
                            "unit_test",
                            "generic_wrapper_testing_resident_dispatch_matches_git_snapshot",
                            "--",
                            "--nocapture",
                        ]
                    },
                    {
                        label: "wrapper-match-performance-gate",
                        argv: [
                            "cargo",
                            "test",
                            "-p",
                            "gql",
                            "--test",
                            "integration_test",
                            "wrapped_command_match_stays_within_git_snapshot_budget",
                            "--",
                            "--nocapture",
                        ]
                    },
                ],
            ),
        ],
    )
}
