use criterion::{Criterion, criterion_group, criterion_main};
use gql_rust_project_harness_policy::{gql_search_scenario_package, gql_workspace_member_policies};

fn policy_lookup_smoke_benchmark(criterion: &mut Criterion) {
    criterion.bench_function("gql_workspace_member_policies", |bencher| {
        bencher.iter(|| gql_workspace_member_policies())
    });
}

fn scenario_package_smoke_benchmark(criterion: &mut Criterion) {
    criterion.bench_function("gql_search_scenario_package", |bencher| {
        bencher.iter(gql_search_scenario_package)
    });
}

criterion_group!(
    performance_verification,
    policy_lookup_smoke_benchmark,
    scenario_package_smoke_benchmark
);
criterion_main!(performance_verification);
