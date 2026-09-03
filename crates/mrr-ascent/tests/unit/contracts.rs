use core::num::NonZeroUsize;

use mrr_ascent::{
    ClosureConfig, ClosureError, ClosureLimits, ClosureStatus, evaluate_transitive_closure,
};
use mrr_bundle::{ReasoningBundle, ReasoningBundleDeclaration, RulePack};
use mrr_identity::{EntityId, FactId, GenerationId, RelationId, RuleId, RulePackId};
use mrr_logic::Rule;
use mrr_query::{Atom, Term, Variable};
use mrr_relation::{
    EvidenceCompleteness, Fact, FactProvenance, FactValidity, RelationAuthority,
    RelationCardinality, RelationContext, RelationField, RelationSchema, Value, ValueType,
};

macro_rules! id {
    ($kind:ident, $value:expr) => {
        $kind::from_canonical_bytes(format!("test:{}:{}", stringify!($kind), $value))
            .expect("test identity input is canonical")
    };
}

fn variable(name: &str) -> Variable {
    Variable::new(name).expect("test variables are valid")
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
    .expect("binary relation schema")
}

fn source_fact(identity: u128, relation: RelationId, from: &str, to: &str) -> Fact {
    let authority = id!(EntityId, identity);
    Fact::new(
        id!(FactId, identity),
        relation,
        vec![Value::String(from.into()), Value::String(to.into())],
        RelationContext::new(
            id!(GenerationId, 900),
            RelationAuthority::Entity(authority),
            FactProvenance::Source(authority),
            EvidenceCompleteness::Complete,
            FactValidity::Valid,
        ),
    )
}

fn admitted_bundle(
    relations: Vec<RelationSchema>,
    facts: Vec<Fact>,
    rules: Vec<Rule>,
) -> ReasoningBundle {
    ReasoningBundle::admit(ReasoningBundleDeclaration {
        relations,
        facts,
        rule_packs: vec![RulePack::new(id!(RulePackId, 1), rules)],
        ..ReasoningBundleDeclaration::default()
    })
    .expect("fixture bundle admission")
}

fn fixture_bundle() -> (ReasoningBundle, ClosureConfig) {
    let edge = id!(RelationId, 1);
    let reachable = id!(RelationId, 2);
    let base_rule = id!(RuleId, 10);
    let transitive_rule = id!(RuleId, 11);

    let relations = vec![
        binary_schema(edge, "edge"),
        binary_schema(reachable, "reachable"),
    ];
    let facts = vec![
        source_fact(100, edge, "Ada", "Bob"),
        source_fact(101, edge, "Bob", "Cy"),
        source_fact(99, edge, "Ada", "Dan"),
        source_fact(103, edge, "Dan", "Cy"),
    ];
    let rules = vec![
        Rule::new(
            base_rule,
            atom(reachable, &["x", "y"]),
            vec![atom(edge, &["x", "y"])],
        )
        .expect("safe base rule"),
        Rule::new(
            transitive_rule,
            atom(reachable, &["x", "z"]),
            vec![atom(reachable, &["x", "y"]), atom(edge, &["y", "z"])],
        )
        .expect("safe transitive rule"),
    ];

    (
        admitted_bundle(relations, facts, rules),
        ClosureConfig::new(
            edge,
            reachable,
            id!(RulePackId, 1),
            base_rule,
            transitive_rule,
        ),
    )
}

fn limits(input: usize, pairs: usize, results: usize) -> ClosureLimits {
    ClosureLimits::new(
        NonZeroUsize::new(input).unwrap(),
        NonZeroUsize::new(pairs).unwrap(),
        NonZeroUsize::new(results).unwrap(),
    )
}

#[test]
fn derives_deterministic_shortest_lineage_candidates_from_a_validated_bundle() {
    let (bundle, config) = fixture_bundle();
    let generation = id!(GenerationId, 900);

    let receipt = evaluate_transitive_closure(&bundle, config, generation, limits(16, 64, 64))
        .expect("validated closure must execute");

    assert_eq!(receipt.status(), ClosureStatus::Complete);
    assert_eq!(receipt.rule_pack(), id!(RulePackId, 1));
    assert_eq!(receipt.input_generation(), generation);
    assert_eq!(receipt.input_fact_count(), 4);
    let mut expected_input_ids = vec![
        id!(FactId, 99),
        id!(FactId, 100),
        id!(FactId, 101),
        id!(FactId, 103),
    ];
    expected_input_ids.sort_unstable();
    assert_eq!(receipt.input_fact_ids(), expected_input_ids);
    assert_eq!(receipt.candidates().len(), 5);
    let ada_to_cy = receipt
        .candidates()
        .iter()
        .find(|candidate| {
            *candidate.values() == [Value::String("Ada".into()), Value::String("Cy".into())]
        })
        .expect("transitive candidate");
    assert_eq!(ada_to_cy.relation(), id!(RelationId, 2));
    assert_eq!(ada_to_cy.rule(), id!(RuleId, 11));
    assert_eq!(ada_to_cy.generation(), generation);
    assert_eq!(
        ada_to_cy.support(),
        &std::cmp::min(
            [id!(FactId, 99), id!(FactId, 103)],
            [id!(FactId, 100), id!(FactId, 101)],
        )
    );
}

#[test]
fn repeated_evaluation_is_identity_and_digest_idempotent() {
    let (bundle, config) = fixture_bundle();
    let generation = id!(GenerationId, 900);

    let first = evaluate_transitive_closure(&bundle, config, generation, limits(16, 64, 64))
        .expect("first evaluation");
    let second = evaluate_transitive_closure(&bundle, config, generation, limits(16, 64, 64))
        .expect("second evaluation");

    assert_eq!(first, second);
    assert_eq!(first.digest(), second.digest());
}

#[test]
fn truncates_only_the_published_receipt_after_computing_a_bounded_closure() {
    let (bundle, config) = fixture_bundle();
    let receipt =
        evaluate_transitive_closure(&bundle, config, id!(GenerationId, 900), limits(16, 64, 2))
            .expect("closure fits its execution budgets");

    assert_eq!(receipt.status(), ClosureStatus::OutputTruncated);
    assert_eq!(receipt.candidates().len(), 2);
    assert_eq!(
        receipt.candidates()[0].values(),
        &[Value::String("Ada".into()), Value::String("Bob".into()),]
    );
}

#[test]
fn rejects_input_budget_before_running_ascent() {
    let (bundle, config) = fixture_bundle();
    let error =
        evaluate_transitive_closure(&bundle, config, id!(GenerationId, 900), limits(3, 64, 64))
            .expect_err("four source facts exceed a three-fact budget");

    assert_eq!(
        error,
        ClosureError::InputFactBudgetExceeded {
            required: 4,
            limit: 3,
        }
    );
}

#[test]
fn rejects_theoretical_pair_budget_before_running_ascent() {
    let (bundle, config) = fixture_bundle();
    let error =
        evaluate_transitive_closure(&bundle, config, id!(GenerationId, 900), limits(16, 4, 64))
            .expect_err("four nodes require a sixteen-pair capacity");

    assert_eq!(
        error,
        ClosureError::DerivedPairBudgetExceeded {
            required: 16,
            limit: 4,
        }
    );
}

#[test]
fn rejects_a_safe_rule_that_is_not_the_owned_transitive_shape() {
    let (bundle, config) = fixture_bundle();
    let mut declaration = bundle.declaration().clone();
    let malformed = Rule::new(
        id!(RuleId, 11),
        atom(id!(RelationId, 2), &["x", "z"]),
        vec![
            atom(id!(RelationId, 1), &["x", "y"]),
            atom(id!(RelationId, 1), &["y", "z"]),
        ],
    )
    .expect("the malformed adapter rule is still a safe generic MRR rule");
    let base = declaration.rule_packs[0].rules()[0].clone();
    declaration.rule_packs = vec![RulePack::new(id!(RulePackId, 1), vec![base, malformed])];
    let bundle = ReasoningBundle::admit(declaration).expect("generic rule is bundle-valid");

    let error =
        evaluate_transitive_closure(&bundle, config, id!(GenerationId, 900), limits(16, 64, 64))
            .expect_err("the adapter must not reinterpret an unsupported generic rule");

    assert_eq!(
        error,
        ClosureError::RuleShapeMismatch {
            rule: id!(RuleId, 11),
        }
    );
}

#[test]
fn rejects_rules_outside_the_configured_rule_pack() {
    let (bundle, config) = fixture_bundle();
    let mut declaration = bundle.declaration().clone();
    let base = declaration.rule_packs[0].rules()[0].clone();
    let transitive = declaration.rule_packs[0].rules()[1].clone();
    declaration.rule_packs[0] = RulePack::new(id!(RulePackId, 1), vec![base]);
    declaration
        .rule_packs
        .push(RulePack::new(id!(RulePackId, 2), vec![transitive]));
    let bundle = ReasoningBundle::admit(declaration).expect("two valid rule packs");

    let error =
        evaluate_transitive_closure(&bundle, config, id!(GenerationId, 900), limits(16, 64, 64))
            .expect_err("the configured rule pack is the authority boundary");

    assert_eq!(
        error,
        ClosureError::RuleNotInPack {
            rule: id!(RuleId, 11),
            rule_pack: id!(RulePackId, 1),
        }
    );
}

#[test]
fn derives_cycle_reflexivity_with_a_finite_shortest_support() {
    let (bundle, config) = fixture_bundle();
    let mut declaration = bundle.declaration().clone();
    declaration
        .facts
        .push(source_fact(104, id!(RelationId, 1), "Cy", "Ada"));
    let bundle = ReasoningBundle::admit(declaration).expect("cyclic bundle admission");

    let receipt =
        evaluate_transitive_closure(&bundle, config, id!(GenerationId, 900), limits(16, 64, 64))
            .expect("finite-domain lattice evaluation must stabilize on cycles");
    let ada_to_ada = receipt
        .candidates()
        .iter()
        .find(|candidate| {
            *candidate.values() == [Value::String("Ada".into()), Value::String("Ada".into())]
        })
        .expect("cycle derives reflexive reachability");

    assert_eq!(
        ada_to_ada.support(),
        &std::cmp::min(
            [id!(FactId, 99), id!(FactId, 103), id!(FactId, 104)],
            [id!(FactId, 100), id!(FactId, 101), id!(FactId, 104)],
        )
    );
}
