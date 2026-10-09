//! Backend-neutral Search factor reasoning and causal Impact projection.
#![forbid(unsafe_code)]

mod dispatch;
pub use dispatch::{
    SearchDispatch, SearchDispatchError, SearchDispatchLease, SearchDispatchReceipt,
    SearchDispatchResources, SearchDispatchSnapshot,
};
mod poo;
mod reasoning;
pub use poo::PooSearchProjection;
#[cfg(feature = "native-inference")]
pub use poo::{PooSearchPlan, PooSearchRole, compile_poo_search_plan};

pub use reasoning::{
    SearchFactor, SearchFactorEdge, SearchFactorRole, SearchFrameworkError, SearchFrameworkLimits,
    SearchFrameworkReceipt, SearchFrameworkStatus, SearchInfluence, SearchObservation,
    SearchReasoningDigest,
};

#[cfg(all(test, feature = "native-inference"))]
#[path = "../tests/unit/mod.rs"]
mod tests;

#[cfg(feature = "native-inference")]
pub use reasoning::{evaluate_poo_search_factors, evaluate_search_factors};

#[cfg(feature = "native-inference")]
pub use mrr_gerbil::reserve_native_worker_host;
