use criterion::{Criterion, criterion_group, criterion_main};
use mrr_asp_rust_project_policy::mrr_workspace_member_policies;

fn policy_lookup_smoke_benchmark(criterion: &mut Criterion) {
    criterion.bench_function("mrr_workspace_member_policies", |bencher| {
        bencher.iter(mrr_workspace_member_policies)
    });
}

criterion_group!(performance_verification, policy_lookup_smoke_benchmark);
criterion_main!(performance_verification);
