//! Backend-neutral Search factor reasoning and causal Impact projection.
#![forbid(unsafe_code)]

mod reasoning;

pub use reasoning::{
    SearchFactor, SearchFactorEdge, SearchFactorRole, SearchFrameworkError, SearchFrameworkLimits,
    SearchFrameworkReceipt, SearchFrameworkStatus, SearchInfluence, SearchObservation,
    SearchReasoningDigest,
};

#[cfg(all(test, feature = "native-inference"))]
#[path = "../tests/unit/mod.rs"]
mod tests;

#[cfg(feature = "native-inference")]
pub use reasoning::evaluate_search_factors;

#[cfg(feature = "native-inference")]
pub use mrr_gerbil::reserve_native_worker_host;
