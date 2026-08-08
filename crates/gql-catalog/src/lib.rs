#![forbid(unsafe_code)]

use gql_types::ValueType;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RelationName(pub String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RelationAuthority {
    Asserted { source: String },
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
