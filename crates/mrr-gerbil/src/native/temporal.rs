//! Explicit advanced Host handoff to the POO Library's inert Scheme datum v1.
//! This shares parsing and inference's single initializer and owner thread.
use super::{
    NativeRuntimeStatus, ffi,
    runtime::{NativeRuntimeError, with_native_runtime},
};
use std::{ffi::CString, fmt};

/// Rejection at the bounded transport or existing native owner boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TemporalRuntimeError {
    InvalidInput,
    Worker(crate::worker_profile::WorkerFailure),
    RuntimeUnavailable,
    RuntimeInitialization(NativeRuntimeStatus),
    NativeRejected(i32),
}
impl fmt::Display for TemporalRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for TemporalRuntimeError {}

/// A process-local advanced Host projection, never a model tool dispatcher.
/// Payloads are bounded inert Scheme v1 owner records, not executable source.
/// POO owns all semantic validation. Successful calls grant no external effects.
#[derive(Clone, Copy, Debug, Default)]
pub struct TemporalHost;
impl TemporalHost {
    fn invoke(self, operation: i32, payload: &[u8]) -> Result<Vec<u8>, TemporalRuntimeError> {
        self.invoke_routed(operation, operation, payload)
    }
    fn invoke_routed(
        self,
        operation: i32,
        worker_operation: i32,
        payload: &[u8],
    ) -> Result<Vec<u8>, TemporalRuntimeError> {
        if let Some(result) =
            crate::worker_profile::with_worker(|worker| worker.invoke(worker_operation, payload))
        {
            return result.map_err(|error| match error {
                crate::NativeWorkerError::Native(error) => error,
                error => TemporalRuntimeError::Worker(error.failure()),
            });
        }
        if payload.len() > 1_048_576 || std::str::from_utf8(payload).is_err() {
            return Err(TemporalRuntimeError::InvalidInput);
        }
        let payload = CString::new(payload).map_err(|_| TemporalRuntimeError::InvalidInput)?;
        with_native_runtime(move || ffi::temporal(operation, &payload))
            .map_err(|e| match e {
                NativeRuntimeError::Unavailable => TemporalRuntimeError::RuntimeUnavailable,
                #[cfg(feature = "embedded-runtime")]
                NativeRuntimeError::Status(s) => TemporalRuntimeError::RuntimeInitialization(s),
            })?
            .map_err(TemporalRuntimeError::NativeRejected)
    }
    pub(crate) fn compile_search_projection(
        self,
        payload: &[u8],
    ) -> Result<Vec<u8>, TemporalRuntimeError> {
        self.invoke_routed(5, 8, payload)
    }
    /// Refresh the process-local policy generation through its Scheme owner.
    pub fn refresh_policy(self, payload: &[u8]) -> Result<Vec<u8>, TemporalRuntimeError> {
        self.invoke(0, payload)
    }
    /// Refresh the admitted proof state through its Scheme owner.
    pub fn refresh_proof_state(self, payload: &[u8]) -> Result<Vec<u8>, TemporalRuntimeError> {
        self.invoke(1, payload)
    }
    /// Register an original derivation against the current native proof state.
    pub fn register_proof(self, payload: &[u8]) -> Result<Vec<u8>, TemporalRuntimeError> {
        self.invoke(2, payload)
    }
    /// Admit a derivation using the POO Library's semantic contract.
    pub fn admit_derivation(self, payload: &[u8]) -> Result<Vec<u8>, TemporalRuntimeError> {
        self.invoke(3, payload)
    }
    /// Check current proof status without granting external effect authority.
    pub fn current_proof(self, payload: &[u8]) -> Result<Vec<u8>, TemporalRuntimeError> {
        self.invoke(4, payload)
    }
}
