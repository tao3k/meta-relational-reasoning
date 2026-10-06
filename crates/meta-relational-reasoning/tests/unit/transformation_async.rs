//! Async dispatch parity, suspension revocation and cancellation qualification.
use super::{CurrentVerifier, NaturalRuntime, concrete_plan, limits};
use crate::{
    AsyncTransformationRuntime, TransformationBinding, TransformationEndpoint, TransformationError,
    TransformationRuntime, TransformationStep, Value, admit_transformation_plan,
    execute_transformation_plan, execute_transformation_plan_async,
};
use std::{
    cell::Cell,
    future::Future,
    pin::pin,
    task::{Context, Poll, Waker},
};

struct SuspendedRuntime<'a> {
    inner: NaturalRuntime<'a>,
    revoke_during_solve: bool,
    extracted: Cell<usize>,
}
impl AsyncTransformationRuntime for SuspendedRuntime<'_> {
    fn identity(&self) -> [u8; 32] {
        self.inner.identity()
    }
    async fn forward(
        &self,
        step: &TransformationStep,
        input: &Value,
        binding: &TransformationBinding,
    ) -> Result<Value, TransformationError> {
        self.inner.forward(step, input, binding)
    }
    async fn solve(
        &self,
        solver: &[u8; 32],
        target: &TransformationEndpoint,
        input: &Value,
        binding: &TransformationBinding,
    ) -> Result<Value, TransformationError> {
        let mut suspended = false;
        std::future::poll_fn(|cx| {
            if !suspended {
                suspended = true;
                if self.revoke_during_solve {
                    self.inner.revoked.set(true);
                }
                cx.waker().wake_by_ref();
                Poll::Pending
            } else {
                Poll::Ready(())
            }
        })
        .await;
        self.inner.solve(solver, target, input, binding)
    }
    async fn extract(
        &self,
        step: &TransformationStep,
        input: &Value,
        answer: &Value,
        binding: &TransformationBinding,
    ) -> Result<Value, TransformationError> {
        self.extracted.set(self.extracted.get() + 1);
        self.inner.extract(step, input, answer, binding)
    }
    async fn check_answer(
        &self,
        endpoint: &TransformationEndpoint,
        input: &Value,
        answer: &Value,
        binding: &TransformationBinding,
    ) -> Result<[u8; 32], TransformationError> {
        self.inner.check_answer(endpoint, input, answer, binding)
    }
}
fn complete<T>(future: impl Future<Output = T>) -> T {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    assert!(future.as_mut().poll(&mut context).is_pending());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("fixture failed to finish after its single suspension"),
    }
}
#[test]
fn async_receipt_matches_sync_execution_after_suspension() {
    let revoked = Cell::new(false);
    let verifier = CurrentVerifier {
        revoked: &revoked,
        failed_step: None,
        solver_missing: false,
    };
    let runtime = SuspendedRuntime {
        inner: NaturalRuntime {
            mode: 0,
            revoked: &revoked,
        },
        revoke_during_solve: false,
        extracted: Cell::new(0),
    };
    let plan = concrete_plan(17);
    let admitted = admit_transformation_plan(&plan, limits(), &verifier).unwrap();
    let sync = execute_transformation_plan(
        &plan,
        &admitted,
        Value::Integer(17),
        limits(),
        &verifier,
        &runtime.inner,
    )
    .unwrap();
    let asynchronous = complete(execute_transformation_plan_async(
        &plan,
        &admitted,
        Value::Integer(17),
        limits(),
        &verifier,
        &runtime,
    ))
    .unwrap();
    assert_eq!(sync, asynchronous);
    assert_eq!(runtime.extracted.get(), 2);
}
#[test]
fn async_revocation_during_solver_releases_no_completed_receipt() {
    let revoked = Cell::new(false);
    let verifier = CurrentVerifier {
        revoked: &revoked,
        failed_step: None,
        solver_missing: false,
    };
    let runtime = SuspendedRuntime {
        inner: NaturalRuntime {
            mode: 0,
            revoked: &revoked,
        },
        revoke_during_solve: true,
        extracted: Cell::new(0),
    };
    let plan = concrete_plan(17);
    let admitted = admit_transformation_plan(&plan, limits(), &verifier).unwrap();
    assert_eq!(
        complete(execute_transformation_plan_async(
            &plan,
            &admitted,
            Value::Integer(17),
            limits(),
            &verifier,
            &runtime
        ))
        .unwrap_err(),
        TransformationError::Revoked
    );
    assert_eq!(runtime.extracted.get(), 0);
}
#[test]
fn cancelling_suspended_solver_never_runs_extraction() {
    let revoked = Cell::new(false);
    let verifier = CurrentVerifier {
        revoked: &revoked,
        failed_step: None,
        solver_missing: false,
    };
    let runtime = SuspendedRuntime {
        inner: NaturalRuntime {
            mode: 0,
            revoked: &revoked,
        },
        revoke_during_solve: false,
        extracted: Cell::new(0),
    };
    let plan = concrete_plan(17);
    let admitted = admit_transformation_plan(&plan, limits(), &verifier).unwrap();
    {
        let mut future = pin!(execute_transformation_plan_async(
            &plan,
            &admitted,
            Value::Integer(17),
            limits(),
            &verifier,
            &runtime
        ));
        assert!(
            future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
    }
    assert_eq!(runtime.extracted.get(), 0);
}
