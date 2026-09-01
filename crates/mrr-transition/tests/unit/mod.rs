use core::num::NonZeroUsize;

use crate::{
    Action, ActionId, Effect, Fact, FactId, GenerationId, InitialState, Invariant, Precondition,
    SafetyLimits, SafetyStatus, StatePredicate, StateSchema, StateSnapshot, Transition,
    TransitionError, TransitionModelError, TransitionSystem, check_safety,
};
use mrr_relation::{
    EntityId, EvidenceCompleteness, FactProvenance, FactValidity, RelationAuthority,
    RelationContext, RelationId, Value,
};

mod oracle;

macro_rules! id {
    ($kind:ident, $value:expr) => {
        $kind::from_canonical_bytes(format!("test:{}:{}", stringify!($kind), $value))
            .expect("test identity")
    };
}
#[test]
fn one_transition_cannot_insert_and_retract_the_same_fact() {
    let fact_id = FactId::from_canonical_bytes(b"fact:transition-conflict").expect("fact id");
    let from = GenerationId::from_canonical_bytes(b"generation:before").expect("from generation");
    let to = GenerationId::from_canonical_bytes(b"generation:after").expect("to generation");
    let authority =
        EntityId::from_canonical_bytes(b"authority:transition-test").expect("authority");
    let fact = Fact::new(
        fact_id,
        RelationId::from_canonical_bytes(b"relation:transition-test").expect("relation id"),
        vec![Value::Boolean(true)],
        RelationContext::new(
            to,
            RelationAuthority::Entity(authority),
            FactProvenance::Source(authority),
            EvidenceCompleteness::Complete,
            FactValidity::Valid,
        ),
    );
    assert_eq!(
        Transition::new(from, to, vec![fact], vec![fact_id]),
        Err(TransitionError::ConflictingFact(fact_id))
    );
}

fn model(action: Action, invariant: Invariant) -> TransitionSystem {
    let active = id!(FactId, "active");
    let standby = id!(FactId, "standby");
    let forbidden = id!(FactId, "forbidden");
    TransitionSystem::admit(
        StateSchema::new(vec![active, standby, forbidden]).expect("schema"),
        InitialState::new(StateSnapshot::from_facts(vec![active]).expect("initial")),
        vec![action],
        vec![invariant],
    )
    .expect("transition system")
}

fn limits(states: usize, transitions: usize) -> SafetyLimits {
    SafetyLimits::new(
        NonZeroUsize::new(states).unwrap(),
        NonZeroUsize::new(transitions).unwrap(),
    )
}

#[test]
fn transition_system_rejects_unknown_fact_references() {
    let active = id!(FactId, "active");
    let unknown = id!(FactId, "unknown");
    let system = TransitionSystem::admit(
        StateSchema::new(vec![active]).unwrap(),
        InitialState::new(StateSnapshot::from_facts(vec![active]).unwrap()),
        vec![Action::new(
            id!(ActionId, "bad"),
            Precondition::all(vec![StatePredicate::Present(unknown)]),
            Effect::new(vec![], vec![]).unwrap(),
        )],
        vec![],
    );
    assert_eq!(system, Err(TransitionModelError::UnknownFact(unknown)));
}

#[test]
fn safety_checker_returns_shortest_counterexample_for_unsafe_system() {
    let active = id!(FactId, "active");
    let forbidden = id!(FactId, "forbidden");
    let action = Action::new(
        id!(ActionId, "violate"),
        Precondition::all(vec![StatePredicate::Present(active)]),
        Effect::new(vec![forbidden], vec![]).unwrap(),
    );
    let system = model(
        action,
        Invariant::forbidden_all("never-forbidden", vec![active, forbidden]).unwrap(),
    );

    let receipt = check_safety(&system, limits(8, 8)).expect("finite safety check");
    assert_eq!(receipt.status(), SafetyStatus::Unsafe);
    let counterexample = receipt.counterexample().expect("counterexample");
    assert_eq!(counterexample.states().len(), 2);
    assert_eq!(counterexample.steps().len(), 1);
    assert_eq!(counterexample.violated_invariant(), "never-forbidden");
}

#[test]
fn safe_cycles_terminate_and_state_budget_is_incomplete() {
    let active = id!(FactId, "active");
    let standby = id!(FactId, "standby");
    let to_standby = Action::new(
        id!(ActionId, "to-standby"),
        Precondition::all(vec![StatePredicate::Present(active)]),
        Effect::new(vec![standby], vec![active]).unwrap(),
    );
    let to_active = Action::new(
        id!(ActionId, "to-active"),
        Precondition::all(vec![StatePredicate::Present(standby)]),
        Effect::new(vec![active], vec![standby]).unwrap(),
    );
    let schema = StateSchema::new(vec![active, standby]).unwrap();
    let initial = InitialState::new(StateSnapshot::from_facts(vec![active]).unwrap());
    let invariant = Invariant::required_any("one-mode", vec![active, standby]).unwrap();
    let system = TransitionSystem::admit(
        schema,
        initial,
        vec![to_standby, to_active],
        vec![invariant],
    )
    .unwrap();

    let safe = check_safety(&system, limits(8, 8)).expect("cycle terminates");
    assert_eq!(safe.status(), SafetyStatus::Safe);
    assert_eq!(safe.explored_states(), 2);

    let incomplete = check_safety(&system, limits(1, 8)).expect("bounded check");
    assert_eq!(incomplete.status(), SafetyStatus::Incomplete);
    assert!(incomplete.counterexample().is_none());
}
