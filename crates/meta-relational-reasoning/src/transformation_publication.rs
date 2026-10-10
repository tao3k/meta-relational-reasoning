//! In-memory publication for one request/scope; failed candidates retain the
//! previous completed result. Persistent transactions belong to source owners.
use super::transformation::{
    TransformationBinding, TransformationError, TransformationLimits, TransformationPlanAdmission,
    TransformationPlanCandidate, TransformationVerifier, admit_transformation_plan,
};
use super::transformation_execution::TransformationExecutionReceipt;
use super::truth::TruthStatus;

/// A completed execution and its exact plan, with independently tracked freshness.
#[derive(Clone, Debug)]
pub struct PublishedTransformationResult {
    plan: TransformationPlanAdmission,
    execution: TransformationExecutionReceipt,
    freshness: TruthStatus,
}
impl PublishedTransformationResult {
    #[must_use]
    pub fn plan(&self) -> &TransformationPlanAdmission {
        &self.plan
    }
    #[must_use]
    pub fn execution(&self) -> &TransformationExecutionReceipt {
        &self.execution
    }
    #[must_use]
    pub fn freshness(&self) -> TruthStatus {
        self.freshness
    }
}

/// One selected source cut for one request and scope. What-if work uses a
/// separate slot and binding; it cannot publish into the production slot.
#[derive(Clone, Debug)]
pub struct TransformationResultSlot {
    selected: TransformationBinding,
    completed: Option<PublishedTransformationResult>,
}
impl TransformationResultSlot {
    #[must_use]
    pub fn new(selected: TransformationBinding) -> Self {
        Self {
            selected,
            completed: None,
        }
    }
    /// Replace a source/context cut, retaining the previous result as STALE.
    pub fn select(&mut self, selected: TransformationBinding) -> Result<(), TransformationError> {
        if selected.scope != self.selected.scope || selected.request != self.selected.request {
            return Err(TransformationError::BindingMismatch);
        }
        if selected != self.selected {
            self.selected = selected;
            self.invalidate();
        }
        Ok(())
    }
    /// Source-owner revocation does not erase historical completed evidence.
    pub fn invalidate(&mut self) {
        if let Some(completed) = &mut self.completed {
            completed.freshness = TruthStatus::Stale;
        }
    }
    #[must_use]
    pub fn selected(&self) -> &TransformationBinding {
        &self.selected
    }
    #[must_use]
    pub fn completed(&self) -> Option<&PublishedTransformationResult> {
        self.completed.as_ref()
    }
    /// Recheck current obligations before replacing the completed publication.
    /// An error leaves the previous completed result and freshness unchanged.
    pub fn publish(
        &mut self,
        candidate: &TransformationPlanCandidate,
        execution: TransformationExecutionReceipt,
        limits: TransformationLimits,
        verifier: &impl TransformationVerifier,
    ) -> Result<(), TransformationError> {
        if candidate.binding != self.selected {
            return Err(TransformationError::BindingMismatch);
        }
        let plan = admit_transformation_plan(candidate, limits, verifier)?;
        if execution.plan_digest() != plan.digest() {
            return Err(TransformationError::BindingMismatch);
        }
        self.completed = Some(PublishedTransformationResult {
            plan,
            execution,
            freshness: TruthStatus::True,
        });
        Ok(())
    }
}
