//! Runtime execution with bounded typed values and independent answer checks.
//!
//! Runtime dispatch and the answer checker are trusted owner interfaces. This
//! receipt records checked execution; it does not prove native refinement.
use super::transformation::{
    TransformationBinding, TransformationEndpoint, TransformationError, TransformationLimits,
    TransformationPlanAdmission, TransformationPlanCandidate, TransformationStep,
    TransformationVerifier, admit_transformation_plan, digest,
};
use mrr_relation::{Value, ValueSchema};

/// Runtime/source owner boundary. Implementations must resolve the exact
/// artifacts and parameter digests, enforce effect authorization and budgets,
/// and report Unknown for uncertain outcomes. Answer checks must use the
/// endpoint's independent specification, not the forward/extractor function.
pub trait TransformationRuntime {
    fn identity(&self) -> [u8; 32];
    fn forward(
        &self,
        step: &TransformationStep,
        input: &Value,
        binding: &TransformationBinding,
    ) -> Result<Value, TransformationError>;
    fn solve(
        &self,
        solver: &[u8; 32],
        target: &TransformationEndpoint,
        input: &Value,
        binding: &TransformationBinding,
    ) -> Result<Value, TransformationError>;
    fn extract(
        &self,
        step: &TransformationStep,
        source_input: &Value,
        target_answer: &Value,
        binding: &TransformationBinding,
    ) -> Result<Value, TransformationError>;
    fn check_answer(
        &self,
        endpoint: &TransformationEndpoint,
        input: &Value,
        answer: &Value,
        binding: &TransformationBinding,
    ) -> Result<[u8; 32], TransformationError>;
}

/// Completed execution checked by the configured runtime's independent oracle.
/// Opaque and non-deserializable; no partial or unknown result has this type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransformationExecutionReceipt {
    plan: [u8; 32],
    input: [u8; 32],
    answer: Value,
    answer_checks: Vec<[u8; 32]>,
    digest: [u8; 32],
}
impl TransformationExecutionReceipt {
    #[must_use]
    pub fn plan_digest(&self) -> &[u8; 32] {
        &self.plan
    }
    #[must_use]
    pub fn input_digest(&self) -> &[u8; 32] {
        &self.input
    }
    #[must_use]
    pub fn answer(&self) -> &Value {
        &self.answer
    }
    #[must_use]
    pub fn answer_checks(&self) -> &[[u8; 32]] {
        &self.answer_checks
    }
    #[must_use]
    pub fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
}

/// Execute a freshly revalidated plan; recheck source authority again before
/// releasing a completed answer. Effects/retries/recovery belong to runtime.
pub fn execute_transformation_plan(
    candidate: &TransformationPlanCandidate,
    admitted: &TransformationPlanAdmission,
    source_input: Value,
    limits: TransformationLimits,
    verifier: &impl TransformationVerifier,
    runtime: &impl TransformationRuntime,
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
    let mut inputs = vec![source_input];
    for step in &candidate.steps {
        let target = runtime.forward(
            step,
            inputs.last().ok_or(TransformationError::Rejected)?,
            &candidate.binding,
        )?;
        let actual = transformation_value_digest(
            &step.admission.definition().target.input,
            &target,
            limits,
        )?;
        if actual != step.target_input {
            return Err(TransformationError::InstanceMismatch);
        }
        inputs.push(target);
    }
    let mut answer = runtime.solve(
        &candidate.solver,
        &candidate.target,
        inputs.last().ok_or(TransformationError::Rejected)?,
        &candidate.binding,
    )?;
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
    let check = runtime.check_answer(
        &candidate.target,
        inputs.last().ok_or(TransformationError::Rejected)?,
        &answer,
        &candidate.binding,
    )?;
    if check == [0; 32] {
        return Err(TransformationError::InvalidDigest);
    }
    checks.push(check);
    trace = digest(&(trace, answer_digest, check), limits)?;
    for (index, step) in candidate.steps.iter().enumerate().rev() {
        answer = runtime.extract(step, &inputs[index], &answer, &candidate.binding)?;
        let answer_digest = transformation_value_digest(
            &step.admission.definition().source.result,
            &answer,
            limits,
        )?;
        let check = runtime.check_answer(
            &step.admission.definition().source,
            &inputs[index],
            &answer,
            &candidate.binding,
        )?;
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

/// Domain-framed concrete instance identity. Schema and value both participate.
/// Values and schemas are bounded before recursive validators/serialization.
pub fn transformation_value_digest(
    schema: &ValueSchema,
    value: &Value,
    limits: TransformationLimits,
) -> Result<[u8; 32], TransformationError> {
    let mut schemas = vec![(schema, 1_usize)];
    let mut count = 0_usize;
    while let Some((schema, depth)) = schemas.pop() {
        count += 1;
        if count > limits.max_schema_nodes.get() || depth > limits.max_schema_depth.get() {
            return Err(TransformationError::Budget);
        }
        match schema {
            ValueSchema::List { element, .. } => schemas.push((element, depth + 1)),
            ValueSchema::Record { fields } => {
                if fields.len()
                    > limits
                        .max_schema_nodes
                        .get()
                        .saturating_sub(count.saturating_add(schemas.len()))
                {
                    return Err(TransformationError::Budget);
                }
                schemas.extend(fields.iter().map(|field| (field.schema(), depth + 1)));
            }
            _ => {}
        }
    }
    let mut values = vec![(value, 1_usize)];
    count = 0;
    while let Some((value, depth)) = values.pop() {
        count += 1;
        if count > limits.max_bytes.get() || depth > limits.max_schema_depth.get() {
            return Err(TransformationError::Budget);
        }
        match value {
            Value::List(elements) => {
                if elements.len()
                    > limits
                        .max_bytes
                        .get()
                        .saturating_sub(count.saturating_add(values.len()))
                {
                    return Err(TransformationError::Budget);
                }
                values.extend(elements.iter().map(|element| (element, depth + 1)));
            }
            Value::Record(fields) => {
                if fields.len()
                    > limits
                        .max_bytes
                        .get()
                        .saturating_sub(count.saturating_add(values.len()))
                {
                    return Err(TransformationError::Budget);
                }
                values.extend(fields.iter().map(|(_, value)| (value, depth + 1)));
            }
            _ => {}
        }
    }
    // Enforce bytes before content validators scan strings/collections.
    let digest = digest(&("mrr.transformation-value.v1", schema, value), limits)?;
    schema
        .validate()
        .map_err(|_| TransformationError::InvalidSchema)?;
    schema
        .validate_value(value)
        .map_err(|_| TransformationError::InvalidSchema)?;
    Ok(digest)
}
