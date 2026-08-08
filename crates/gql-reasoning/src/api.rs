//! Public model and contracts for derived relation reasoning.
#![forbid(unsafe_code)]

use gql_types::Value;

/// Name of a derived relation owned by the reasoning boundary.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RelationName(pub String);

/// Descriptor for a derived predicate exposed by a reasoning provider.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivedPredicateDescriptor {
    pub name: RelationName,
    pub columns: Vec<gql_types::ValueType>,
}

/// Concrete fact value used by derived relation providers.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Fact {
    pub predicate: RelationName,
    pub values: Vec<Value>,
}

/// Budget limits for derivation execution and witness generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DerivationLimits {
    pub max_results: usize,
    pub max_derived_facts: usize,
    pub max_rule_firings: usize,
    pub max_depth: usize,
    pub wall_budget_ms: u64,
}

impl Default for DerivationLimits {
    fn default() -> Self {
        Self {
            max_results: 1_000,
            max_derived_facts: 100_000,
            max_rule_firings: 10_000,
            max_depth: 128,
            wall_budget_ms: 2_000,
        }
    }
}

/// Closing status for a derivation run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClosureStatus {
    Complete,
    OutputTruncated,
    BudgetExhausted,
    Cancelled,
}

/// Request sent to a derived-relation provider.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivationRequest<'a> {
    pub predicate: &'a RelationName,
    pub bindings: &'a [Option<Value>],
    pub facts: &'a [Fact],
    pub snapshot: &'a str,
    pub limits: DerivationLimits,
}

/// Witness for a derivation tuple.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivationWitness {
    pub provider: String,
    pub ruleset: String,
    pub snapshot: String,
    pub derivation_id: u64,
    pub support_set_id: u64,
    pub support: Vec<Fact>,
}

/// Single derivation tuple with witness data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivedTuple {
    pub values: Vec<Value>,
    pub witness: DerivationWitness,
}

/// Full derivation result for one query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivationResult {
    pub tuples: Vec<DerivedTuple>,
    pub closure: ClosureStatus,
}

/// Error describing derivation execution failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DerivationError {
    UnknownPredicate(RelationName),
    InvalidBindingArity { expected: usize, actual: usize },
    BudgetExhausted,
}

/// Provider of derived relation facts and witnesses.
pub trait DerivedRelationProvider {
    fn predicates(&self) -> &[DerivedPredicateDescriptor];
    fn derive(&self, request: DerivationRequest<'_>) -> Result<DerivationResult, DerivationError>;
}

/// Identifier for a derived relation tuple.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DerivationId(pub u64);

/// Identifier for a derived relation rule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuleId(pub u64);

/// Identifier for a derived relation fact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FactId(pub u64);
