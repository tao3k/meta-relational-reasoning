#![forbid(unsafe_code)]

//! Bounded Ascent evaluation for already validated MRR bundles.

mod api;

pub use api::{
    ClosureConfig, ClosureError, ClosureLimits, ClosureReceipt, ClosureStatus, DerivationCandidate,
    DerivationReceiptDigest, evaluate_transitive_closure,
};
