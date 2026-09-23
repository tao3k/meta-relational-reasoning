use std::num::NonZeroUsize;

use mrr_identity::{FactId, GenerationId, QueryOperatorId};

use crate::{
    SearchFactor, SearchFactorEdge, SearchFactorRole, SearchFrameworkError, SearchFrameworkLimits,
    SearchFrameworkStatus, SearchObservation, evaluate_search_factors,
};

fn id<T>(value: &str) -> T
where
    T: CanonicalId,
{
    T::from_bytes(value.as_bytes())
}

trait CanonicalId {
    fn from_bytes(bytes: &[u8]) -> Self;
}

macro_rules! canonical_id {
    ($type:ty) => {
        impl CanonicalId for $type {
            fn from_bytes(bytes: &[u8]) -> Self {
                <$type>::from_canonical_bytes(bytes).expect("test identity")
            }
        }
    };
}

canonical_id!(FactId);
canonical_id!(GenerationId);
canonical_id!(QueryOperatorId);

fn limits() -> SearchFrameworkLimits {
    SearchFrameworkLimits::new(
        NonZeroUsize::new(8).unwrap(),
        NonZeroUsize::new(16).unwrap(),
        NonZeroUsize::new(32).unwrap(),
        NonZeroUsize::new(128).unwrap(),
        NonZeroUsize::new(128).unwrap(),
    )
}

#[test]
fn derives_factor_influence_trajectory_and_reusable_impact() {
    let acquire = id::<QueryOperatorId>("factor:acquire");
    let refine = id::<QueryOperatorId>("factor:refine");
    let reason = id::<QueryOperatorId>("factor:reason");
    let candidate = id::<FactId>("candidate:one");
    let acquired = id::<FactId>("event:acquired");
    let refined = id::<FactId>("event:refined");
    let factors = [
        SearchFactor::new(acquire, SearchFactorRole::Acquisition),
        SearchFactor::new(refine, SearchFactorRole::Refinement),
        SearchFactor::new(reason, SearchFactorRole::Reasoning),
    ];
    let edges = [
        SearchFactorEdge::new(acquire, refine),
        SearchFactorEdge::new(refine, reason),
    ];
    let observations = [
        SearchObservation::new(acquired, candidate, acquire, 1, vec![]),
        SearchObservation::new(refined, candidate, refine, 2, vec![acquired]),
    ];

    let receipt = evaluate_search_factors(
        id::<GenerationId>("generation:one"),
        &factors,
        &edges,
        &observations,
        limits(),
    )
    .expect("bounded factor reasoning");

    assert_eq!(receipt.status(), SearchFrameworkStatus::Complete);
    let reason_influence = receipt
        .influences()
        .iter()
        .find(|influence| influence.factor() == reason && influence.support_event() == acquired)
        .expect("acquisition reaches reasoning");
    assert_eq!(reason_influence.factor_path(), &[acquire, refine, reason]);
    let impact = receipt.impact(acquired).expect("source observation Impact");
    assert!(!impact.directly_invalidated().is_empty());
    assert!(!impact.transitively_invalidated().is_empty());

    let reordered = evaluate_search_factors(
        id::<GenerationId>("generation:one"),
        &[factors[2], factors[1], factors[0]],
        &[edges[1], edges[0]],
        &[observations[1].clone(), observations[0].clone()],
        limits(),
    )
    .expect("input order must not change the receipt");
    assert_eq!(receipt.digest(), reordered.digest());

    let later_trace = evaluate_search_factors(
        id::<GenerationId>("generation:one"),
        &factors,
        &edges,
        &[
            observations[0].clone(),
            SearchObservation::new(refined, candidate, refine, 3, vec![acquired]),
        ],
        limits(),
    )
    .expect("later logical position remains causal");
    assert_ne!(receipt.digest(), later_trace.digest());
}

#[test]
fn rejects_missing_temporal_parent_and_factor_edge() {
    let acquire = id::<QueryOperatorId>("factor:acquire");
    let refine = id::<QueryOperatorId>("factor:refine");
    let candidate = id::<FactId>("candidate:one");
    let missing = id::<FactId>("event:missing");
    let refined = id::<FactId>("event:refined");
    let factors = [
        SearchFactor::new(acquire, SearchFactorRole::Acquisition),
        SearchFactor::new(refine, SearchFactorRole::Refinement),
    ];
    let error = evaluate_search_factors(
        id::<GenerationId>("generation:one"),
        &factors,
        &[SearchFactorEdge::new(acquire, refine)],
        &[SearchObservation::new(
            refined,
            candidate,
            refine,
            2,
            vec![missing],
        )],
        limits(),
    )
    .expect_err("missing temporal parent must fail");
    assert_eq!(
        error,
        SearchFrameworkError::MissingCausalParent {
            observation: refined,
            parent: missing,
        }
    );

    let acquired = id::<FactId>("event:acquired");
    let error = evaluate_search_factors(
        id::<GenerationId>("generation:one"),
        &factors,
        &[],
        &[
            SearchObservation::new(acquired, candidate, acquire, 1, vec![]),
            SearchObservation::new(refined, candidate, refine, 2, vec![acquired]),
        ],
        limits(),
    )
    .expect_err("causal parent requires a declared factor edge");
    assert_eq!(
        error,
        SearchFrameworkError::CausalFactorEdgeMissing {
            observation: refined,
            parent: acquired,
        }
    );
}

#[test]
fn rejects_factor_cycles_before_publishing_a_receipt() {
    let first = id::<QueryOperatorId>("factor:first");
    let second = id::<QueryOperatorId>("factor:second");
    let factors = [
        SearchFactor::new(first, SearchFactorRole::Acquisition),
        SearchFactor::new(second, SearchFactorRole::Refinement),
    ];
    let error = evaluate_search_factors(
        id::<GenerationId>("generation:one"),
        &factors,
        &[
            SearchFactorEdge::new(first, second),
            SearchFactorEdge::new(second, first),
        ],
        &[],
        limits(),
    )
    .expect_err("factor cycle must fail");
    assert_eq!(error, SearchFrameworkError::FactorCycle);
}

#[test]
fn bounds_inference_before_execution_and_marks_publication_truncation() {
    let acquire = id::<QueryOperatorId>("factor:acquire");
    let refine = id::<QueryOperatorId>("factor:refine");
    let event = id::<FactId>("event:acquired");
    let candidate = id::<FactId>("candidate:one");
    let factors = [
        SearchFactor::new(acquire, SearchFactorRole::Acquisition),
        SearchFactor::new(refine, SearchFactorRole::Refinement),
    ];
    let edges = [SearchFactorEdge::new(acquire, refine)];
    let observations = [SearchObservation::new(event, candidate, acquire, 1, vec![])];
    let truncated = SearchFrameworkLimits::new(
        NonZeroUsize::new(2).unwrap(),
        NonZeroUsize::new(1).unwrap(),
        NonZeroUsize::new(1).unwrap(),
        NonZeroUsize::new(2).unwrap(),
        NonZeroUsize::new(1).unwrap(),
    );
    let receipt = evaluate_search_factors(
        id::<GenerationId>("generation:one"),
        &factors,
        &edges,
        &observations,
        truncated,
    )
    .expect("fixed point fits, publication truncates explicitly");
    assert_eq!(receipt.status(), SearchFrameworkStatus::OutputTruncated);
    assert_eq!(receipt.total_influence_count(), 2);
    assert_eq!(receipt.influences().len(), 1);

    let too_small = SearchFrameworkLimits::new(
        NonZeroUsize::new(2).unwrap(),
        NonZeroUsize::new(1).unwrap(),
        NonZeroUsize::new(1).unwrap(),
        NonZeroUsize::new(1).unwrap(),
        NonZeroUsize::new(1).unwrap(),
    );
    assert_eq!(
        evaluate_search_factors(
            id::<GenerationId>("generation:one"),
            &factors,
            &edges,
            &observations,
            too_small,
        ),
        Err(SearchFrameworkError::BudgetExceeded {
            resource: "influences",
            required: 2,
            limit: 1,
        })
    );
}
