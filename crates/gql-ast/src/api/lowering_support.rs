//! Shared syntax classification for AST lowering owners.
#![forbid(unsafe_code)]

use gql_syntax::SyntaxKind;

pub(super) const fn is_expression_kind(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Expression
            | SyntaxKind::NameExpression
            | SyntaxKind::LiteralExpression
            | SyntaxKind::CharacterStringLiteralExpression
            | SyntaxKind::DynamicParameterExpression
            | SyntaxKind::ByteStringLiteralExpression
            | SyntaxKind::TemporalLiteralExpression
            | SyntaxKind::DurationLiteralExpression
            | SyntaxKind::UnaryExpression
            | SyntaxKind::BinaryExpression
            | SyntaxKind::NullPredicateExpression
            | SyntaxKind::TruthPredicateExpression
            | SyntaxKind::ValueTypePredicateExpression
            | SyntaxKind::DirectedPredicateExpression
            | SyntaxKind::EndpointPredicateExpression
            | SyntaxKind::ElementIdentityPredicateExpression
            | SyntaxKind::PropertyExistsPredicateExpression
            | SyntaxKind::LabelPredicateExpression
            | SyntaxKind::PropertyAccessExpression
            | SyntaxKind::FunctionCallExpression
            | SyntaxKind::AggregateFunctionExpression
            | SyntaxKind::ListExpression
            | SyntaxKind::RecordExpression
            | SyntaxKind::SubscriptExpression
            | SyntaxKind::ParenthesizedExpression
            | SyntaxKind::CaseExpression
    )
}
