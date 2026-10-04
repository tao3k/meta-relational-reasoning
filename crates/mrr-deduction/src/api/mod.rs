//! Exposes typed admission contracts and optional native execution.

#[cfg(feature = "native-inference")]
mod execution;
mod model;

pub use model::{
    ClosureConfig, ClosureError, ClosureLimits, ClosureReceipt, ClosureStatus, DerivationCandidate,
    DerivationReceiptDigest,
};

#[cfg(feature = "native-inference")]
pub use execution::evaluate_transitive_closure;
