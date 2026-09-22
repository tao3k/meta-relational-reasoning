//! Fail-closed truth and incompleteness projection for public MRR receipts.

use mrr_intent::IntentBindingStatus;
use mrr_logic::{WhyNotIncomplete, WhyNotStatus};
use mrr_transition::SafetyStatus;

/// Public truth state for authority-bearing MRR conclusions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TruthStatus {
    True,
    False,
    Unknown,
    Incomplete,
    Stale,
    Conflict,
}

/// Projects a WHY-NOT status without treating incomplete evidence as false.
#[must_use]
pub const fn why_not_truth(status: &WhyNotStatus) -> TruthStatus {
    match status {
        WhyNotStatus::Proven => TruthStatus::True,
        WhyNotStatus::MissingPremises { .. } | WhyNotStatus::NoApplicableRule => TruthStatus::False,
        WhyNotStatus::Incomplete(WhyNotIncomplete::EvidenceCoverage) => TruthStatus::Unknown,
        WhyNotStatus::Incomplete(
            WhyNotIncomplete::Cycle
            | WhyNotIncomplete::DepthBudget
            | WhyNotIncomplete::ExpansionBudget
            | WhyNotIncomplete::AlternativeBudget,
        ) => TruthStatus::Incomplete,
    }
}

/// Projects a safety receipt without treating a budget stop as safe.
#[must_use]
pub const fn safety_truth(status: SafetyStatus) -> TruthStatus {
    match status {
        SafetyStatus::Safe => TruthStatus::True,
        SafetyStatus::Unsafe => TruthStatus::False,
        SafetyStatus::Incomplete => TruthStatus::Incomplete,
    }
}

/// Projects intent selection freshness.
#[must_use]
pub const fn intent_binding_truth(status: IntentBindingStatus) -> TruthStatus {
    match status {
        IntentBindingStatus::Current => TruthStatus::True,
        IntentBindingStatus::Stale => TruthStatus::Stale,
    }
}

/// Projects an explicit authority conflict.
#[must_use]
pub const fn conflict_truth(has_conflict: bool) -> TruthStatus {
    if has_conflict {
        TruthStatus::Conflict
    } else {
        TruthStatus::True
    }
}
