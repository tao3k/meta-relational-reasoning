//! Public abstract syntax model and lowering entrypoint for GQL.
#![forbid(unsafe_code)]

mod api;

pub use api::{
    BinaryOperator, CaseBranch, CatalogCreatePolicy, CatalogDropPolicy, CatalogObjectName,
    CatalogStatement, CharacterStringForm, CharacterStringLiteral,
    ClosedReferenceTypeSpecification, EdgeDirection, EdgeKind, EdgePattern, EdgeTypeSpecification,
    Expression, GraphPattern, GraphTypeSource, GraphTypeSpecification, Identifier, IdentifierForm,
    LabelExpression, LetBinding, MatchClause, NestedGraphTypeSpecification, NodePattern,
    NodeTypeReference, NodeTypeSpecification, PathMode, PathPattern, PathQuantifier,
    PatternElement, ProcedureCall, PropertyConstraint, PropertyType, PropertyValueType,
    PropertyValueTypeForm, Query, QueryClause, RecordField, ReferenceValueTypeKind,
    ReturnProjection, SessionCommand, SetItem, SortDirection, SortKey, Statement,
    SyntaxParseOutput, TransactionAccessMode, TransactionCommand, TypeParameter, UnaryOperator,
    lower_from_syntax,
};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
