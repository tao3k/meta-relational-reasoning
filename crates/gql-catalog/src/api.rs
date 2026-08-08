//! Catalog identity and predicate contracts used by semantic and IR layers.

use gql_types::ValueType;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// Catalog name identifier.
pub struct CatalogName(pub String);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// Graph name identifier.
pub struct GraphName(pub String);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// Schema name identifier.
pub struct SchemaName(pub String);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// Node type name identifier.
pub struct NodeTypeName(pub String);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// Edge type name identifier.
pub struct EdgeTypeName(pub String);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// Relation name identifier.
pub struct RelationName(pub String);

#[derive(Clone, Debug, Eq, PartialEq)]
/// Canonical relation identity coordinates.
pub struct RelationIdentity {
    /// Catalog component.
    pub catalog: CatalogName,
    /// Graph component.
    pub graph: GraphName,
    /// Optional schema component.
    pub schema: Option<SchemaName>,
    /// Node types participating in the relation.
    pub node_types: Vec<NodeTypeName>,
    /// Edge types participating in the relation.
    pub edge_types: Vec<EdgeTypeName>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Source of authority for a relation declaration.
pub enum RelationAuthority {
    /// Relation is explicitly asserted in catalog data.
    Asserted { source: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Descriptor for a resolved relation.
pub struct PredicateDescriptor {
    /// Relation name.
    pub name: RelationName,
    /// Output columns.
    pub columns: Vec<ValueType>,
    /// Declaring authority.
    pub authority: RelationAuthority,
    /// Canonical relation identity.
    pub relation_identity: RelationIdentity,
}

/// Source-owned catalog lookup abstraction.
pub trait GqlCatalog {
    /// Resolve a relation by name.
    fn relation(&self, name: &RelationName) -> Option<PredicateDescriptor>;
}
