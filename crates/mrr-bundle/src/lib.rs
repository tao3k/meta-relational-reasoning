//! Validated portable composition boundary for MRR contracts.
#![forbid(unsafe_code)]
mod api;
pub use api::{
    BundleError, InverseGoal, LineagePolicy, ProjectionPolicy, QueryTemplate, ReasoningBundle,
    ReasoningBundleDeclaration, RulePack, TransitionSystem, ValidationProfile,
};
pub use mrr_logic::Rule;
pub use mrr_relation::{Fact, RelationError, RelationSchema};
pub use mrr_transition::Transition;
#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
