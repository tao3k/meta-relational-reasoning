//! Scenario packaging primitives for downstream GQL Rust harness users.

/// One command expectation attached to a custom GQL Rust harness scenario.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GqlRustProjectHarnessScenarioCommand {
    pub label: &'static str,
    pub argv: &'static [&'static str],
}

/// A reusable custom scenario owned by the GQL Rust harness policy crate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GqlRustProjectHarnessScenario {
    pub name: &'static str,
    pub package_name: &'static str,
    pub description: &'static str,
    pub fixture_root: &'static str,
    pub tags: &'static [&'static str],
    pub commands: &'static [GqlRustProjectHarnessScenarioCommand],
}

/// A package of custom scenarios that a member crate can expose from tests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GqlRustProjectHarnessScenarioPackage {
    pub package_name: &'static str,
    pub scenarios: Vec<GqlRustProjectHarnessScenario>,
}

/// Builds an GQL Rust harness scenario from declarative package data.
#[macro_export]
macro_rules! gql_rust_project_harness_scenario {
    (
        name: $name:expr,
        package: $package_name:expr,
        description: $description:expr,
        fixture_root: $fixture_root:expr,
        tags: [$($tag:expr),* $(,)?],
        commands: [
            $(
                {
                    label: $label:expr,
                    argv: [$($argv:expr),* $(,)?]
                }
            ),* $(,)?
        ] $(,)?
    ) => {
        $crate::GqlRustProjectHarnessScenario {
            name: $name,
            package_name: $package_name,
            description: $description,
            fixture_root: $fixture_root,
            tags: &[$($tag),*],
            commands: &[
                $(
                    $crate::GqlRustProjectHarnessScenarioCommand {
                        label: $label,
                        argv: &[$($argv),*],
                    }
                ),*
            ],
        }
    };
}

/// Builds a package-level collection of GQL Rust harness scenarios.
#[macro_export]
macro_rules! gql_rust_project_harness_scenario_package {
    (
        package: $package_name:expr,
        scenarios: [$($scenario:expr),* $(,)?] $(,)?
    ) => {
        $crate::GqlRustProjectHarnessScenarioPackage {
            package_name: $package_name,
            scenarios: vec![$($scenario),*],
        }
    };
}

/// Backward-compatible alias: ASP-prefixed scenario macro remains available.
#[macro_export]
macro_rules! asp_rust_project_harness_scenario {
    (
        name: $name:expr,
        package: $package_name:expr,
        description: $description:expr,
        fixture_root: $fixture_root:expr,
        tags: [$($tag:expr),* $(,)?],
        commands: [
            $(
                {
                    label: $label:expr,
                    argv: [$($argv:expr),* $(,)?]
                }
            ),* $(,)?
        ] $(,)?
    ) => {
        $crate::gql_rust_project_harness_scenario! {
            name: $name,
            package: $package_name,
            description: $description,
            fixture_root: $fixture_root,
            tags: [$($tag),*],
            commands: [
                $(
                    {
                        label: $label,
                        argv: [$($argv),*],
                    }
                ),*
            ],
        }
    };
}

/// Backward-compatible alias: ASP-prefixed scenario-package macro remains available.
#[macro_export]
macro_rules! asp_rust_project_harness_scenario_package {
    (
        package: $package_name:expr,
        scenarios: [$($scenario:expr),* $(,)?] $(,)?
    ) => {
        $crate::gql_rust_project_harness_scenario_package! {
            package: $package_name,
            scenarios: [$($scenario),*],
        }
    };
}
