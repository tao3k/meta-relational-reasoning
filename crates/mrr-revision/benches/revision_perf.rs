// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

use criterion::{Criterion, criterion_group, criterion_main};
use mrr_revision::{ExternalRevisionIdentity, GenerationId, RevisionBinding};

fn benchmark(c: &mut Criterion) {
    let external = ExternalRevisionIdentity::new("jj", "change", "content").unwrap();
    let generation = GenerationId::from_canonical_bytes(b"benchmark-generation").unwrap();
    c.bench_function("revision_binding", |b| {
        b.iter(|| RevisionBinding::admit(external.clone(), generation).unwrap())
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
