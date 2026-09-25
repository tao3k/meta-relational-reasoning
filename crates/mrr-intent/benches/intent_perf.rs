// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

use criterion::{Criterion, criterion_group, criterion_main};
use mrr_intent::IntentSemanticModel;

fn benchmark(c: &mut Criterion) {
    let source = include_str!("../../../fixtures/org/runtime-lifecycle.org");
    c.bench_function("intent_projection", |b| {
        b.iter(|| IntentSemanticModel::project_org(source).unwrap())
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
