//! Canonical reasoning contracts for derived predicates and witnessed derivation.
#![forbid(unsafe_code)]

mod api;

pub use api::{
    ClosureStatus, DerivationError, DerivationId, DerivationLimits, DerivationRequest,
    DerivationResult, DerivationWitness, DerivedPredicateDescriptor, DerivedRelationProvider,
    DerivedTuple, Fact, FactId, RelationName, RuleId,
};
