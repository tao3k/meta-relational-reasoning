use core::num::NonZeroUsize;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mrr_logic::{Rule, WhyNotLimits, why_not};
use mrr_query::{Atom, RelationId, Term, Variable};
use mrr_relation::{
    EntityId, EvidenceCompleteness, Fact, FactId, FactProvenance, FactValidity, GenerationId,
    RelationAuthority, RelationContext, Value,
};

const SCALES: &[usize] = &[1_000, 10_000, 100_000];

fn variable(name: &str) -> Variable {
    Variable::new(name).expect("variable")
}

fn fixture(size: usize) -> (Atom, GenerationId, Rule, Vec<Fact>) {
    let goal_relation = RelationId::from_canonical_bytes(b"why-not:goal").expect("goal relation");
    let premise_relation =
        RelationId::from_canonical_bytes(b"why-not:premise").expect("premise relation");
    let unrelated =
        RelationId::from_canonical_bytes(b"why-not:unrelated").expect("unrelated relation");
    let generation = GenerationId::from_canonical_bytes(b"why-not:generation").expect("generation");
    let authority = EntityId::from_canonical_bytes(b"why-not:authority").expect("authority");
    let goal = Atom {
        relation: goal_relation,
        terms: vec![Term::Value(Value::String("missing".into()))],
    };
    let rule = Rule::new(
        mrr_logic::RuleId::from_canonical_bytes(b"why-not:rule").expect("rule identity"),
        Atom {
            relation: goal_relation,
            terms: vec![Term::Variable(variable("x"))],
        },
        vec![Atom {
            relation: premise_relation,
            terms: vec![Term::Variable(variable("x"))],
        }],
    )
    .expect("rule");
    let facts = (0..size)
        .map(|index| {
            Fact::new(
                FactId::from_canonical_bytes(format!("why-not:fact:{index}"))
                    .expect("fact identity"),
                unrelated,
                vec![Value::String(index.to_string())],
                RelationContext::new(
                    generation,
                    RelationAuthority::Entity(authority),
                    FactProvenance::Source(authority),
                    EvidenceCompleteness::Complete,
                    FactValidity::Valid,
                ),
            )
        })
        .collect();
    (goal, generation, rule, facts)
}

fn bench(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("why_not_analysis");
    let limits = WhyNotLimits::new(
        NonZeroUsize::new(8).expect("depth"),
        NonZeroUsize::new(8).expect("expansions"),
        NonZeroUsize::new(8).expect("alternatives"),
    );
    for &size in SCALES {
        let (goal, generation, rule, facts) = fixture(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &facts,
            |bencher, facts| {
                bencher.iter(|| {
                    why_not(
                        &goal,
                        generation,
                        std::slice::from_ref(&rule),
                        facts,
                        limits,
                    )
                    .expect("WHY-NOT analysis")
                })
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
