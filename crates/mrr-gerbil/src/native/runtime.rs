//! Process-global serialization boundary for the embedded Gambit runtime.

use std::sync::{Mutex, MutexGuard};

static NATIVE_RUNTIME: Mutex<()> = Mutex::new(());

const RUNTIME_INITIALIZED: i32 = 0;
const RUNTIME_ALREADY_INITIALIZED: i32 = 6;

/// Capability proving exclusive access to the process-global Gambit runtime.
pub(super) struct NativeRuntimeAccess {
    _guard: MutexGuard<'static, ()>,
}

pub(super) fn native_runtime_access() -> Result<NativeRuntimeAccess, ()> {
    NATIVE_RUNTIME
        .lock()
        .map(|guard| NativeRuntimeAccess { _guard: guard })
        .map_err(|_| ())
}

/// Classifies the idempotent initialization receipts owned by the native shim.
pub(super) const fn native_runtime_status_is_ready(status: i32) -> bool {
    matches!(status, RUNTIME_INITIALIZED | RUNTIME_ALREADY_INITIALIZED)
}
