use core::num::NonZeroUsize;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mrr_ascent::{ClosureConfig, ClosureLimits, evaluate_transitive_closure};
use mrr_bundle::{ReasoningBundle, ReasoningBundleDeclaration, RulePack};
use mrr_identity::{EntityId, FactId, GenerationId, RelationId, RuleId, RulePackId};
use mrr_logic::Rule;
use mrr_query::{Atom, Term, Variable};
use mrr_relation::{
    EvidenceCompleteness, Fact, FactProvenance, FactValidity, RelationAuthority,
    RelationCardinality, RelationContext, RelationField, RelationSchema, Value, ValueType,
};

const SCALES: &[usize] = &[1_000, 10_000, 100_000];

struct Fixture {
    bundle: ReasoningBundle,
    config: ClosureConfig,
    generation: GenerationId,
    limits: ClosureLimits,
}

fn variable(name: &str) -> Variable {
    Variable::new(name).expect("variable")
}

fn atom(relation: RelationId, names: &[&str]) -> Atom {
    Atom {
        relation,
        terms: names
            .iter()
            .map(|name| Term::Variable(variable(name)))
            .collect(),
    }
}

fn schema(relation: RelationId, name: &str) -> RelationSchema {
    RelationSchema::new(
        relation,
        name,
        vec![
            RelationField::new("from", ValueType::String).expect("from field"),
            RelationField::new("to", ValueType::String).expect("to field"),
        ],
        RelationCardinality::ManyToMany,
    )
    .expect("schema")
}

fn fixture(size: usize) -> Fixture {
    let edge = RelationId::from_canonical_bytes(b"closure:edge").expect("edge identity");
    let reachable =
        RelationId::from_canonical_bytes(b"closure:reachable").expect("reachable identity");
    let base = RuleId::from_canonical_bytes(b"closure:base").expect("base identity");
    let transitive =
        RuleId::from_canonical_bytes(b"closure:transitive").expect("transitive identity");
    let pack = RulePackId::from_canonical_bytes(b"closure:pack").expect("pack identity");
    let generation =
        GenerationId::from_canonical_bytes(b"closure:generation").expect("generation identity");
    let authority =
        EntityId::from_canonical_bytes(b"closure:authority").expect("authority identity");
    let facts = (0..size)
        .map(|index| {
            let value = Value::String(index.to_string());
            Fact::new(
                FactId::from_canonical_bytes(format!("closure:fact:{index}"))
                    .expect("fact identity"),
                edge,
                vec![value.clone(), value],
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
    let rules = vec![
        Rule::new(
            base,
            atom(reachable, &["x", "y"]),
            vec![atom(edge, &["x", "y"])],
        )
        .expect("base rule"),
        Rule::new(
            transitive,
            atom(reachable, &["x", "z"]),
            vec![atom(reachable, &["x", "y"]), atom(edge, &["y", "z"])],
        )
        .expect("transitive rule"),
    ];
    let bundle = ReasoningBundle::admit(ReasoningBundleDeclaration {
        relations: vec![schema(edge, "edge"), schema(reachable, "reachable")],
        facts,
        rule_packs: vec![RulePack::new(pack, rules)],
        ..ReasoningBundleDeclaration::default()
    })
    .expect("bundle");
    Fixture {
        bundle,
        config: ClosureConfig::new(edge, reachable, pack, base, transitive),
        generation,
        limits: ClosureLimits::new(
            NonZeroUsize::new(size).expect("input budget"),
            NonZeroUsize::new(size.checked_mul(size).expect("pair budget")).expect("pair budget"),
            NonZeroUsize::new(size).expect("result budget"),
        ),
    }
}

fn bench(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("ascent_closure");
    for &size in SCALES {
        let fixture = fixture(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &fixture,
            |bencher, fixture| {
                bencher.iter(|| {
                    evaluate_transitive_closure(
                        &fixture.bundle,
                        fixture.config,
                        fixture.generation,
                        fixture.limits,
                    )
                    .expect("closure")
                })
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
