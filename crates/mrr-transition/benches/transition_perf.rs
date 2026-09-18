use core::num::NonZeroUsize;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mrr_transition::{
    Action, ActionId, Effect, FactId, InitialState, Invariant, Precondition, SafetyLimits,
    StatePredicate, StateSchema, StateSnapshot, TransitionSystem, check_safety,
};

const SCALES: &[usize] = &[1_000, 10_000, 100_000];

fn fixture(size: usize) -> TransitionSystem {
    let facts = (0..=size)
        .map(|index| {
            FactId::from_canonical_bytes(format!("state:fact:{index}")).expect("fact identity")
        })
        .collect::<Vec<_>>();
    let actions = (0..size)
        .map(|index| {
            Action::new(
                ActionId::from_canonical_bytes(format!("state:action:{index}"))
                    .expect("action identity"),
                Precondition::all(vec![StatePredicate::Present(facts[index])]),
                Effect::new(vec![facts[index + 1]], vec![facts[index]]).expect("effect"),
            )
        })
        .collect();
    TransitionSystem::admit(
        StateSchema::new(facts.clone()).expect("state schema"),
        InitialState::new(StateSnapshot::from_facts(vec![facts[0]]).expect("initial state")),
        actions,
        vec![Invariant::forbidden_all("terminal", vec![facts[size]]).expect("invariant")],
    )
    .expect("transition system")
}

fn bench(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("state_exploration");
    for &size in SCALES {
        let system = fixture(size);
        let limits = SafetyLimits::new(
            NonZeroUsize::new(size + 2).expect("state budget"),
            NonZeroUsize::new(size + 1).expect("transition budget"),
        );
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &system,
            |bencher, system| {
                bencher.iter(|| check_safety(system, limits).expect("state exploration"))
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
