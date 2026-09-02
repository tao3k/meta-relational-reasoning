use core::num::NonZeroUsize;

use crate::{
    Atom, GroundAtom, RelationId, Rule, RuleError, RuleId, Term, Variable, WhyNotIncomplete,
    WhyNotLimits, WhyNotStatus, why_not,
};
use mrr_identity::{EntityId, FactId, GenerationId};
use mrr_relation::{
    EvidenceCompleteness, Fact, FactProvenance, FactValidity, RelationAuthority, RelationContext,
    Value,
};

macro_rules! id {
    ($kind:ident, $value:expr) => {
        $kind::from_canonical_bytes(format!("test:{}:{}", stringify!($kind), $value))
            .expect("test identity")
    };
}

fn variable_atom(relation: RelationId) -> Atom {
    Atom {
        relation,
        terms: vec![Term::Variable(Variable::new("x").expect("variable"))],
    }
}

fn ground_atom(relation: RelationId, value: &str) -> Atom {
    Atom {
        relation,
        terms: vec![Term::Value(Value::String(value.into()))],
    }
}

fn fact(relation: RelationId, value: &str, generation: GenerationId) -> Fact {
    let authority = id!(EntityId, value);
    Fact::new(
        id!(FactId, value),
        relation,
        vec![Value::String(value.into())],
        RelationContext::new(
            generation,
            RelationAuthority::Entity(authority),
            FactProvenance::Source(authority),
            EvidenceCompleteness::Complete,
            FactValidity::Valid,
        ),
    )
}

fn limits() -> WhyNotLimits {
    WhyNotLimits::new(
        NonZeroUsize::new(8).unwrap(),
        NonZeroUsize::new(32).unwrap(),
        NonZeroUsize::new(16).unwrap(),
    )
}
#[test]
fn rule_heads_cannot_invent_unbound_variables() {
    let variable = Variable::new("x").expect("variable");
    let head = Atom {
        relation: RelationId::from_canonical_bytes(b"relation:logic-head").expect("relation id"),
        terms: vec![Term::Variable(variable.clone())],
    };
    let body = vec![Atom {
        relation: RelationId::from_canonical_bytes(b"relation:logic-body").expect("relation id"),
        terms: Vec::new(),
    }];
    assert_eq!(
        Rule::new(
            RuleId::from_canonical_bytes(b"rule:unsafe-head").expect("rule id"),
            head,
            body,
        ),
        Err(RuleError::UnsafeHeadVariable(variable))
    );
}

#[test]
fn why_not_distinguishes_proven_one_and_multiple_missing_premises() {
    let generation = id!(GenerationId, 1);
    let goal_relation = id!(RelationId, "goal");
    let premise_a = id!(RelationId, "premise-a");
    let premise_b = id!(RelationId, "premise-b");
    let goal = ground_atom(goal_relation, "x");
    let one_rule = Rule::new(
        id!(RuleId, "one"),
        variable_atom(goal_relation),
        vec![variable_atom(premise_a)],
    )
    .expect("safe rule");

    let proven = why_not(
        &goal,
        generation,
        std::slice::from_ref(&one_rule),
        &[fact(premise_a, "x", generation)],
        limits(),
    )
    .expect("ground analysis");
    assert_eq!(proven.status(), &WhyNotStatus::Proven);

    let one_missing = why_not(&goal, generation, &[one_rule], &[], limits())
        .expect("ground missing-premise analysis");
    assert_eq!(
        one_missing.status(),
        &WhyNotStatus::MissingPremises {
            alternatives: vec![vec![GroundAtom::new(
                premise_a,
                vec![Value::String("x".into())],
            )]],
        }
    );

    let two_rule = Rule::new(
        id!(RuleId, "two"),
        variable_atom(goal_relation),
        vec![variable_atom(premise_a), variable_atom(premise_b)],
    )
    .expect("safe rule");
    let two_missing =
        why_not(&goal, generation, &[two_rule], &[], limits()).expect("two-premise analysis");
    assert_eq!(
        two_missing.status(),
        &WhyNotStatus::MissingPremises {
            alternatives: vec![vec![
                GroundAtom::new(premise_a, vec![Value::String("x".into())]),
                GroundAtom::new(premise_b, vec![Value::String("x".into())]),
            ]],
        }
    );
}

#[test]
fn why_not_reports_no_rule_and_bounds_cycles() {
    let generation = id!(GenerationId, 1);
    let goal_relation = id!(RelationId, "goal");
    let loop_relation = id!(RelationId, "loop");
    let goal = ground_atom(goal_relation, "x");
    let no_rule = why_not(&goal, generation, &[], &[], limits()).expect("bounded analysis");
    assert_eq!(no_rule.status(), &WhyNotStatus::NoApplicableRule);

    let rules = vec![
        Rule::new(
            id!(RuleId, 1),
            variable_atom(goal_relation),
            vec![variable_atom(loop_relation)],
        )
        .expect("goal rule"),
        Rule::new(
            id!(RuleId, 2),
            variable_atom(loop_relation),
            vec![variable_atom(goal_relation)],
        )
        .expect("cycle rule"),
    ];
    let cyclic = why_not(&goal, generation, &rules, &[], limits()).expect("bounded cycle");
    assert_eq!(
        cyclic.status(),
        &WhyNotStatus::Incomplete(WhyNotIncomplete::Cycle)
    );
}
