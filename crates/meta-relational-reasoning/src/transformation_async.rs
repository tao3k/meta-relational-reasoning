//! Asynchronous physical execution preserving the same admission boundaries.
//! Dropping the future releases no completed receipt. Provider cancellation and
//! effect recovery remain owned by the backend executing the operation.
use super::transformation::{
    TransformationBinding, TransformationEndpoint, TransformationError, TransformationLimits,
    TransformationPlanAdmission, TransformationPlanCandidate, TransformationStep,
    TransformationVerifier, admit_transformation_plan, digest,
};
use super::transformation_execution::{
    TransformationExecutionReceipt, transformation_value_digest,
};
use mrr_relation::Value;

/// Async owner boundary for actual physical query/solver providers.
/// Independent answer checks and current source authority are required just as
/// for the synchronous owner. Futures may be cancelled between operations.
#[allow(
    async_fn_in_trait,
    reason = "Executor locality and Send are chosen by each physical backend."
)]
pub trait AsyncTransformationRuntime {
    fn identity(&self) -> [u8; 32];
    async fn forward(
        &self,
        step: &TransformationStep,
        input: &Value,
        binding: &TransformationBinding,
    ) -> Result<Value, TransformationError>;
    async fn solve(
        &self,
        solver: &[u8; 32],
        target: &TransformationEndpoint,
        input: &Value,
        binding: &TransformationBinding,
    ) -> Result<Value, TransformationError>;
    async fn extract(
        &self,
        step: &TransformationStep,
        source_input: &Value,
        target_answer: &Value,
        binding: &TransformationBinding,
    ) -> Result<Value, TransformationError>;
    async fn check_answer(
        &self,
        endpoint: &TransformationEndpoint,
        input: &Value,
        answer: &Value,
        binding: &TransformationBinding,
    ) -> Result<[u8; 32], TransformationError>;
}

pub async fn execute_transformation_plan_async(
    candidate: &TransformationPlanCandidate,
    admitted: &TransformationPlanAdmission,
    source_input: Value,
    limits: TransformationLimits,
    verifier: &impl TransformationVerifier,
    runtime: &impl AsyncTransformationRuntime,
) -> Result<TransformationExecutionReceipt, TransformationError> {
    let current = admit_transformation_plan(candidate, limits, verifier)?;
    if current != *admitted {
        return Err(TransformationError::BindingMismatch);
    }
    let runtime_id = runtime.identity();
    if runtime_id == [0; 32] {
        return Err(TransformationError::InvalidDigest);
    }
    let input_digest = transformation_value_digest(&candidate.source.input, &source_input, limits)?;
    if input_digest != candidate.input {
        return Err(TransformationError::InstanceMismatch);
    }
    let refresh = || {
        if admit_transformation_plan(candidate, limits, verifier)? != *admitted
            || runtime.identity() != runtime_id
        {
            return Err(TransformationError::BindingMismatch);
        }
        Ok(())
    };
    let mut inputs = vec![source_input];
    for step in &candidate.steps {
        let target = runtime
            .forward(
                step,
                inputs.last().ok_or(TransformationError::Rejected)?,
                &candidate.binding,
            )
            .await?;
        let actual = transformation_value_digest(
            &step.admission.definition().target.input,
            &target,
            limits,
        )?;
        if actual != step.target_input {
            return Err(TransformationError::InstanceMismatch);
        }
        refresh()?;
        inputs.push(target);
    }
    let mut answer = runtime
        .solve(
            &candidate.solver,
            &candidate.target,
            inputs.last().ok_or(TransformationError::Rejected)?,
            &candidate.binding,
        )
        .await?;
    refresh()?;
    let mut checks = Vec::new();
    let mut trace = digest(
        &(
            "mrr.transformation-execution.v1",
            admitted.digest(),
            runtime_id,
            input_digest,
        ),
        limits,
    )?;
    // Check the solver result before any extraction consumes it.
    let answer_digest = transformation_value_digest(&candidate.target.result, &answer, limits)?;
    let check = runtime
        .check_answer(
            &candidate.target,
            inputs.last().ok_or(TransformationError::Rejected)?,
            &answer,
            &candidate.binding,
        )
        .await?;
    refresh()?;
    if check == [0; 32] {
        return Err(TransformationError::InvalidDigest);
    }
    checks.push(check);
    trace = digest(&(trace, answer_digest, check), limits)?;
    for (index, step) in candidate.steps.iter().enumerate().rev() {
        answer = runtime
            .extract(step, &inputs[index], &answer, &candidate.binding)
            .await?;
        refresh()?;
        let answer_digest = transformation_value_digest(
            &step.admission.definition().source.result,
            &answer,
            limits,
        )?;
        let check = runtime
            .check_answer(
                &step.admission.definition().source,
                &inputs[index],
                &answer,
                &candidate.binding,
            )
            .await?;
        refresh()?;
        if check == [0; 32] {
            return Err(TransformationError::InvalidDigest);
        }
        checks.push(check);
        trace = digest(
            &(
                trace,
                index as u64,
                step.admission.digest(),
                step.input,
                step.target_input,
                answer_digest,
                check,
            ),
            limits,
        )?;
    }
    // A late revocation or uncertain authorization cannot release the result.
    let refreshed = admit_transformation_plan(candidate, limits, verifier)?;
    if refreshed != *admitted || runtime.identity() != runtime_id {
        return Err(TransformationError::BindingMismatch);
    }
    Ok(TransformationExecutionReceipt {
        plan: *admitted.digest(),
        input: input_digest,
        answer,
        answer_checks: checks,
        digest: trace,
    })
}
