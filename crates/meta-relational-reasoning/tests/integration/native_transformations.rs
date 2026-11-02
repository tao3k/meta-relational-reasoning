//! Runs the actual Scheme ASCENT owner, then the native finite runtime.
//! No mocked native relation evaluator or automatic fallback is installed.
pub use meta_relational_reasoning::{
    ClosureStatus, ExternalRevisionIdentity, FiniteProblem, FiniteTransformationCatalog,
    FiniteTransport, GenerationId, NativeTransformationExecution,
    NativeTransformationExecutionRequest, RevisionBinding, SemanticSnapshot,
    TransformationAdmission, TransformationBinding, TransformationError, TransformationLimits,
    TransformationPlanCandidate, TransformationStep, Value, ValueSchema, admit_transformation,
    execute_native_transformation_plan, transformation_value_digest,
};
mod fixture {
    use super::{
        FiniteProblem, FiniteTransformationCatalog, FiniteTransport, TransformationPlanCandidate,
        TransformationStep, Value, ValueSchema, admit_transformation, transformation_value_digest,
    };
    use crate::transformation::{binding, limits};
    pub(super) fn finite_catalog() -> (FiniteTransformationCatalog, TransformationPlanCandidate) {
        let p = FiniteProblem {
            domain: [81; 32],
            answers: 3,
            correct: vec![vec![0], vec![1], vec![2]],
        };
        let q = FiniteProblem {
            domain: [82; 32],
            answers: 3,
            correct: vec![vec![1], vec![2], vec![0]],
        };
        let transport = FiniteTransport {
            lineage: vec![],
            source: p,
            target: q,
            forward: vec![0, 1, 2],
            extract: vec![vec![2, 0, 1]; 3],
        };
        finite_plan(vec![transport], 1)
    }
    pub(super) fn finite_plan(
        transports: Vec<FiniteTransport>,
        input: usize,
    ) -> (FiniteTransformationCatalog, TransformationPlanCandidate) {
        let b = binding(1);
        let values: Vec<_> = transports
            .iter()
            .scan(input, |current, t| {
                let source = *current;
                *current = t.forward[source];
                Some((source, *current))
            })
            .collect();
        let catalog = FiniteTransformationCatalog::new(transports, b.clone(), limits()).unwrap();
        let definitions = catalog.definitions();
        let steps: Vec<_> = definitions
            .iter()
            .enumerate()
            .map(|(i, definition)| TransformationStep {
                admission: admit_transformation(
                    definition,
                    &catalog.evidence(i).unwrap(),
                    &b,
                    limits(),
                    &catalog,
                )
                .unwrap(),
                input: transformation_value_digest(
                    &ValueSchema::Integer,
                    &Value::Integer(values[i].0 as i64),
                    limits(),
                )
                .unwrap(),
                target_input: transformation_value_digest(
                    &ValueSchema::Integer,
                    &Value::Integer(values[i].1 as i64),
                    limits(),
                )
                .unwrap(),
                parameters: catalog.parameters().unwrap(),
            })
            .collect();
        let candidate = TransformationPlanCandidate {
            binding: b,
            source: definitions.first().unwrap().source.clone(),
            target: definitions.last().unwrap().target.clone(),
            input: steps[0].input,
            solver: catalog.solver(&definitions.last().unwrap().target).unwrap(),
            steps,
        };
        (catalog, candidate)
    }
}
#[path = "../unit/transformation_fixture.rs"]
mod transformation;
use fixture::finite_catalog;
use mrr_deduction::ClosureLimits;
use std::num::NonZeroUsize;
use transformation::limits;

fn closure_limits() -> ClosureLimits {
    ClosureLimits::new(
        NonZeroUsize::new(16).unwrap(),
        NonZeroUsize::new(64).unwrap(),
        NonZeroUsize::new(16).unwrap(),
    )
}
fn run(
    catalog: &FiniteTransformationCatalog,
    candidate: &TransformationPlanCandidate,
    edges: &[TransformationAdmission],
    input: Value,
) -> Result<NativeTransformationExecution, TransformationError> {
    execute_native_transformation_plan(
        NativeTransformationExecutionRequest {
            edges,
            candidate,
            input,
            limits: limits(),
            closure_limits: closure_limits(),
        },
        catalog,
        catalog,
    )
}
#[test]
fn native_discovery_dispatch_preserves_original_receipt_and_checked_answer() {
    let (catalog, candidate) = finite_catalog();
    let edges: Vec<_> = candidate
        .steps
        .iter()
        .map(|s| s.admission.clone())
        .collect();
    let receipt = run(&catalog, &candidate, &edges, Value::Integer(1)).unwrap();
    assert_eq!(receipt.search().binding(), &candidate.binding);
    assert_eq!(receipt.search().closure().status(), ClosureStatus::Complete);
    assert_eq!(receipt.search().routes()[0].edges, vec![*edges[0].digest()]);
    assert_eq!(receipt.execution().answer(), &Value::Integer(1));
    assert_eq!(receipt.execution().answer_checks().len(), 2);
}
#[test]
fn native_dispatch_rejects_stale_source_wrong_input_and_unavailable_solver() {
    let (catalog, candidate) = finite_catalog();
    let edges: Vec<_> = candidate
        .steps
        .iter()
        .map(|s| s.admission.clone())
        .collect();
    assert_eq!(
        run(&catalog, &candidate, &edges, Value::Integer(2)).unwrap_err(),
        TransformationError::InstanceMismatch
    );
    assert_eq!(
        run(&catalog, &candidate, &[], Value::Integer(1)).unwrap_err(),
        TransformationError::Unknown
    );
    let mut changed = candidate.clone();
    changed.solver = [99; 32];
    assert_eq!(
        run(&catalog, &changed, &edges, Value::Integer(1)).unwrap_err(),
        TransformationError::Rejected
    );
    changed = candidate.clone();
    changed.binding = transformation::binding(2);
    assert_eq!(
        run(&catalog, &changed, &edges, Value::Integer(1)).unwrap_err(),
        TransformationError::BindingMismatch
    );
    catalog.revoke().unwrap();
    assert_eq!(
        run(&catalog, &candidate, &edges, Value::Integer(1)).unwrap_err(),
        TransformationError::Revoked
    );
}
