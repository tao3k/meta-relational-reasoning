//! Abstract syntax model and lowering entrypoint for GQL source trees.
#![forbid(unsafe_code)]

#[cfg(feature = "syntax-lowering")]
mod aggregate_lowering;
#[cfg(feature = "syntax-lowering")]
mod data_management_lowering;
#[cfg(feature = "syntax-lowering")]
mod general_literal_lowering;
#[cfg(feature = "syntax-lowering")]
mod identifier_lowering;
#[cfg(feature = "syntax-lowering")]
mod label_lowering;
#[cfg(feature = "syntax-lowering")]
mod lowering;
#[cfg(feature = "syntax-lowering")]
mod lowering_support;
mod numeric_lowering;
#[cfg(feature = "syntax-lowering")]
mod order_page_lowering;
#[cfg(feature = "syntax-lowering")]
mod pattern_graph_lowering;
#[cfg(feature = "syntax-lowering")]
mod pattern_lowering;
#[cfg(feature = "syntax-lowering")]
mod predicate_lowering;
#[cfg(feature = "syntax-lowering")]
mod primitive_query_lowering;
mod types;
#[cfg(feature = "syntax-lowering")]
mod value_type_predicate_lowering;

#[cfg(feature = "syntax-lowering")]
pub use lowering::lower_from_syntax;
pub use numeric_lowering::lower_numeric_literal;
pub use types::{
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
    TransactionCommand, TruthValue, TypeParameter, UnaryOperator,
};
