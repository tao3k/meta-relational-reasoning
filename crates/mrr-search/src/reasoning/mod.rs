//! Exposes typed admission contracts and optional native execution.

#[cfg(feature = "native-inference")]
mod execution;
mod model;

pub use model::{
    SearchFactor, SearchFactorEdge, SearchFactorRole, SearchFrameworkError, SearchFrameworkLimits,
    SearchFrameworkReceipt, SearchFrameworkStatus, SearchInfluence, SearchObservation,
    SearchReasoningDigest,
};

#[cfg(feature = "native-inference")]
pub use execution::evaluate_search_factors;
