//! Process-global serialization boundary for the embedded Gambit runtime.

use std::sync::{Mutex, MutexGuard};

static NATIVE_RUNTIME: Mutex<()> = Mutex::new(());

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
