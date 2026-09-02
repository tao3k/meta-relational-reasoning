//! Internal IR surface shared by analysis and compiler passes.
#![forbid(unsafe_code)]

mod api;

pub use api::{
    AggregateFunction, BinaryOperator, Binding, CaseBranch, CatalogCommand, CatalogCreatePolicy,
    CatalogDropPolicy, CatalogObjectName, ClosedReferenceTypeSpecification, DeclaredTypeParameter,
    DeclaredValueType, DeclaredValueTypeForm, EdgeDirection, EdgePattern, EdgeTypeSpecification,
    Expression, GraphPattern, GraphPatternElement, GraphTypeSource, GraphTypeSpecification,
    LabelExpression, LetBinding, Mutation, NodePattern, NodeTypeReference, NodeTypeSpecification,
    OptionalMatch, PathMode, PathPattern, PathQuantifier, ProcedureCommand, Projection,
    PropertyConstraint, PropertyType, QueryBlock, RecordField, ReferenceValueTypeKind,
    SessionCommand, SetOperation, SetOperator, SortDirection, SortKey, TransactionAccessMode,
    TransactionCommand, UnaryOperator,
};
