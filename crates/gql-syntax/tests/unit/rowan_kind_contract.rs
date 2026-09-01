use crate::{GqlSyntax, SyntaxKind};
use rowan::Language;

#[test]
fn every_syntax_kind_round_trips_through_rowan() {
    let kinds = [
        SyntaxKind::SourceFile,
        SyntaxKind::Query,
        SyntaxKind::MatchClause,
        SyntaxKind::WhereClause,
        SyntaxKind::LetClause,
        SyntaxKind::ReturnClause,
        SyntaxKind::GraphPattern,
        SyntaxKind::NodePattern,
        SyntaxKind::PropertyMap,
        SyntaxKind::PropertyEntry,
        SyntaxKind::EdgePattern,
        SyntaxKind::LabelList,
        SyntaxKind::Expression,
        SyntaxKind::NameExpression,
        SyntaxKind::LiteralExpression,
        SyntaxKind::UnaryExpression,
        SyntaxKind::BinaryExpression,
        SyntaxKind::ParenthesizedExpression,
        SyntaxKind::Keyword,
        SyntaxKind::Identifier,
        SyntaxKind::Number,
        SyntaxKind::String,
        SyntaxKind::Whitespace,
        SyntaxKind::Punctuation,
        SyntaxKind::Comment,
        SyntaxKind::Unknown,
        SyntaxKind::PropertyAccessExpression,
        SyntaxKind::PathPattern,
        SyntaxKind::PathQuantifier,
        SyntaxKind::OptionalMatchClause,
        SyntaxKind::ListExpression,
        SyntaxKind::SubscriptExpression,
        SyntaxKind::ProjectionAlias,
        SyntaxKind::UnionClause,
        SyntaxKind::LimitClause,
        SyntaxKind::OrderByClause,
        SyntaxKind::OffsetClause,
        SyntaxKind::CaseExpression,
        SyntaxKind::CaseWhenClause,
        SyntaxKind::CaseElseClause,
    ];

    for (raw, kind) in kinds.into_iter().enumerate() {
        assert_eq!(kind as u16, raw as u16, "kind list must follow repr(u16)");
        assert_eq!(GqlSyntax::kind_from_raw(GqlSyntax::kind_to_raw(kind)), kind);
    }

    assert_eq!(
        GqlSyntax::kind_from_raw(rowan::SyntaxKind(u16::MAX)),
        SyntaxKind::Unknown
    );
}
