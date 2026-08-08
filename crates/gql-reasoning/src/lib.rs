#![forbid(unsafe_code)]

use gql_catalog::RelationName;
use gql_types::{Value, ValueType};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Fact {
    pub predicate: RelationName,
    pub values: Vec<Value>,
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClosureStatus {
    Complete,
    OutputTruncated,
    BudgetExhausted,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivationRequest<'a> {
    pub predicate: &'a RelationName,
    pub bindings: &'a [Option<Value>],
    pub facts: &'a [Fact],
    pub snapshot: &'a str,
    pub limits: DerivationLimits,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivationWitness {
    pub provider: String,
    pub ruleset: String,
    pub snapshot: String,
    pub derivation_id: u64,
    pub support_set_id: u64,
    pub support: Vec<Fact>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivedTuple {
    pub values: Vec<Value>,
    pub witness: DerivationWitness,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivationResult {
    pub tuples: Vec<DerivedTuple>,
    pub closure: ClosureStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DerivationError {
    UnknownPredicate(RelationName),
    InvalidBindingArity { expected: usize, actual: usize },
    BudgetExhausted,
}

pub trait DerivedRelationProvider {
    fn predicates(&self) -> &[gql_catalog::PredicateDescriptor];
    fn derive(&self, request: DerivationRequest<'_>) -> Result<DerivationResult, DerivationError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DerivationId(pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuleId(pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FactId(pub u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredicateDescriptor {
    pub name: RelationName,
    pub columns: Vec<ValueType>,
}
