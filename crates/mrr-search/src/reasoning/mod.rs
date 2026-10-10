//! Exposes typed admission contracts and optional native execution.

#[cfg(any(feature = "native-inference", feature = "worker-inference"))]
mod execution;
mod model;

pub use model::{
    SearchFactor, SearchFactorEdge, SearchFactorRole, SearchFrameworkError, SearchFrameworkLimits,
    SearchFrameworkReceipt, SearchFrameworkStatus, SearchInfluence, SearchObservation,
    SearchReasoningDigest,
};

#[cfg(any(feature = "native-inference", feature = "worker-inference"))]
pub use execution::{evaluate_poo_search_factors, evaluate_search_factors};
