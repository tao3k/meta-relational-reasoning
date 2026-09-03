use core::num::NonZeroUsize;

use crate::{
    CandidateIdentities, ClosureAdmissionError, ClosureReceipt, DeductionError, DeductionLimits,
    DeductionPlan, DerivationId, EntityId, EvidenceCompleteness, Fact, FactId, FactProvenance,
    FactValidity, GenerationId, GenerationTransitionError, MrrEngine, ReasoningBundle,
    ReasoningBundleDeclaration, RelationAuthority, RelationCardinality, RelationContext,
    RelationField, RelationId, RelationSchema, Rule, RuleId, RulePack, RulePackId, Term, Value,
    ValueType, Variable, admit_closure_candidates,
};
use mrr_lineage::LineageError;
use mrr_query::Atom;

macro_rules! id {
    ($kind:ident, $value:expr) => {
        $kind::from_canonical_bytes(format!("test:{}:{}", stringify!($kind), $value))
            .expect("test identity input is canonical")
    };
}

fn variable(name: &str) -> Variable {
    Variable::new(name).expect("test variable")
}

fn atom(relation: RelationId, variables: &[&str]) -> Atom {
    Atom {
        relation,
        terms: variables
            .iter()
            .map(|name| Term::Variable(variable(name)))
            .collect(),
    }
}

fn binary_schema(relation: RelationId, predicate: &str) -> RelationSchema {
    RelationSchema::new(
        relation,
        predicate,
        vec![
            RelationField::new("from", ValueType::String).expect("from field"),
            RelationField::new("to", ValueType::String).expect("to field"),
        ],
        RelationCardinality::ManyToMany,
    )
    .expect("binary schema")
}

fn source_fact(identity: u128, relation: RelationId, from: &str, to: &str) -> Fact {
    let authority = id!(EntityId, identity);
    Fact::new(
        id!(FactId, identity),
        relation,
        vec![Value::String(from.into()), Value::String(to.into())],
        RelationContext::new(
            id!(GenerationId, 51),
            RelationAuthority::Entity(authority),
            FactProvenance::Source(authority),
            EvidenceCompleteness::Complete,
            FactValidity::Valid,
        ),
    )
}

fn fixture() -> (ReasoningBundle, DeductionPlan) {
    let edge = id!(RelationId, 1);
    let reachable = id!(RelationId, 2);
    let base = id!(RuleId, 10);
    let transitive = id!(RuleId, 11);
    let relations = vec![
        binary_schema(edge, "edge"),
        binary_schema(reachable, "reachable"),
    ];
    let facts = vec![
        source_fact(100, edge, "Ada", "Bob"),
        source_fact(101, edge, "Bob", "Cy"),
    ];
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
    (
        ReasoningBundle::admit(ReasoningBundleDeclaration {
            relations,
            facts,
            rule_packs: vec![RulePack::new(id!(RulePackId, 1), rules)],
            ..ReasoningBundleDeclaration::default()
        })
        .expect("bundle admission"),
        DeductionPlan::transitive_closure(edge, reachable, id!(RulePackId, 1), base, transitive),
    )
}

fn limits(results: usize) -> DeductionLimits {
    DeductionLimits::new(
        NonZeroUsize::new(8).unwrap(),
        NonZeroUsize::new(16).unwrap(),
        NonZeroUsize::new(results).unwrap(),
    )
}

fn evaluate_transitive_closure(
    bundle: &ReasoningBundle,
    plan: DeductionPlan,
    generation: GenerationId,
    limits: DeductionLimits,
) -> Result<ClosureReceipt, DeductionError> {
    MrrEngine::builder()
        .with_bundle(bundle.clone())
        .build()
        .expect("engine")
        .derive(plan, generation, limits)
}

fn identities(count: usize) -> Vec<CandidateIdentities> {
    (0..count)
        .map(|offset| {
            CandidateIdentities::new(
                id!(FactId, 1_000 + offset as u128),
                id!(DerivationId, 2_000 + offset as u128),
            )
        })
        .collect()
}

#[test]
fn admits_complete_candidates_into_exact_lineage_and_transition_outputs() {
    let (bundle, config) = fixture();
    let from = id!(GenerationId, 50);
    let to = id!(GenerationId, 51);
    let receipt =
        evaluate_transitive_closure(&bundle, config, to, limits(16)).expect("closure evaluation");
    let assigned = identities(receipt.candidates().len());

    let materialized = admit_closure_candidates(&receipt, from, to, &assigned)
        .expect("complete receipt admission");

    assert_eq!(materialized.transition().from(), from);
    assert_eq!(materialized.transition().to(), to);
    assert_eq!(
        materialized.transition().insertions().len(),
        receipt.candidates().len()
    );
    assert_eq!(materialized.transition().retractions(), &[]);
    assert_eq!(materialized.derivations().len(), assigned.len());
    assert_eq!(materialized.receipt().rule_pack(), id!(RulePackId, 1));
    assert_eq!(materialized.receipt().input_generation(), to);
    assert_eq!(
        materialized.receipt().input_fact_ids(),
        receipt.input_fact_ids()
    );
    assert_eq!(
        materialized.receipt().derived_fact_ids(),
        assigned
            .iter()
            .map(|identity| identity.fact())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        materialized.receipt().derivation_ids(),
        assigned
            .iter()
            .map(|identity| identity.derivation())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        materialized.receipt().closure_status(),
        crate::ClosureStatus::Complete
    );
    assert_ne!(materialized.receipt().digest(), &[0; 32]);
    for ((candidate, assigned), derivation) in receipt
        .candidates()
        .iter()
        .zip(&assigned)
        .zip(materialized.derivations())
    {
        assert_eq!(derivation.id(), assigned.derivation());
        assert_eq!(derivation.output().id(), assigned.fact());
        assert_eq!(derivation.output().relation(), candidate.relation());
        assert_eq!(derivation.output().values(), candidate.values());
        assert_eq!(derivation.output().context().generation(), to);
        assert_eq!(
            derivation.output().context().provenance(),
            FactProvenance::Derivation(assigned.derivation())
        );
        assert_eq!(derivation.rule(), candidate.rule());
        assert_eq!(derivation.generation(), to);
        assert_eq!(derivation.support(), candidate.support());
    }
}

#[test]
fn rejects_truncated_or_cross_generation_receipts_before_materialization() {
    let (bundle, config) = fixture();
    let from = id!(GenerationId, 50);
    let evaluated_generation = id!(GenerationId, 51);
    let truncated = evaluate_transitive_closure(&bundle, config, evaluated_generation, limits(1))
        .expect("bounded closure evaluation");
    assert_eq!(
        admit_closure_candidates(&truncated, from, evaluated_generation, &identities(1)),
        Err(ClosureAdmissionError::IncompleteReceipt)
    );

    let complete = evaluate_transitive_closure(&bundle, config, evaluated_generation, limits(16))
        .expect("complete closure evaluation");
    let wrong_target = id!(GenerationId, 52);
    assert_eq!(
        admit_closure_candidates(
            &complete,
            from,
            wrong_target,
            &identities(complete.candidates().len()),
        ),
        Err(ClosureAdmissionError::GenerationMismatch {
            expected: wrong_target,
            actual: evaluated_generation,
        })
    );
}

#[test]
fn rejects_identity_cardinality_duplicates_and_self_support() {
    let (bundle, config) = fixture();
    let from = id!(GenerationId, 50);
    let to = id!(GenerationId, 51);
    let receipt = evaluate_transitive_closure(&bundle, config, to, limits(16))
        .expect("complete closure evaluation");

    assert_eq!(
        admit_closure_candidates(&receipt, from, to, &[]),
        Err(ClosureAdmissionError::IdentityCountMismatch {
            candidates: receipt.candidates().len(),
            identities: 0,
        })
    );

    let mut duplicate = identities(receipt.candidates().len());
    duplicate[1] = CandidateIdentities::new(duplicate[0].fact(), duplicate[1].derivation());
    assert_eq!(
        admit_closure_candidates(&receipt, from, to, &duplicate),
        Err(ClosureAdmissionError::DuplicateFactId(duplicate[0].fact()))
    );

    let mut duplicate = identities(receipt.candidates().len());
    duplicate[1] = CandidateIdentities::new(duplicate[1].fact(), duplicate[0].derivation());
    assert_eq!(
        admit_closure_candidates(&receipt, from, to, &duplicate),
        Err(ClosureAdmissionError::DuplicateDerivationId(
            duplicate[0].derivation()
        ))
    );

    assert_eq!(
        admit_closure_candidates(&receipt, to, to, &identities(receipt.candidates().len())),
        Err(ClosureAdmissionError::Transition(
            GenerationTransitionError::SameGeneration
        ))
    );

    let mut self_support = identities(receipt.candidates().len());
    self_support[0] = CandidateIdentities::new(
        receipt.candidates()[0].support()[0],
        self_support[0].derivation(),
    );
    assert_eq!(
        admit_closure_candidates(&receipt, from, to, &self_support),
        Err(ClosureAdmissionError::Lineage(LineageError::SelfSupport))
    );
}
