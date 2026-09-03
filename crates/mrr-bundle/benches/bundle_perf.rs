use criterion::{Criterion, criterion_group, criterion_main};
use mrr_bundle::{ReasoningBundle, ReasoningBundleDeclaration};
use mrr_identity::RelationId;
use mrr_relation::{RelationCardinality, RelationField, RelationSchema, ValueType};

fn bench(c: &mut Criterion) {
    c.bench_function("mrr_bundle_admit_minimal", |b| {
        let declaration = ReasoningBundleDeclaration {
            relations: vec![
                RelationSchema::new(
                    RelationId::from_canonical_bytes(b"bench:relation").expect("relation"),
                    "bench",
                    vec![RelationField::new("value", ValueType::Integer).expect("field")],
                    RelationCardinality::ManyToMany,
                )
                .expect("schema"),
            ],
            ..ReasoningBundleDeclaration::default()
        };
        b.iter(|| ReasoningBundle::admit(declaration.clone()))
    });
}
criterion_group!(benches, bench);
criterion_main!(benches);
