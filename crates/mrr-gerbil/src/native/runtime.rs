//! Single-owner execution boundary for the embedded Gambit runtime.

use std::sync::{OnceLock, mpsc};
use std::thread;

use super::ffi;

type NativeJob = Box<dyn FnOnce() + Send + 'static>;

static NATIVE_RUNTIME: OnceLock<Result<mpsc::Sender<NativeJob>, NativeRuntimeError>> =
    OnceLock::new();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum NativeRuntimeError {
    Unavailable,
    Status(i32),
}

/// Executes one complete native operation on the unique Gambit owner thread.
///
/// A mutex is insufficient: Rust callers can acquire it from different OS
/// threads, while Gambit's allocation state is thread-affine.
pub(super) fn with_native_runtime<T, F>(operation: F) -> Result<T, NativeRuntimeError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let runtime = NATIVE_RUNTIME.get_or_init(|| {
        let (sender, receiver) = mpsc::channel::<NativeJob>();
        let (ready_sender, ready_receiver) = mpsc::sync_channel(0);
        thread::Builder::new()
            .name("mrr-gerbil-runtime".to_owned())
            .spawn(move || {
                let status = ffi::runtime_init();
                if ready_sender.send(status).is_err() {
                    return;
                }
                if status != 0 {
                    return;
                }
                while let Ok(job) = receiver.recv() {
                    job();
                }
            })
            .map_err(|_| NativeRuntimeError::Unavailable)?;
        match ready_receiver.recv() {
            Ok(0) => Ok(sender),
            Ok(status) => Err(NativeRuntimeError::Status(status)),
            Err(_) => Err(NativeRuntimeError::Unavailable),
        }
    });
    let (result_sender, result_receiver) = mpsc::sync_channel(0);
    runtime
        .as_ref()
        .map_err(|error| *error)?
        .send(Box::new(move || {
            let _ = result_sender.send(Ok(operation()));
        }))
        .map_err(|_| NativeRuntimeError::Unavailable)?;
    result_receiver
        .recv()
        .map_err(|_| NativeRuntimeError::Unavailable)?
}
