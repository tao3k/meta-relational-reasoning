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
    publication: std::sync::Arc<PublicationState>,
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
        publication: std::sync::Arc::new(PublicationState::default()),
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
        publication: std::sync::Arc::new(PublicationState::default()),
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
        publication: std::sync::Arc::new(PublicationState::default()),
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

#[derive(Debug, Default)]
struct PublicationState {
    revoked: std::sync::atomic::AtomicBool,
    revoke_after_write: std::sync::atomic::AtomicBool,
}
impl crate::TransformationPublicationLease for PublicationState {
    fn is_current(&self) -> bool {
        !self.revoked.load(std::sync::atomic::Ordering::Acquire)
    }
}
impl crate::AsyncTransformationPublisher for SuspendedRuntime<'_> {
    fn publication_supports(&self) -> Vec<[u8; 32]> {
        vec![[91; 32]]
    }
    fn publication_lease(&self) -> std::sync::Arc<dyn crate::TransformationPublicationLease> {
        self.publication.clone()
    }
    fn publish<T>(
        &self,
        _: &TransformationBinding,
        _: u64,
        action: impl FnOnce() -> Result<T, TransformationError>,
    ) -> Result<T, TransformationError> {
        use crate::TransformationPublicationLease;
        use std::sync::atomic::Ordering;
        if !self.publication.is_current() {
            return Err(TransformationError::Revoked);
        }
        let result = action()?;
        if self.publication.revoke_after_write.load(Ordering::Acquire) {
            self.publication.revoked.store(true, Ordering::Release);
            return Err(TransformationError::PublicationUncertain);
        }
        Ok(result)
    }
}
#[test]
fn physical_archive_requires_replay_and_retains_durable_support_invalidation() {
    let path = std::env::temp_dir().join(format!("mrr-physical-store-{}.cbor", std::process::id()));
    let _ = std::fs::remove_file(&path);
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
        publication: std::sync::Arc::new(PublicationState::default()),
    };
    let plan = concrete_plan(17);
    let mut store = crate::TransformationResultStore::open(&path, limits()).unwrap();
    complete(store.execute_and_publish_async(&plan, Value::Integer(17), &verifier, &runtime))
        .unwrap();
    assert_eq!(store.freshness(), crate::TruthStatus::True);
    assert_eq!(store.affected_plans(&[91; 32]).len(), 1);
    drop(store);
    let mut store = crate::TransformationResultStore::open(&path, limits()).unwrap();
    assert_eq!(store.freshness(), crate::TruthStatus::Stale);
    complete(store.replay_async(&plan, Value::Integer(17), &verifier, &runtime)).unwrap();
    assert_eq!(store.freshness(), crate::TruthStatus::True);
    assert!(store.invalidate(&[91; 32]).unwrap());
    drop(store);
    let mut store = crate::TransformationResultStore::open(&path, limits()).unwrap();
    {
        let mut future = pin!(store.replay_async(&plan, Value::Integer(17), &verifier, &runtime));
        assert!(matches!(
            future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Ready(Err(TransformationError::Revoked))
        ));
    }
    drop(store);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(path.with_extension("lock"));
}
#[test]
fn physical_archive_uncertain_write_never_becomes_fresh() {
    use std::sync::atomic::Ordering;
    let path = std::env::temp_dir().join(format!(
        "mrr-physical-uncertain-{}.cbor",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
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
        publication: std::sync::Arc::new(PublicationState::default()),
    };
    runtime
        .publication
        .revoke_after_write
        .store(true, Ordering::Release);
    let mut store = crate::TransformationResultStore::open(&path, limits()).unwrap();
    assert_eq!(
        complete(store.execute_and_publish_async(
            &concrete_plan(17),
            Value::Integer(17),
            &verifier,
            &runtime
        )),
        Err(TransformationError::PublicationUncertain)
    );
    assert_ne!(store.freshness(), crate::TruthStatus::True);
    drop(store);
    let store = crate::TransformationResultStore::open(&path, limits()).unwrap();
    assert_eq!(store.freshness(), crate::TruthStatus::Stale);
    assert!(store.historical_answer().is_some());
    drop(store);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(path.with_extension("lock"));
}
