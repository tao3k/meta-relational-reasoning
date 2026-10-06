pub(super) use crate::transformation::{binding, limits};
pub(super) use crate::{
    FiniteProblem, FiniteTransformationCatalog, FiniteTransport, TransformationResultStore,
};
pub(super) use crate::{
    TransformationError, TransformationPlanCandidate, TransformationStep, TruthStatus, Value,
    ValueSchema, admit_transformation, admit_transformation_plan, execute_transformation_plan,
    transformation_value_digest,
};
pub(super) use sha2::Sha256;

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
