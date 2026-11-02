use crate::{FiniteInferenceError, evaluate_finite_relations};

#[test]
fn finite_native_preserves_shortest_distance_and_original_observation_order() {
    let candidate = evaluate_finite_relations(4, vec![(0, 1), (1, 2), (2, 3), (0, 3)], vec![0, 2])
        .expect("native Scheme inference");
    assert!(candidate.paths.contains(&(0, 3, 1)));
    assert!(candidate.influences.contains(&(0, 0, 3, 1)));
    assert!(candidate.influences.contains(&(1, 2, 3, 1)));
    assert!(!candidate.influences.contains(&(0, 0, 3, 3)));
}

#[test]
fn finite_native_releases_request_state_and_rejects_foreign_coordinates() {
    assert_eq!(
        evaluate_finite_relations(2, vec![(0, 2)], vec![]),
        Err(FiniteInferenceError::ForeignNode)
    );
    let cycle = evaluate_finite_relations(2, vec![(0, 1), (1, 0)], vec![]).expect("cycle");
    assert!(cycle.paths.contains(&(0, 0, 2)));
    let empty = evaluate_finite_relations(0, vec![], vec![]).expect("new empty request");
    assert!(empty.paths.is_empty());
    assert!(empty.influences.is_empty());
}

#[test]
fn finite_native_serializes_independent_concurrent_requests() {
    let workers: Vec<_> = (0..4)
        .map(|node| {
            std::thread::spawn(move || {
                let candidate = evaluate_finite_relations(5, vec![(node, node + 1)], vec![node])
                    .expect("owner-thread operation");
                assert_eq!(candidate.paths, vec![(node, node + 1, 1)]);
                assert_eq!(candidate.influences.len(), 2);
            })
        })
        .collect();
    for worker in workers {
        worker.join().expect("request worker");
    }
}
