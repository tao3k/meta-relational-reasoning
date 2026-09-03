//! Internal IR surface shared by analysis and compiler passes.
#![forbid(unsafe_code)]

mod api;

pub use api::{
    AggregateFunction, BinaryOperator, Binding, CaseBranch, CatalogCommand, CatalogCreatePolicy,
    CatalogDropPolicy, CatalogObjectName, ClosedReferenceTypeSpecification, DeclaredTypeParameter,
    DeclaredValueType, DeclaredValueTypeForm, EdgeDirection, EdgePattern, EdgeTypeSpecification,
    ElementIdentityKind, EndpointKind, Expression, ForBinding, ForPositionBinding, ForPositionKind,
    GraphMatch, GraphMatchMode, GraphPattern, GraphPatternElement, GraphTypeSource,
    GraphTypeSpecification, LabelExpression, LetBinding, Mutation, NodePattern, NodeTypeReference,
    NodeTypeSpecification, NonNegativeIntegerSpecification, NullOrdering, OptionalMatch, PathMode,
    PathPattern, PathPrefix, PathQuantifier, PathSearch, ProcedureCommand, Projection,
    PropertyConstraint, PropertyType, QueryBlock, RecordField, ReferenceValueTypeKind,
    SessionCommand, SetOperation, SetOperator, SetQuantifier, SortDirection, SortKey,
    TransactionAccessMode, TransactionCommand, UnaryOperator,
};
