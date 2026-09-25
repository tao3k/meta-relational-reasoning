// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use mrr_identity::{EntityId, RelationId};
use std::str::FromStr;

fn bench(c: &mut Criterion) {
    c.bench_function("mrr_identity_new", |b| {
        b.iter(|| RelationId::from_canonical_bytes(std::hint::black_box(b"relation:benchmark")))
    });

    let relation = RelationId::from_canonical_bytes(b"relation:benchmark")
        .expect("benchmark relation")
        .to_string();
    c.bench_function("mrr_identity_parse", |b| {
        b.iter(|| RelationId::from_str(std::hint::black_box(&relation)))
    });

    let encoded = (0..10_000)
        .map(|index| {
            EntityId::from_canonical_bytes(format!("benchmark-entity-{index}"))
                .expect("benchmark entity")
                .to_string()
        })
        .collect::<Vec<_>>();
    let mut group = c.benchmark_group("mrr_identity_parse_batch");
    group.throughput(Throughput::Elements(encoded.len() as u64));
    group.bench_function("10k", |b| {
        b.iter(|| {
            encoded
                .iter()
                .map(|value| EntityId::from_str(std::hint::black_box(value)))
                .collect::<Result<Vec<_>, _>>()
        })
    });
    group.finish();
}
criterion_group!(benches, bench);
criterion_main!(benches);
