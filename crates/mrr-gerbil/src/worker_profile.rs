//! Explicit process profile. No path discovery, embedded fallback or restart.
use crate::{NativeWorker, NativeWorkerError};
use std::{
    path::Path,
    sync::{Mutex, OnceLock},
};
static WORKER: OnceLock<Mutex<NativeWorker>> = OnceLock::new();
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkerFailure {
    InvalidInput,
    Busy,
    Closed,
    Protocol,
    Timeout,
    Io,
    Native,
}
impl NativeWorkerError {
    pub(crate) fn failure(&self) -> WorkerFailure {
        match self {
            Self::InvalidInput => WorkerFailure::InvalidInput,
            Self::Busy => WorkerFailure::Busy,
            Self::Closed | Self::ModeConflict | Self::AlreadyConfigured => WorkerFailure::Closed,
            Self::Protocol => WorkerFailure::Protocol,
            Self::Timeout => WorkerFailure::Timeout,
            Self::Io(_) => WorkerFailure::Io,
            Self::Native(_) | Self::Parser(_) | Self::Finite(_) => WorkerFailure::Native,
        }
    }
}
/// Select the isolated process profile before any embedded native call.
/// Concurrent calls fail Busy instead of waiting outside the operation deadline.
pub fn configure_native_worker(executable: &Path) -> Result<(), NativeWorkerError> {
    if WORKER.get().is_some() {
        return Err(NativeWorkerError::AlreadyConfigured);
    }
    let worker = NativeWorker::start(executable)?;
    WORKER
        .set(Mutex::new(worker))
        .map_err(|_| NativeWorkerError::AlreadyConfigured)
}
pub(crate) fn with_worker<T>(
    operation: impl FnOnce(&mut NativeWorker) -> Result<T, NativeWorkerError>,
) -> Option<Result<T, NativeWorkerError>> {
    WORKER.get().map(|worker| match worker.try_lock() {
        Ok(mut worker) => operation(&mut worker),
        Err(std::sync::TryLockError::WouldBlock) => Err(NativeWorkerError::Busy),
        Err(std::sync::TryLockError::Poisoned(_)) => Err(NativeWorkerError::Closed),
    })
}
/// Terminal shutdown retains the process profile; no later embedded fallback.
pub fn shutdown_native_worker() -> Result<(), NativeWorkerError> {
    with_worker(|worker| {
        worker.cancel();
        Ok(())
    })
    .unwrap_or(Err(NativeWorkerError::Closed))
}
