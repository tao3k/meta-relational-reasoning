//! Same typed Scheme services without linking the producer program into the host.
#![cfg(not(feature = "embedded-runtime"))]
use mrr_gerbil::{
    FiniteInferenceError, PooSearchController, PooSearchPlan, PooSearchRole, TemporalHost,
    TemporalRuntimeError, configure_native_worker, evaluate_finite_relations,
    project_poo_search_strategy, shutdown_native_worker,
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
    {
        let controller =
            PooSearchController::new("worker-controller", &"a".repeat(64), "config", "cut", &plan)
                .unwrap();
        let old = controller.issue("source").unwrap();
        assert!(controller.frontier().unwrap().is_empty());
        assert_eq!(
            controller.issue("source").unwrap_err(),
            TemporalRuntimeError::NativeRejected(4)
        );
        controller.revise(&["source".into()], "cut2").unwrap();
        let fresh = controller.issue("source").unwrap();
        assert_eq!(
            controller.complete(&old).unwrap_err(),
            TemporalRuntimeError::NativeRejected(4)
        );
        assert!(controller.cancel(&old).unwrap().is_empty());
        controller.complete(&fresh).unwrap();
        assert!(controller.frontier().unwrap().is_empty());
    }
    println!("CASE retained POO controller and stale-attempt isolation over V1 worker transport");
    {
        use mrr_gerbil::{PooSearchModality, PooSearchObservation};
        let generation = "b".repeat(64);
        let evidence = PooSearchController::with_evidence(
            "worker-evidence",
            &generation,
            "config",
            "cut",
            &plan,
        )
        .unwrap();
        let request = evidence.issue("source").unwrap();
        assert!(evidence.inputs(&request).unwrap().is_empty());
        assert_eq!(
            evidence.complete(&request),
            Err(TemporalRuntimeError::NativeRejected(4))
        );
        let event = PooSearchObservation {
            identity: "worker-event".into(),
            generation,
            stage: "source".into(),
            source_cut: "cut".into(),
            logical_position: 1,
            payload_identity: "candidate".into(),
            causal_parents: vec![],
            modality: PooSearchModality::Observed,
            committed: true,
        };
        for invalid in [
            PooSearchObservation {
                committed: false,
                ..event.clone()
            },
            PooSearchObservation {
                source_cut: "foreign-cut".into(),
                ..event.clone()
            },
            PooSearchObservation {
                causal_parents: vec!["missing-parent".into()],
                ..event.clone()
            },
            PooSearchObservation {
                modality: PooSearchModality::Hypothesized,
                ..event.clone()
            },
        ] {
            assert_eq!(
                evidence.observe(&request, &invalid),
                Err(TemporalRuntimeError::NativeRejected(4))
            );
            assert!(evidence.inputs(&request).unwrap().is_empty());
        }
        evidence.observe(&request, &event).unwrap();
        evidence.revise(&["source".into()], "cut2").unwrap();
        let fresh = evidence.issue("source").unwrap();
        let mut revised = event.clone();
        revised.source_cut = "cut2".into();
        revised.logical_position = 2;
        assert_eq!(
            evidence.observe(&fresh, &revised),
            Err(TemporalRuntimeError::NativeRejected(4))
        );
        revised.identity = "worker-event-2".into();
        evidence.observe(&fresh, &revised).unwrap();
    }
    println!("CASE actual Temporal observation and replay rejection over V1 worker transport");
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
