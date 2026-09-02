use crate::{GqlSyntax, SyntaxKind};
use rowan::Language;

#[test]
fn every_syntax_kind_round_trips_through_rowan() {
    for (raw, kind) in SyntaxKind::ALL.iter().copied().enumerate() {
        assert_eq!(kind as u16, raw as u16, "kind list must follow repr(u16)");
        assert_eq!(GqlSyntax::kind_from_raw(GqlSyntax::kind_to_raw(kind)), kind);
    }

    assert_eq!(
        GqlSyntax::kind_from_raw(rowan::SyntaxKind(u16::MAX)),
        SyntaxKind::Unknown
    );
}
