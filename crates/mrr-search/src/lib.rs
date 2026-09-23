//! Backend-neutral Search factor reasoning and causal Impact projection.
#![forbid(unsafe_code)]

mod reasoning;

pub use reasoning::{
    SearchFactor, SearchFactorEdge, SearchFactorRole, SearchFrameworkError, SearchFrameworkLimits,
    SearchFrameworkReceipt, SearchFrameworkStatus, SearchInfluence, SearchObservation,
    SearchReasoningDigest, evaluate_search_factors,
};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
