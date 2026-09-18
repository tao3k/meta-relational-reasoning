use core::num::NonZeroUsize;

use crate::{
    Action, ActionId, CounterexampleFactIdentity, CounterexampleLineageIdentities, Effect, FactId,
    InitialState, Invariant, LineageEdgeId, LineageNodeId, Precondition, SafetyLimits,
    StatePredicate, StateSchema, StateSnapshot, TransitionSystem, counterexample_lineage,
};
use mrr_lineage::why;
use mrr_transition::check_safety;

macro_rules! id {
    ($kind:ident, $value:expr) => {
        $kind::from_canonical_bytes(format!("test:{}:{}", stringify!($kind), $value))
            .expect("test identity")
    };
}

#[test]
fn counterexample_lineage_reaches_every_transition_fact() {
    let active = id!(FactId, "active");
    let forbidden = id!(FactId, "forbidden");
    let system = TransitionSystem::admit(
        StateSchema::new(vec![active, forbidden]).unwrap(),
        InitialState::new(StateSnapshot::from_facts(vec![active]).unwrap()),
        vec![Action::new(
            id!(ActionId, "violate"),
            Precondition::all(vec![StatePredicate::Present(active)]),
            Effect::new(vec![forbidden], vec![]).unwrap(),
        )],
        vec![Invariant::forbidden_all("never-forbidden", vec![active, forbidden]).unwrap()],
    )
    .unwrap();
    let safety = check_safety(
        &system,
        SafetyLimits::new(NonZeroUsize::new(8).unwrap(), NonZeroUsize::new(8).unwrap()),
    )
    .unwrap();
    let counterexample = safety.counterexample().expect("unsafe fixture");
    let violation = id!(LineageNodeId, "violation");
    let identities = CounterexampleLineageIdentities::new(
        violation,
        vec![id!(LineageNodeId, "state-0"), id!(LineageNodeId, "state-1")],
        vec![id!(LineageNodeId, "transition-0")],
        vec![
            CounterexampleFactIdentity::source(active, id!(LineageNodeId, "active")),
            CounterexampleFactIdentity::source(forbidden, id!(LineageNodeId, "forbidden")),
        ],
        (0..5).map(|index| id!(LineageEdgeId, index)).collect(),
    );

    let graph = counterexample_lineage(counterexample, &system, &identities)
        .expect("complete identity-bound counterexample lineage");
    assert_eq!(graph.nodes().len(), 6);
    assert_eq!(graph.edges().len(), 5);
    let explanation = why(&graph, violation).expect("why counterexample");
    assert_eq!(explanation.graph(), &graph);
}
