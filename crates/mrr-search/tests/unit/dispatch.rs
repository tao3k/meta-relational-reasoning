use crate::*;
use mrr_identity::GenerationId;
use std::num::NonZeroUsize;
fn projection() -> PooSearchProjection {
    let stage = |name: &str| PooSearchPlan::Stage {
        name: name.into(),
        role: PooSearchRole::Acquisition,
        input_domain: "workspace".into(),
        output_domain: "candidate-set".into(),
    };
    compile_poo_search_plan(
        "dispatch",
        GenerationId::from_canonical_bytes("dispatch-source").unwrap(),
        &PooSearchPlan::Parallel {
            name: "independent".into(),
            children: vec![stage("a"), stage("b")],
        },
    )
    .unwrap()
}
fn resources(n: usize) -> SearchDispatchResources {
    SearchDispatchResources {
        memory_bytes: n,
        input_bytes: n,
        output_bytes: n,
        results: n,
    }
}
#[test]
fn dispatch_independent_completion_retains_original_factors_and_aggregate_budget() {
    let p = projection();
    let a = p.factor_by_name("a").unwrap();
    let b = p.factor_by_name("b").unwrap();
    let d = SearchDispatch::new(p, NonZeroUsize::new(2).unwrap(), resources(4));
    let first = d.reserve(d.generation(), a, resources(2)).unwrap();
    let second = d.reserve(d.generation(), b, resources(2)).unwrap();
    assert!(matches!(
        d.reserve(d.generation(), a, resources(1)),
        Err(SearchDispatchError::CapacityExceeded)
    ));
    let second = std::thread::spawn(move || second.admit(1, 1).unwrap())
        .join()
        .unwrap();
    assert_eq!(second.factor, b);
    assert_eq!(d.snapshot().unwrap().in_flight, 1);
    let first = first.admit(2, 2).unwrap();
    assert_eq!(first.factor, a);
    let s = d.snapshot().unwrap();
    assert_eq!(s.in_flight, 0);
    assert_eq!(s.reserved, resources(0));
    assert_eq!(s.consumed.input_bytes, 4);
    assert_eq!(s.consumed.output_bytes, 3);
    assert_eq!(s.consumed.results, 3);
    // Capacity release cannot refund the cumulative input or output budget.
    assert!(matches!(
        d.reserve(d.generation(), a, resources(1)),
        Err(SearchDispatchError::CapacityExceeded)
    ));
    println!("SEARCH-DISPATCH-OK completion=b,a inputs=4 output=3 results=3");
}
#[test]
fn retirement_rejects_new_and_late_results_without_reactivating_old_leases() {
    let p = projection();
    let factor = p.factor_by_name("a").unwrap();
    let d = SearchDispatch::new(p.clone(), NonZeroUsize::new(2).unwrap(), resources(8));
    let lease = d.reserve(d.generation(), factor, resources(2)).unwrap();
    d.retire().unwrap();
    d.retire().unwrap();
    assert!(matches!(
        d.reserve(d.generation(), factor, resources(1)),
        Err(SearchDispatchError::Retired)
    ));
    let fresh = SearchDispatch::new(p, NonZeroUsize::new(2).unwrap(), resources(8));
    fresh
        .reserve(fresh.generation(), factor, resources(1))
        .unwrap()
        .admit(1, 1)
        .unwrap();
    assert_eq!(lease.admit(1, 1), Err(SearchDispatchError::Retired));
    let s = d.snapshot().unwrap();
    assert!(s.retired);
    assert_eq!(s.in_flight, 0);
    assert_eq!(s.consumed.output_bytes, 0);
    assert_eq!(s.consumed.results, 0);
    println!("SEARCH-DISPATCH-OK retired=sticky late-result=rejected");
}
#[test]
fn foreign_requests_and_each_resource_overflow_fail_without_mutating_admission() {
    let p = projection();
    let factor = p.factor_by_name("a").unwrap();
    let d = SearchDispatch::new(p, NonZeroUsize::new(2).unwrap(), resources(2));
    let foreign = GenerationId::from_canonical_bytes("foreign").unwrap();
    assert!(matches!(
        d.reserve(foreign, factor, resources(1)),
        Err(SearchDispatchError::GenerationMismatch)
    ));
    let fake =
        SearchFactor::from_canonical_input("foreign", SearchFactorRole::Acquisition).unwrap();
    assert!(matches!(
        d.reserve(d.generation(), fake, resources(1)),
        Err(SearchDispatchError::ForeignFactor)
    ));
    for r in [
        SearchDispatchResources {
            memory_bytes: 3,
            ..resources(1)
        },
        SearchDispatchResources {
            input_bytes: 3,
            ..resources(1)
        },
        SearchDispatchResources {
            output_bytes: 3,
            ..resources(1)
        },
        SearchDispatchResources {
            results: 3,
            ..resources(1)
        },
        resources(usize::MAX),
    ] {
        assert!(matches!(
            d.reserve(d.generation(), factor, r),
            Err(SearchDispatchError::CapacityExceeded)
        ));
        assert_eq!(d.snapshot().unwrap(), SearchDispatchSnapshot::default());
    }
    let lease = d.reserve(d.generation(), factor, resources(1)).unwrap();
    assert_eq!(
        lease.admit(2, 1),
        Err(SearchDispatchError::ReservationExceeded)
    );
    assert_eq!(d.snapshot().unwrap().in_flight, 0);
    assert_eq!(d.snapshot().unwrap().consumed.output_bytes, 0);
    // A cancelled/dropped future releases reserved resources, but not charged input.
    drop(d.reserve(d.generation(), factor, resources(1)).unwrap());
    assert_eq!(d.snapshot().unwrap().reserved, resources(0));
}
#[test]
fn dependent_acquisition_must_return_through_scheme_continuation() {
    let stage = |name: &str| PooSearchPlan::Stage {
        name: name.into(),
        role: PooSearchRole::Acquisition,
        input_domain: "workspace".into(),
        output_domain: "workspace".into(),
    };
    let p = compile_poo_search_plan(
        "dependent",
        GenerationId::from_canonical_bytes("source").unwrap(),
        &PooSearchPlan::Chain {
            name: "chain".into(),
            children: vec![stage("a"), stage("b")],
        },
    )
    .unwrap();
    let b = p.factor_by_name("b").unwrap();
    let d = SearchDispatch::new(p, NonZeroUsize::new(1).unwrap(), resources(2));
    assert!(matches!(
        d.reserve(d.generation(), b, resources(1)),
        Err(SearchDispatchError::UnsupportedPrerequisites)
    ));
}
