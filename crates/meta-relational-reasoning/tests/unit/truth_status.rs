use crate::{
    GroundAtom, IntentBindingStatus, RelationId, SafetyStatus, TruthStatus, Value,
    WhyNotIncomplete, WhyNotStatus, conflict_truth, intent_binding_truth, safety_truth,
    why_not_truth,
};

fn missing() -> WhyNotStatus {
    WhyNotStatus::MissingPremises {
        alternatives: vec![vec![GroundAtom::new(
            RelationId::from_canonical_bytes(b"truth:missing").expect("relation identity"),
            vec![Value::Boolean(true)],
        )]],
    }
}

#[test]
fn truth_projection_is_fail_closed_for_unknown_and_incomplete_results() {
    assert_eq!(why_not_truth(&WhyNotStatus::Proven), TruthStatus::True);
    assert_eq!(why_not_truth(&missing()), TruthStatus::False);
    assert_eq!(
        why_not_truth(&WhyNotStatus::NoApplicableRule),
        TruthStatus::False
    );
    assert_eq!(
        why_not_truth(&WhyNotStatus::Incomplete(
            WhyNotIncomplete::EvidenceCoverage
        )),
        TruthStatus::Unknown
    );
    for reason in [
        WhyNotIncomplete::Cycle,
        WhyNotIncomplete::DepthBudget,
        WhyNotIncomplete::ExpansionBudget,
        WhyNotIncomplete::AlternativeBudget,
    ] {
        assert_eq!(
            why_not_truth(&WhyNotStatus::Incomplete(reason)),
            TruthStatus::Incomplete
        );
    }

    assert_eq!(safety_truth(SafetyStatus::Safe), TruthStatus::True);
    assert_eq!(safety_truth(SafetyStatus::Unsafe), TruthStatus::False);
    assert_eq!(
        safety_truth(SafetyStatus::Incomplete),
        TruthStatus::Incomplete
    );
    assert_ne!(safety_truth(SafetyStatus::Incomplete), TruthStatus::True);
}

#[test]
fn stale_and_conflict_are_not_collapsed_into_false() {
    assert_eq!(
        intent_binding_truth(IntentBindingStatus::Current),
        TruthStatus::True
    );
    assert_eq!(
        intent_binding_truth(IntentBindingStatus::Stale),
        TruthStatus::Stale
    );
    assert_eq!(conflict_truth(true), TruthStatus::Conflict);
    assert_eq!(conflict_truth(false), TruthStatus::True);
}
