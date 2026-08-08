#![forbid(unsafe_code)]

use gql_catalog::RelationName;
use gql_types::{Value, ValueType};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Binding {
    pub name: String,
    pub value_type: ValueType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationScan {
    pub relation: RelationName,
    pub bindings: Vec<Binding>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct QueryBlock {
    pub scans: Vec<RelationScan>,
    pub predicates: Vec<Predicate>,
    pub projection: Vec<Binding>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Predicate {
    Equals(Binding, Value),
    IsNull(Binding),
}
