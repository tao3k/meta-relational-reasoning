//! ASP Rust build-support scenario contracts used by the GQL workspace.

pub use asp_rust_build_support::{
    AspRustScenario as AspRustProjectScenario,
    AspRustScenarioBenchmarkSpec as AspRustProjectScenarioBenchmarkSpec,
    AspRustScenarioCommand as AspRustProjectScenarioCommand,
    AspRustScenarioMeasurement as AspRustProjectScenarioMeasurement,
    AspRustScenarioMetricKind as AspRustProjectScenarioMetricKind,
    AspRustScenarioMetricSpec as AspRustProjectScenarioMetricSpec,
    AspRustScenarioObservation as AspRustProjectScenarioObservation,
    AspRustScenarioPackage as AspRustProjectScenarioPackage,
    measure_asp_rust_scenario as measure_asp_rust_project_scenario,
    render_asp_rust_scenario_benchmark_toml as render_asp_rust_project_scenario_benchmark_toml,
    write_asp_rust_scenario_benchmark_toml as write_asp_rust_project_scenario_benchmark_toml,
};

pub use asp_rust_build_support::{
    asp_rust_scenario as asp_rust_project_scenario,
    asp_rust_scenario_package as asp_rust_project_scenario_package,
};
