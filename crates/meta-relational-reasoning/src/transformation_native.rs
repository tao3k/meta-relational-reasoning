//! Connect native ASCENT route discovery to independently admitted execution.
//! The runtime owns physical provider dispatch; native reachability supplies no
//! solver, source authority or answer correctness by itself.
use crate::{
    TransformationAdmission, TransformationError, TransformationExecutionReceipt,
    TransformationLimits, TransformationPlanCandidate, TransformationRouteSearch,
    TransformationRuntime, TransformationVerifier, Value, admit_transformation,
    admit_transformation_plan, execute_transformation_plan, search_transformation_routes,
};

/// Original native search evidence and checked physical execution kept together.
#[derive(Clone, Debug)]
pub struct NativeTransformationExecution {
    search: TransformationRouteSearch,
    execution: TransformationExecutionReceipt,
}
impl NativeTransformationExecution {
    #[must_use]
    pub fn search(&self) -> &TransformationRouteSearch {
        &self.search
    }
    #[must_use]
    pub fn execution(&self) -> &TransformationExecutionReceipt {
        &self.execution
    }
}

/// Current admitted native cut, concrete plan and bounded physical input.
pub struct NativeTransformationExecutionRequest<'a> {
    pub edges: &'a [TransformationAdmission],
    pub candidate: &'a TransformationPlanCandidate,
    pub input: Value,
    pub limits: TransformationLimits,
    pub closure_limits: mrr_deduction::ClosureLimits,
}

/// Execute one concrete plan only when its exact ordered edges are discovered
/// by the native provider in the current admitted cut. Revalidate every edge and
/// every concrete premise before native dispatch, and again during execution.
/// A truncated search can admit an explicitly returned route; absence remains
/// Unknown. No fallback evaluator or automatic route substitution is used.
pub fn execute_native_transformation_plan(
    request: NativeTransformationExecutionRequest<'_>,
    verifier: &impl TransformationVerifier,
    runtime: &impl TransformationRuntime,
) -> Result<NativeTransformationExecution, TransformationError> {
    let NativeTransformationExecutionRequest {
        edges,
        candidate,
        input,
        limits,
        closure_limits,
    } = request;
    if crate::transformation_value_digest(&candidate.source.input, &input, limits)?
        != candidate.input
    {
        return Err(TransformationError::InstanceMismatch);
    }
    if edges.len() > limits.max_dependencies.get() {
        return Err(TransformationError::Budget);
    }
    for edge in edges {
        if edge.binding() != &candidate.binding {
            return Err(TransformationError::BindingMismatch);
        }
        let current = admit_transformation(
            edge.definition(),
            edge.evidence(),
            &candidate.binding,
            limits,
            verifier,
        )?;
        if current != *edge {
            return Err(TransformationError::SourceMismatch);
        }
    }
    if !candidate
        .steps
        .iter()
        .all(|step| edges.contains(&step.admission))
    {
        return Err(TransformationError::Unknown);
    }
    let admitted = admit_transformation_plan(candidate, limits, verifier)?;
    let search = search_transformation_routes(edges, &candidate.binding, limits, closure_limits)?;
    let selected: Vec<_> = candidate
        .steps
        .iter()
        .map(|s| *s.admission.digest())
        .collect();
    if !search.routes().iter().any(|route| {
        route.source == candidate.source
            && route.target == candidate.target
            && route.edges == selected
    }) {
        return Err(TransformationError::Unknown);
    }
    let execution =
        execute_transformation_plan(candidate, &admitted, input, limits, verifier, runtime)?;
    Ok(NativeTransformationExecution { search, execution })
}
