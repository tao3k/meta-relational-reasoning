//! Exposes typed admission contracts and optional native execution.

#[cfg(any(feature = "native-inference", feature = "worker-inference"))]
mod execution;
mod model;

pub use model::{
    ClosureConfig, ClosureError, ClosureLimits, ClosureReceipt, ClosureStatus, DerivationCandidate,
    DerivationReceiptDigest,
};

#[cfg(any(feature = "native-inference", feature = "worker-inference"))]
pub use execution::evaluate_transitive_closure;
