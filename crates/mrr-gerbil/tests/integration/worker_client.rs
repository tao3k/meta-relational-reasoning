//! Same typed Scheme services without linking the producer program into the host.
#![cfg(not(feature = "embedded-runtime"))]
use mrr_gerbil::{
    FiniteInferenceError, PooSearchPlan, PooSearchRole, TemporalHost, TemporalRuntimeError,
    configure_native_worker, evaluate_finite_relations, project_poo_search_strategy,
    shutdown_native_worker,
};

#[test]
fn transport_only_host_executes_existing_poo_owner_and_finite_solver() {
    assert_eq!(
        TemporalHost.refresh_policy(b"()"),
        Err(TemporalRuntimeError::RuntimeUnavailable)
    );
    assert_eq!(
        evaluate_finite_relations(1, vec![], vec![]),
        Err(FiniteInferenceError::RuntimeUnavailable)
    );
    println!("CASE unconfigured transport fails closed");
    // The producer build is an explicit prerequisite in this Cargo artifact
    // directory; this test never discovers or builds an executable on PATH.
    let executable = std::env::current_exe().unwrap();
    let worker = executable
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("mrr-native-worker");
    assert!(
        worker.is_file(),
        "build the producer mrr-native-worker before this test"
    );
    configure_native_worker(&worker).unwrap();
    let plan = PooSearchPlan::Stage {
        name: "source".into(),
        role: PooSearchRole::Acquisition,
        input_domain: "workspace".into(),
        output_domain: "candidates".into(),
    };
    let graph = project_poo_search_strategy("worker-client", &"a".repeat(64), &plan).unwrap();
    assert_eq!(graph.factors.len(), 1);
    assert!(!graph.dag_receipt.is_empty());
    println!("CASE real POO projection across existing V1 transport");
    let result = evaluate_finite_relations(3, vec![(0, 1), (1, 2)], vec![0]).unwrap();
    assert!(result.paths.contains(&(0, 2, 2)));
    assert_eq!(
        evaluate_finite_relations(2, vec![(0, 2)], vec![]),
        Err(FiniteInferenceError::ForeignNode)
    );
    println!("CASE Scheme finite inference and invalid coordinate rejection");
    shutdown_native_worker().unwrap();
    assert!(TemporalHost.refresh_policy(b"()").is_err());
    println!("CASE terminal owner shutdown without embedded fallback");
}
