use mrr_asp_rust_project_policy::{gql_search_scenario_package, mrr_workspace_member_policies};
use criterion::{Criterion, criterion_group, criterion_main};

fn policy_lookup_smoke_benchmark(criterion: &mut Criterion) {
    criterion.bench_function("mrr_workspace_member_policies", |bencher| {
        bencher.iter(mrr_workspace_member_policies)
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
