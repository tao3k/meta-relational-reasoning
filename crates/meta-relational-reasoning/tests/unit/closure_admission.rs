use core::num::NonZeroUsize;

use crate::{
    BundleBoundClosure, CandidateIdentities, ClosureAdmissionError, ClosurePairComparisonError,
    DeductionError, DeductionLimits, DeductionPlan, DerivationId, EntityId, EvidenceCompleteness,
    Fact, FactId, FactProvenance, FactValidity, GenerationId, GenerationTransitionError, MrrEngine,
    ReasoningBundle, ReasoningBundleDeclaration, RelationAuthority, RelationContext, RelationField,
    RelationId, RelationSchema, Rule, RuleId, RulePack, RulePackId, Term, Value, ValueSchema,
    Variable, admit_closure_candidates,
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
            RelationField::new("from", ValueSchema::String, false).expect("from field"),
            RelationField::new("to", ValueSchema::String, false).expect("to field"),
        ],
        Vec::new(),
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
        )
        .expect("source context"),
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
) -> Result<BundleBoundClosure, DeductionError> {
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
fn compares_external_pairs_only_against_the_exact_complete_bundle_generation() {
    let (bundle, plan) = fixture();
    let generation = id!(GenerationId, 51);
    let engine = MrrEngine::builder()
        .with_bundle(bundle.clone())
        .build()
        .expect("engine");
    let complete = engine
        .derive(plan, generation, limits(16))
        .expect("complete Ascent result");
    let pairs = vec![
        ("Bob".into(), "Cy".into()),
        ("Ada".into(), "Cy".into()),
        ("Ada".into(), "Bob".into()),
    ];
    assert_eq!(
        engine.compare_closure_pairs(&complete, generation, &pairs),
        Ok(())
    );

    assert_eq!(
        engine.compare_closure_pairs(&complete, id!(GenerationId, 52), &pairs),
        Err(ClosurePairComparisonError::GenerationMismatch {
            expected: id!(GenerationId, 52),
            actual: generation,
        })
    );
    assert_eq!(
        engine.compare_closure_pairs(&complete, generation, &pairs[..2]),
        Err(ClosurePairComparisonError::PairCountMismatch {
            expected: 3,
            actual: 2,
        })
    );
    assert_eq!(
        engine.compare_closure_pairs(
            &complete,
            generation,
            &[pairs[0].clone(), pairs[0].clone(), pairs[2].clone()],
        ),
        Err(ClosurePairComparisonError::DuplicatePair(
            "Bob".into(),
            "Cy".into(),
        ))
    );
    assert_eq!(
        engine.compare_closure_pairs(
            &complete,
            generation,
            &[
                pairs[0].clone(),
                pairs[1].clone(),
                ("Ada".into(), "Eve".into())
            ],
        ),
        Err(ClosurePairComparisonError::PairSetMismatch)
    );

    let truncated = engine
        .derive(plan, generation, limits(1))
        .expect("bounded output");
    assert_eq!(
        engine.compare_closure_pairs(&truncated, generation, &pairs),
        Err(ClosurePairComparisonError::IncompleteReceipt)
    );

    let mut changed = bundle.declaration().clone();
    *changed
        .facts
        .iter_mut()
        .find(|fact| fact.id() == id!(FactId, 100))
        .expect("source fact 100") = source_fact(100, id!(RelationId, 1), "Ada", "Eve");
    let changed = MrrEngine::builder()
        .with_bundle(ReasoningBundle::admit(changed).expect("changed bundle"))
        .build()
        .expect("changed engine");
    assert_eq!(
        changed.compare_closure_pairs(&complete, generation, &pairs),
        Err(ClosurePairComparisonError::SourceBundleMismatch {
            expected: changed.bundle().id(),
            actual: bundle.id(),
        })
    );

    let mut empty_declaration = bundle.declaration().clone();
    empty_declaration.facts.clear();
    let empty_engine = MrrEngine::builder()
        .with_bundle(ReasoningBundle::admit(empty_declaration).expect("empty source bundle"))
        .build()
        .expect("empty engine");
    let empty_generation = id!(GenerationId, 53);
    let empty = empty_engine
        .derive(plan, empty_generation, limits(16))
        .expect("complete empty closure");
    assert_eq!(
        empty_engine.compare_closure_pairs(&empty, empty_generation, &[]),
        Ok(())
    );
    assert_eq!(
        empty_engine.compare_closure_pairs(&empty, generation, &[]),
        Err(ClosurePairComparisonError::GenerationMismatch {
            expected: generation,
            actual: empty_generation,
        })
    );
}

#[test]
fn compares_the_poo_cyclic_closure_and_withdrawn_snapshot() {
    let (bundle, plan) = fixture();
    let edge = id!(RelationId, 1);
    let mut declaration = bundle.declaration().clone();
    declaration.facts = vec![
        source_fact(100, edge, "1", "2"),
        source_fact(101, edge, "2", "3"),
        source_fact(102, edge, "3", "4"),
        source_fact(103, edge, "4", "2"),
        source_fact(104, edge, "1", "3"),
    ];
    let engine = MrrEngine::builder()
        .with_bundle(ReasoningBundle::admit(declaration.clone()).expect("cyclic bundle"))
        .build()
        .expect("cyclic engine");
    let first_generation = id!(GenerationId, 51);
    let first = engine
        .derive(plan, first_generation, limits(16))
        .expect("complete cyclic closure");
    let pairs = vec![
        ("1".into(), "2".into()),
        ("1".into(), "3".into()),
        ("1".into(), "4".into()),
        ("2".into(), "2".into()),
        ("2".into(), "3".into()),
        ("2".into(), "4".into()),
        ("3".into(), "2".into()),
        ("3".into(), "3".into()),
        ("3".into(), "4".into()),
        ("4".into(), "2".into()),
        ("4".into(), "3".into()),
        ("4".into(), "4".into()),
    ];
    assert_eq!(
        engine.compare_closure_pairs(&first, first_generation, &pairs),
        Ok(())
    );

    declaration
        .facts
        .retain(|fact| fact.id() != id!(FactId, 103));
    let withdrawn = MrrEngine::builder()
        .with_bundle(ReasoningBundle::admit(declaration).expect("withdrawn bundle"))
        .build()
        .expect("withdrawn engine");
    let next_generation = id!(GenerationId, 52);
    let second = withdrawn
        .derive(plan, next_generation, limits(16))
        .expect("complete closure after withdrawal");
    let remaining = vec![
        ("1".into(), "2".into()),
        ("1".into(), "3".into()),
        ("1".into(), "4".into()),
        ("2".into(), "3".into()),
        ("2".into(), "4".into()),
        ("3".into(), "4".into()),
    ];
    assert_eq!(
        withdrawn.compare_closure_pairs(&second, next_generation, &remaining),
        Ok(())
    );
    assert_eq!(
        withdrawn.compare_closure_pairs(&first, first_generation, &pairs),
        Err(ClosurePairComparisonError::SourceBundleMismatch {
            expected: withdrawn.bundle().id(),
            actual: engine.bundle().id(),
        })
    );
}

#[test]
fn admits_complete_candidates_into_exact_lineage_and_transition_outputs() {
    let (bundle, config) = fixture();
    let from = id!(GenerationId, 50);
    let to = id!(GenerationId, 51);
    let receipt =
        evaluate_transitive_closure(&bundle, config, to, limits(16)).expect("closure evaluation");
    let assigned = identities(receipt.closure().candidates().len());

    let materialized = admit_closure_candidates(&bundle, &receipt, from, to, &assigned)
        .expect("complete receipt admission");

    assert_eq!(materialized.transition().from(), from);
    assert_eq!(materialized.transition().to(), to);
    assert_eq!(
        materialized.transition().insertions().len(),
        receipt.closure().candidates().len()
    );
    assert_eq!(materialized.transition().retractions(), &[]);
    assert_eq!(materialized.derivations().len(), assigned.len());
    assert_eq!(materialized.receipt().rule_pack(), id!(RulePackId, 1));
    assert_eq!(receipt.source_bundle(), bundle.id());
    assert_eq!(materialized.receipt().input_generation(), to);
    assert_eq!(
        materialized.receipt().input_fact_ids(),
        receipt.closure().input_fact_ids()
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
        .closure()
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
fn rejects_a_receipt_from_another_bundle_with_the_same_fact_ids() {
    let (bundle, plan) = fixture();
    let to = id!(GenerationId, 51);
    let receipt = evaluate_transitive_closure(&bundle, plan, to, limits(16))
        .expect("complete original closure");
    let mut declaration = bundle.declaration().clone();
    *declaration
        .facts
        .iter_mut()
        .find(|fact| fact.id() == id!(FactId, 100))
        .expect("source fact 100") = source_fact(100, id!(RelationId, 1), "Ada", "Eve");
    let changed = ReasoningBundle::admit(declaration).expect("changed source bundle");
    let error = ClosureAdmissionError::SourceBundleMismatch {
        expected: changed.id(),
        actual: bundle.id(),
    };
    let assigned = identities(receipt.closure().candidates().len());

    assert_eq!(
        admit_closure_candidates(&changed, &receipt, id!(GenerationId, 50), to, &assigned),
        Err(error.clone())
    );
    let engine = MrrEngine::builder()
        .with_bundle(changed)
        .build()
        .expect("changed engine");
    assert_eq!(
        engine.materialize(&receipt, id!(GenerationId, 50), to, &assigned),
        Err(error)
    );
}

#[test]
fn rejects_an_empty_closure_receipt_for_another_generation() {
    let (bundle, plan) = fixture();
    let mut declaration = bundle.declaration().clone();
    declaration.facts.clear();
    let empty = ReasoningBundle::admit(declaration).expect("empty source bundle");
    let evaluated = id!(GenerationId, 51);
    let requested = id!(GenerationId, 52);
    let receipt = evaluate_transitive_closure(&empty, plan, evaluated, limits(16))
        .expect("complete empty closure");
    assert!(receipt.closure().candidates().is_empty());

    assert_eq!(
        admit_closure_candidates(&empty, &receipt, id!(GenerationId, 50), requested, &[]),
        Err(ClosureAdmissionError::GenerationMismatch {
            expected: requested,
            actual: evaluated,
        })
    );
}

#[test]
fn rejects_truncated_or_cross_generation_receipts_before_materialization() {
    let (bundle, config) = fixture();
    let from = id!(GenerationId, 50);
    let evaluated_generation = id!(GenerationId, 51);
    let truncated = evaluate_transitive_closure(&bundle, config, evaluated_generation, limits(1))
        .expect("bounded closure evaluation");
    assert_eq!(
        admit_closure_candidates(
            &bundle,
            &truncated,
            from,
            evaluated_generation,
            &identities(1)
        ),
        Err(ClosureAdmissionError::IncompleteReceipt)
    );

    let complete = evaluate_transitive_closure(&bundle, config, evaluated_generation, limits(16))
        .expect("complete closure evaluation");
    let wrong_target = id!(GenerationId, 52);
    assert_eq!(
        admit_closure_candidates(
            &bundle,
            &complete,
            from,
            wrong_target,
            &identities(complete.closure().candidates().len()),
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
        admit_closure_candidates(&bundle, &receipt, from, to, &[]),
        Err(ClosureAdmissionError::IdentityCountMismatch {
            candidates: receipt.closure().candidates().len(),
            identities: 0,
        })
    );

    let mut duplicate = identities(receipt.closure().candidates().len());
    duplicate[1] = CandidateIdentities::new(duplicate[0].fact(), duplicate[1].derivation());
    assert_eq!(
        admit_closure_candidates(&bundle, &receipt, from, to, &duplicate),
        Err(ClosureAdmissionError::DuplicateFactId(duplicate[0].fact()))
    );

    let mut duplicate = identities(receipt.closure().candidates().len());
    duplicate[1] = CandidateIdentities::new(duplicate[1].fact(), duplicate[0].derivation());
    assert_eq!(
        admit_closure_candidates(&bundle, &receipt, from, to, &duplicate),
        Err(ClosureAdmissionError::DuplicateDerivationId(
            duplicate[0].derivation()
        ))
    );

    assert_eq!(
        admit_closure_candidates(
            &bundle,
            &receipt,
            to,
            to,
            &identities(receipt.closure().candidates().len())
        ),
        Err(ClosureAdmissionError::Transition(
            GenerationTransitionError::SameGeneration
        ))
    );

    let mut self_support = identities(receipt.closure().candidates().len());
    self_support[0] = CandidateIdentities::new(
        receipt.closure().candidates()[0].support()[0],
        self_support[0].derivation(),
    );
    assert_eq!(
        admit_closure_candidates(&bundle, &receipt, from, to, &self_support),
        Err(ClosureAdmissionError::Lineage(LineageError::SelfSupport))
    );
}
