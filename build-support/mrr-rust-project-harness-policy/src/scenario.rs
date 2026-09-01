//! Scenario packaging primitives for downstream GQL Rust harness users.

/// One command expectation attached to a custom GQL Rust harness scenario.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MrrRustProjectHarnessScenarioCommand {
    pub label: &'static str,
    pub argv: &'static [&'static str],
}

/// A reusable custom scenario owned by the GQL Rust harness policy crate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MrrRustProjectHarnessScenario {
    pub name: &'static str,
    pub package_name: &'static str,
    pub description: &'static str,
    pub fixture_root: &'static str,
    pub tags: &'static [&'static str],
    pub commands: &'static [MrrRustProjectHarnessScenarioCommand],
}

/// A package of custom scenarios that a member crate can expose from tests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MrrRustProjectHarnessScenarioPackage {
    pub package_name: &'static str,
    pub scenarios: Vec<MrrRustProjectHarnessScenario>,
}

/// Builds an GQL Rust harness scenario from declarative package data.
#[macro_export]
macro_rules! mrr_rust_project_harness_scenario {
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
        $crate::MrrRustProjectHarnessScenario {
            name: $name,
            package_name: $package_name,
            description: $description,
            fixture_root: $fixture_root,
            tags: &[$($tag),*],
            commands: &[
                $(
                    $crate::MrrRustProjectHarnessScenarioCommand {
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
macro_rules! mrr_rust_project_harness_scenario_package {
    (
        package: $package_name:expr,
        scenarios: [$($scenario:expr),* $(,)?] $(,)?
    ) => {
        $crate::MrrRustProjectHarnessScenarioPackage {
            package_name: $package_name,
            scenarios: vec![$($scenario),*],
        }
    };
}
