//! Public abstract syntax model and lowering entrypoint for GQL.
#![forbid(unsafe_code)]

mod api;

pub use api::{
    AggregateFunction, BinaryOperator, CaseBranch, CatalogCreatePolicy, CatalogDropPolicy,
    CatalogObjectName, CatalogStatement, CharacterStringForm, CharacterStringLiteral,
    ClosedReferenceTypeSpecification, DynamicParameterReference, EdgeDirection, EdgeKind,
    EdgePattern, EdgeTypeSpecification, ElementIdentityKind, EndpointKind, Expression, ForItem,
    ForOrdinalityBinding, ForOrdinalityKind, GraphMatchMode, GraphPattern, GraphTypeSource,
    GraphTypeSpecification, Identifier, IdentifierForm, LabelExpression, LetBinding, MatchClause,
    NestedGraphTypeSpecification, NodePattern, NodeTypeReference, NodeTypeSpecification,
    NonNegativeIntegerSpecification, NullOrdering, ParameterNameForm, PathMode, PathPattern,
    PathPrefix, PathQuantifier, PathSearch, PathTarget, PatternElement, ProcedureCall,
    PropertyConstraint, PropertyType, PropertyValueType, PropertyValueTypeForm, Query, QueryClause,
    RecordField, ReferenceValueTypeKind, ReturnProjection, SessionCommand, SetItem, SetQuantifier,
    SortDirection, SortKey, Statement, SyntaxParseOutput, TransactionAccessMode,
    TransactionCommand, TruthValue, TypeParameter, UnaryOperator, lower_from_syntax,
};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
