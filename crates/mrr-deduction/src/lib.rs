#![forbid(unsafe_code)]

//! Bounded closure admission over the Scheme-owned finite solver.

mod api;

pub use api::{
    ClosureConfig, ClosureError, ClosureLimits, ClosureReceipt, ClosureStatus, DerivationCandidate,
    DerivationReceiptDigest,
};

#[cfg(feature = "native-inference")]
pub use api::evaluate_transitive_closure;

/// Native process Host controls owned by the inference backend.
/// The caller remains responsible for source and external-effect authorization.
#[cfg(feature = "native-inference")]
pub use mrr_gerbil::{
    NativeWorker, NativeWorkerError, TemporalHost, TemporalRuntimeError, configure_native_worker,
    shutdown_native_worker,
};
