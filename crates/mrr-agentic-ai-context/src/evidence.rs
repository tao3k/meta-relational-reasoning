//! Pure evidence policy shared by native admission and source extraction.

use mrr_relation::EvidenceCompleteness;

pub(crate) enum EvidenceAdmission {
    Accepted,
    Invalid,
    Incomplete,
}

pub(crate) fn admit_evidence(
    valid: bool,
    coverage: EvidenceCompleteness,
    require_complete: bool,
) -> EvidenceAdmission {
    if !valid {
        return EvidenceAdmission::Invalid;
    }
    match coverage {
        EvidenceCompleteness::Complete => EvidenceAdmission::Accepted,
        EvidenceCompleteness::Partial | EvidenceCompleteness::Unknown => {
            if require_complete {
                EvidenceAdmission::Incomplete
            } else {
                EvidenceAdmission::Accepted
            }
        }
    }
}

pub(crate) fn merge_completeness(
    left: EvidenceCompleteness,
    right: EvidenceCompleteness,
) -> EvidenceCompleteness {
    match (left, right) {
        (EvidenceCompleteness::Unknown, _) | (_, EvidenceCompleteness::Unknown) => {
            EvidenceCompleteness::Unknown
        }
        (EvidenceCompleteness::Partial, _) | (_, EvidenceCompleteness::Partial) => {
            EvidenceCompleteness::Partial
        }
        _ => EvidenceCompleteness::Complete,
    }
}
