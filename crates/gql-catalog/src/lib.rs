//! Catalog-facing contracts for backend-neutral GQL catalog identity.
#![forbid(unsafe_code)]

mod api;

pub use api::{
    CatalogName,
    EdgeTypeName,
    GqlCatalog,
    GraphName,
    NodeTypeName,
    PredicateDescriptor,
    RelationAuthority,
    RelationIdentity,
    RelationName,
    SchemaName,
};
