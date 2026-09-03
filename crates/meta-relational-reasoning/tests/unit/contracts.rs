use crate::{
    ReasoningBundle, ReasoningBundleDeclaration, RelationCardinality, RelationField, RelationId,
    RelationSchema, ValueType,
};
#[test]
fn facade_exposes_one_composable_contract_graph() {
    let bundle = ReasoningBundle::admit(ReasoningBundleDeclaration {
        relations: vec![
            RelationSchema::new(
                RelationId::from_canonical_bytes(b"relation:depends-on").expect("relation id"),
                "depends_on",
                vec![
                    RelationField::new("subject", ValueType::Entity).expect("subject field"),
                    RelationField::new("object", ValueType::Entity).expect("object field"),
                ],
                RelationCardinality::ManyToMany,
            )
            .expect("schema"),
        ],
        facts: Vec::new(),
        ..ReasoningBundleDeclaration::default()
    })
    .expect("bundle admission");
    assert_eq!(bundle.validate(), Ok(()));
}
#[path = "closure_admission.rs"]
mod closure_admission;
#[path = "counterexample_lineage.rs"]
mod counterexample_lineage;
#[path = "truth_status.rs"]
mod truth_status;
