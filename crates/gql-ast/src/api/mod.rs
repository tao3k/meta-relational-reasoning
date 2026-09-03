//! Abstract syntax model and lowering entrypoint for GQL source trees.
#![forbid(unsafe_code)]

mod aggregate_lowering;
mod data_management_lowering;
mod general_literal_lowering;
mod identifier_lowering;
mod label_lowering;
mod lowering;
mod lowering_support;
mod numeric_lowering;
mod order_page_lowering;
mod pattern_graph_lowering;
mod pattern_lowering;
mod predicate_lowering;
mod primitive_query_lowering;
mod types;
mod value_type_predicate_lowering;

pub use lowering::lower_from_syntax;
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
