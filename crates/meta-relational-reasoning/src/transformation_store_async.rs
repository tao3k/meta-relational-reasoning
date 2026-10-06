//! Physical execution precedes the synchronous durable publication lease.
use super::{
    ArchivedResult, AsyncTransformationPublisher, PublicationAuthority, TransformationError,
    TransformationExecutionReceipt, TransformationPlanCandidate, TransformationResultStore, Value,
    admit_transformation_plan,
};
use crate::{TransformationVerifier, execute_transformation_plan_async};

impl TransformationResultStore {
    /// Execute the actual provider, independently check its answer, and publish
    /// under its live authority. Cancellation never installs a fresh record.
    pub async fn execute_and_publish_async(
        &mut self,
        candidate: &TransformationPlanCandidate,
        source_input: Value,
        verifier: &impl TransformationVerifier,
        runtime: &impl AsyncTransformationPublisher,
    ) -> Result<TransformationExecutionReceipt, TransformationError> {
        self.check_slot(candidate)?;
        self.current = false;
        let plan = admit_transformation_plan(candidate, self.limits, verifier)?;
        let identity = runtime.identity();
        let execution = execute_transformation_plan_async(
            candidate,
            &plan,
            source_input,
            self.limits,
            verifier,
            runtime,
        )
        .await?;
        let mut supports = runtime.publication_supports();
        supports.push(identity);
        let record = ArchivedResult {
            version: 1,
            request: candidate.binding.request,
            scope: candidate.binding.scope,
            plan: *plan.digest(),
            execution: *execution.digest(),
            answer: execution.answer().clone(),
            supports: self.support_cut(candidate, supports)?,
            invalidated: false,
        };
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(&record, &mut bytes)
            .map_err(|_| TransformationError::Encoding)?;
        if bytes.len() > self.limits.max_bytes.get() {
            return Err(TransformationError::Budget);
        }
        let lease = runtime.publication_lease();
        let mut committed = false;
        let published = runtime.publish(&candidate.binding, bytes.len() as u64, || {
            if runtime.identity() != identity
                || admit_transformation_plan(candidate, self.limits, verifier)? != plan
            {
                return Err(TransformationError::BindingMismatch);
            }
            self.write(&record)?;
            committed = true;
            Ok(())
        });
        if let Err(error) = published {
            return Err(if committed {
                TransformationError::PublicationUncertain
            } else {
                error
            });
        }
        if !lease.is_current()
            || runtime.identity() != identity
            || admit_transformation_plan(candidate, self.limits, verifier) != Ok(plan)
        {
            return Err(TransformationError::PublicationUncertain);
        }
        self.archived = Some(record);
        self.current = true;
        self.authority = Some(PublicationAuthority::Physical(lease));
        self.index();
        Ok(execution)
    }

    /// Restart recovery runs the physical solver and oracle again. Serialized
    /// receipts alone cannot recover freshness or bypass support invalidation.
    pub async fn replay_async(
        &mut self,
        candidate: &TransformationPlanCandidate,
        source_input: Value,
        verifier: &impl TransformationVerifier,
        runtime: &impl AsyncTransformationPublisher,
    ) -> Result<TransformationExecutionReceipt, TransformationError> {
        self.current = false;
        let archived = self.archived.as_ref().ok_or(TransformationError::Unknown)?;
        if archived.invalidated {
            return Err(TransformationError::Revoked);
        }
        self.check_slot(candidate)?;
        let plan = admit_transformation_plan(candidate, self.limits, verifier)?;
        let identity = runtime.identity();
        let execution = execute_transformation_plan_async(
            candidate,
            &plan,
            source_input,
            self.limits,
            verifier,
            runtime,
        )
        .await?;
        let mut supports = runtime.publication_supports();
        supports.push(identity);
        if archived.supports != self.support_cut(candidate, supports)?
            || archived.plan != *plan.digest()
            || archived.execution != *execution.digest()
            || archived.answer != *execution.answer()
        {
            return Err(TransformationError::BindingMismatch);
        }
        let lease = runtime.publication_lease();
        runtime.publish(&candidate.binding, 0, || {
            if !lease.is_current()
                || runtime.identity() != identity
                || admit_transformation_plan(candidate, self.limits, verifier)? != plan
            {
                return Err(TransformationError::BindingMismatch);
            }
            Ok(())
        })?;
        if !lease.is_current()
            || runtime.identity() != identity
            || admit_transformation_plan(candidate, self.limits, verifier) != Ok(plan)
        {
            return Err(TransformationError::Revoked);
        }
        self.current = true;
        self.authority = Some(PublicationAuthority::Physical(lease));
        Ok(execution)
    }
}
