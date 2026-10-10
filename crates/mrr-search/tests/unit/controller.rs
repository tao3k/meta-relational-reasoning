use crate::{
    PooSearchPlan, PooSearchRole, SearchDispatch, SearchDispatchError, SearchDispatchResources,
};
use mrr_identity::GenerationId;
use std::num::NonZeroUsize;
fn stage(name: &str) -> PooSearchPlan {
    PooSearchPlan::Stage {
        name: name.into(),
        role: PooSearchRole::Acquisition,
        input_domain: "rows".into(),
        output_domain: "rows".into(),
    }
}
fn resources(n: usize) -> SearchDispatchResources {
    SearchDispatchResources {
        memory_bytes: n,
        input_bytes: n,
        output_bytes: n,
        results: n,
    }
}
fn chain() -> SearchDispatch {
    SearchDispatch::from_plan(
        "controlled",
        GenerationId::from_canonical_bytes("controller-source").unwrap(),
        &PooSearchPlan::Chain {
            name: "chain".into(),
            children: vec![stage("a"), stage("b")],
        },
        "config",
        "cut",
        NonZeroUsize::new(2).unwrap(),
        resources(16),
    )
    .unwrap()
}
#[test]
fn controlled_chain_admits_dependent_work_only_after_scheme_completion() {
    let d = chain();
    let a = d.factor_by_name("a").unwrap();
    let b = d.factor_by_name("b").unwrap();
    assert_eq!(d.frontier().unwrap(), [a]);
    assert!(matches!(
        d.reserve(d.generation(), b, resources(1)),
        Err(SearchDispatchError::ControllerRejected)
    ));
    assert_eq!(d.snapshot().unwrap().consumed.input_bytes, 0);
    let a_lease = d.reserve(d.generation(), a, resources(1)).unwrap();
    assert!(d.frontier().unwrap().is_empty());
    assert_eq!(a_lease.admit(1, 1).unwrap().attempt, Some((0, 0)));
    assert_eq!(d.frontier().unwrap(), [b]);
    d.reserve(d.generation(), b, resources(1))
        .unwrap()
        .admit(1, 1)
        .unwrap();
    assert!(d.frontier().unwrap().is_empty());
    println!("SEARCH-CONTROLLER-OK chain=a,b owner=Scheme");
}
#[test]
fn dropped_physical_lease_reopens_through_scheme_revision() {
    let d = chain();
    let a = d.factor_by_name("a").unwrap();
    drop(d.reserve(d.generation(), a, resources(1)).unwrap());
    assert_eq!(d.snapshot().unwrap().in_flight, 0);
    assert_eq!(d.frontier().unwrap(), [a]);
    let receipt = d
        .reserve(d.generation(), a, resources(1))
        .unwrap()
        .admit(1, 1)
        .unwrap();
    assert_eq!(receipt.attempt, Some((1, 1)));
    assert_eq!(d.snapshot().unwrap().consumed.input_bytes, 2);
    println!("SEARCH-CONTROLLER-OK cancelled attempt fenced and input not refunded");
}
#[test]
fn rejected_revision_preserves_the_live_owner_and_active_attempt() {
    let d = chain();
    let a = d.factor_by_name("a").unwrap();
    let lease = d.reserve(d.generation(), a, resources(1)).unwrap();
    assert!(matches!(
        d.revise(&[a], ""),
        Err(SearchDispatchError::ControllerRejected)
    ));
    assert!(!d.snapshot().unwrap().retired);
    assert!(d.frontier().unwrap().is_empty());
    assert_eq!(lease.admit(1, 1).unwrap().attempt, Some((0, 0)));
    println!("SEARCH-CONTROLLER-OK rejected revision preserves active request");
}
#[test]
fn stale_completion_and_old_cancellation_cannot_change_new_attempt() {
    let d = chain();
    let a = d.factor_by_name("a").unwrap();
    let old = d.reserve(d.generation(), a, resources(1)).unwrap();
    d.revise(&[a], "cut2").unwrap();
    let fresh = d.reserve(d.generation(), a, resources(1)).unwrap();
    assert!(matches!(
        old.admit(1, 1),
        Err(SearchDispatchError::ControllerRejected)
    ));
    assert!(d.frontier().unwrap().is_empty());
    assert_eq!(fresh.admit(1, 1).unwrap().attempt, Some((1, 1)));
    assert_eq!(d.snapshot().unwrap().consumed.output_bytes, 1);
    println!("SEARCH-CONTROLLER-OK old completion and cancellation rejected");
}
#[test]
fn controlled_fan_in_retains_independent_branch_after_revision() {
    let plan = PooSearchPlan::Merge {
        name: "join".into(),
        parallel: Box::new(PooSearchPlan::Parallel {
            name: "roots".into(),
            children: vec![stage("a"), stage("b")],
        }),
        stage_name: "c".into(),
        role: PooSearchRole::Projection,
        output_domain: "results".into(),
    };
    let d = SearchDispatch::from_plan(
        "fan-in",
        GenerationId::from_canonical_bytes("fan-in-source").unwrap(),
        &plan,
        "config",
        "cut",
        NonZeroUsize::new(2).unwrap(),
        resources(16),
    )
    .unwrap();
    let a = d.factor_by_name("a").unwrap();
    let b = d.factor_by_name("b").unwrap();
    let c = d.factor_by_name("c").unwrap();
    d.reserve(d.generation(), b, resources(1))
        .unwrap()
        .admit(1, 1)
        .unwrap();
    d.reserve(d.generation(), a, resources(1))
        .unwrap()
        .admit(1, 1)
        .unwrap();
    assert_eq!(d.frontier().unwrap(), [c]);
    d.revise(&[a], "cut2").unwrap();
    assert_eq!(d.frontier().unwrap(), [a]);
    d.reserve(d.generation(), a, resources(1))
        .unwrap()
        .admit(1, 1)
        .unwrap();
    assert_eq!(d.frontier().unwrap(), [c]);
    d.reserve(d.generation(), c, resources(1))
        .unwrap()
        .admit(1, 1)
        .unwrap();
    assert!(d.frontier().unwrap().is_empty());
    println!("SEARCH-CONTROLLER-OK fan-in retained B and reran A/C");
}
#[test]
fn retirement_and_resource_overflow_cannot_complete_logical_work() {
    let d = chain();
    let a = d.factor_by_name("a").unwrap();
    let lease = d.reserve(d.generation(), a, resources(1)).unwrap();
    assert!(matches!(
        lease.admit(2, 1),
        Err(SearchDispatchError::ReservationExceeded)
    ));
    assert_eq!(d.frontier().unwrap(), [a]);
    let lease = d.reserve(d.generation(), a, resources(1)).unwrap();
    d.retire().unwrap();
    assert!(matches!(
        lease.admit(1, 1),
        Err(SearchDispatchError::Retired)
    ));
    assert!(matches!(d.frontier(), Err(SearchDispatchError::Retired)));
    assert_eq!(d.snapshot().unwrap().consumed.output_bytes, 0);
    println!("SEARCH-CONTROLLER-OK retirement and oversize fail before Scheme completion");
}

#[test]
fn controller_sessions_reject_foreign_and_duplicate_completions() {
    let generation = GenerationId::from_canonical_bytes("isolated-controller")
        .unwrap()
        .to_string();
    let first = mrr_gerbil::PooSearchController::new(
        "same-plan",
        &generation,
        "config",
        "cut",
        &stage("a"),
    )
    .unwrap();
    let second = mrr_gerbil::PooSearchController::new(
        "same-plan",
        &generation,
        "config",
        "cut",
        &stage("a"),
    )
    .unwrap();
    let first_request = first.issue("a").unwrap();
    let second_request = second.issue("a").unwrap();
    assert!(second.complete(&first_request).is_err());
    assert!(second.frontier().unwrap().is_empty());
    second.complete(&second_request).unwrap();
    assert!(second.complete(&second_request).is_err());
    first.complete(&first_request).unwrap();
    println!("SEARCH-CONTROLLER-OK distinct sessions and once-only completion");
}
