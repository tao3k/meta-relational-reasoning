#![forbid(unsafe_code)]

//! Bounded closure admission over the Scheme-owned finite solver.

mod api;

pub use api::{
    ClosureConfig, ClosureError, ClosureLimits, ClosureReceipt, ClosureStatus, DerivationCandidate,
    DerivationReceiptDigest,
};

#[cfg(feature = "native-inference")]
pub use api::evaluate_transitive_closure;
