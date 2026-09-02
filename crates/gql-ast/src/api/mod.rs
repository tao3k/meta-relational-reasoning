//! Abstract syntax model and lowering entrypoint for GQL source trees.
#![forbid(unsafe_code)]

mod data_management_lowering;
mod general_literal_lowering;
mod identifier_lowering;
mod label_lowering;
mod lowering;
mod numeric_lowering;
mod pattern_graph_lowering;
mod pattern_lowering;
mod types;

pub use lowering::lower_from_syntax;
pub use types::{
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
};
