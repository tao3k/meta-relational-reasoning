#![forbid(unsafe_code)]

use gql_types::{Value, ValueType};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RelationName(pub String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RelationAuthority {
    Asserted { source: String },
    Derived { provider: String, ruleset: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredicateDescriptor {
    pub name: RelationName,
    pub columns: Vec<ValueType>,
    pub authority: RelationAuthority,
}

pub trait GqlCatalog {
    fn relation(&self, name: &RelationName) -> Option<PredicateDescriptor>;
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Fact {
    pub predicate: RelationName,
    pub values: Vec<Value>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DerivationLimits {
    pub max_results: usize,
}

impl Default for DerivationLimits {
    fn default() -> Self {
        Self { max_results: 1_000 }
    }
}

#[derive(Clone, Debug)]
pub struct DerivationRequest<'a> {
    pub predicate: &'a RelationName,
    pub bindings: &'a [Option<Value>],
    pub facts: &'a [Fact],
    pub snapshot: &'a str,
    pub limits: DerivationLimits,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClosureStatus {
    Complete,
    ResultBoundReached,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivationWitness {
    pub provider: String,
    pub ruleset: String,
    pub snapshot: String,
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
}

pub trait DerivedRelationProvider {
    fn predicates(&self) -> &[PredicateDescriptor];
    fn derive(&self, request: DerivationRequest<'_>) -> Result<DerivationResult, DerivationError>;
}
