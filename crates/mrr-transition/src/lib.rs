//! Snapshot transitions over immutable MRR fact identities.
#![forbid(unsafe_code)]
mod api;
mod safety;
mod state;
pub use api::{Transition, TransitionError};
pub use mrr_identity::{ActionId, FactId, GenerationId, StateId, TransitionId};
pub use mrr_relation::Fact;
pub use safety::{
    CounterexampleIr, SafetyCheckReceipt, SafetyLimits, SafetyStatus, TransitionStep, check_safety,
};
pub use state::{
    Action, Effect, InitialState, Invariant, Precondition, StatePredicate, StateSchema,
    StateSnapshot, TransitionModelError, TransitionSystem,
};
#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
