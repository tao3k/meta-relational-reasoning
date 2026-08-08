//! Catalog-facing contracts for backend-neutral GQL catalog identity.
#![forbid(unsafe_code)]

mod api;

pub use api::{
    Catalog,
    CatalogName,
    EdgeType,
    EdgeTypeName,
    GqlCatalog,
    Graph,
    GraphName,
    GraphTypeName,
    NodeType,
    NodeTypeName,
    PredicateDescriptor,
    Property,
    ProcedureSignature,
    RelationAuthority,
    RelationIdentity,
    RelationName,
    Schema,
    SchemaName,
};
