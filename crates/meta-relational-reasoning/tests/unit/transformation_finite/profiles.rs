use super::{
    FiniteProblem, TransformationError, TransformationPlanCandidate, TransformationStep, Value,
    ValueSchema, admit_transformation, admit_transformation_plan, binding,
    execute_transformation_plan, limits, transformation_value_digest,
};

fn kernel_profile_plan(
    kernel: &crate::KernelCheckedFiniteCatalog,
    input: Value,
) -> TransformationPlanCandidate {
    use crate::TransformationRuntime;
    let b = kernel.native().selected_binding().clone();
    let definition = &kernel.native().definitions()[0];
    let admission = admit_transformation(
        definition,
        &kernel.evidence(0).unwrap(),
        &b,
        limits(),
        kernel,
    )
    .unwrap();
    let mut step = TransformationStep {
        admission,
        input: transformation_value_digest(&ValueSchema::Integer, &input, limits()).unwrap(),
        target_input: [1; 32],
        parameters: kernel.native().parameters().unwrap(),
    };
    let target_input = kernel.forward(&step, &input, &b).unwrap();
    step.target_input =
        transformation_value_digest(&ValueSchema::Integer, &target_input, limits()).unwrap();
    TransformationPlanCandidate {
        binding: b,
        source: definition.source.clone(),
        target: definition.target.clone(),
        input: step.input,
        steps: vec![step],
        solver: kernel.native().solver(&definition.target).unwrap(),
    }
}

#[test]
fn concrete_profiles_execute_decisions_optima_and_partial_instances_under_actual_kernel() {
    use crate::{
        FiniteDecisionProblem as D, FiniteOptimizationProblem as O, FiniteOptimizationTransport,
        FinitePartialTransport, KernelCheckedDecisionReduction as Decision,
        KernelCheckedOptimizationReduction as Optimization,
        KernelCheckedPartialTransformation as Partial,
    };
    let elan = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|p| p.join("elan"))
        .find(|p| p.is_file())
        .unwrap();
    let mut bounded = limits();
    bounded.max_bytes = std::num::NonZeroUsize::new(65536).unwrap();
    let source = D {
        domain: [11; 32],
        yes: vec![true, false],
    };
    let target = D {
        domain: [12; 32],
        yes: vec![false, true],
    };
    assert!(
        Decision::check(
            source.clone(),
            target.clone(),
            vec![0, 1],
            binding(1),
            bounded,
            &elan,
            &std::env::temp_dir()
        )
        .is_err()
    );
    let decision = Decision::check(
        source,
        target,
        vec![1, 0],
        binding(1),
        bounded,
        &elan,
        &std::env::temp_dir(),
    )
    .unwrap();
    for (input, expected) in [(0, 1), (1, 0)] {
        let k = decision.kernel();
        let c = kernel_profile_plan(k, Value::Integer(input));
        let p = admit_transformation_plan(&c, bounded, k).unwrap();
        assert_eq!(
            execute_transformation_plan(&c, &p, Value::Integer(input), bounded, k, k)
                .unwrap()
                .answer(),
            &Value::Integer(expected)
        );
    }
    let source = O {
        domain: [21; 32],
        feasible: vec![vec![true; 3]],
        costs: vec![vec![10, 1, 1]],
    };
    let target = O {
        domain: [22; 32],
        feasible: vec![vec![true; 3]],
        costs: vec![vec![9, 3, 0]],
    };
    let mut specification = FiniteOptimizationTransport {
        source: source.clone(),
        target,
        forward: vec![0],
        extract: vec![vec![0, 0, 0]],
    };
    // Even a forged feasible-row lowering that passes generic witness soundness
    // must fail the independent kernel optimality agreement obligation.
    assert!(matches!(
        Optimization::check_compiled(
            specification.clone(),
            crate::CompiledFiniteOptimizationRows {
                source: vec![vec![0, 1, 2]],
                target: vec![vec![0, 1, 2]]
            },
            binding(1),
            bounded,
            &elan,
            &std::env::temp_dir()
        ),
        Err(TransformationError::KernelRejected { .. })
    ));
    // Answer 0 is feasible but nonoptimal: witness soundness is insufficient.
    assert!(
        Optimization::check(
            specification.clone(),
            binding(1),
            bounded,
            &elan,
            &std::env::temp_dir()
        )
        .is_err()
    );
    specification.extract[0][2] = 2;
    let optimal = Optimization::check(
        specification,
        binding(1),
        bounded,
        &elan,
        &std::env::temp_dir(),
    )
    .unwrap();
    let k = optimal.kernel();
    let c = kernel_profile_plan(k, Value::Integer(0));
    let p = admit_transformation_plan(&c, bounded, k).unwrap();
    let result = execute_transformation_plan(&c, &p, Value::Integer(0), bounded, k, k).unwrap();
    let Value::Integer(answer) = result.answer() else {
        panic!("integer optimum")
    };
    assert_eq!(*answer, 2);
    for other in 0..3 {
        assert!(source.costs[0][*answer as usize] <= source.costs[0][other]);
    }
    let problem = FiniteProblem {
        domain: [31; 32],
        answers: 3,
        correct: vec![vec![0], vec![1], vec![2]],
    };
    let partial = Partial::check(
        FinitePartialTransport {
            source: problem.clone(),
            target: problem,
            forward: vec![Some(0), None, Some(2)],
            extract: vec![vec![0, 1, 2]; 3],
        },
        binding(1),
        bounded,
        &elan,
        &std::env::temp_dir(),
    )
    .unwrap();
    assert_eq!(partial.original_inputs(), &[0, 2]);
    assert_eq!(
        partial.source_instance(1),
        Err(TransformationError::Unknown)
    );
    let input = partial.source_instance(2).unwrap();
    assert_eq!(input, Value::Integer(1));
    let k = partial.kernel();
    let c = kernel_profile_plan(k, input.clone());
    let p = admit_transformation_plan(&c, bounded, k).unwrap();
    assert_eq!(
        execute_transformation_plan(&c, &p, input, bounded, k, k)
            .unwrap()
            .answer(),
        &Value::Integer(2)
    );
}
