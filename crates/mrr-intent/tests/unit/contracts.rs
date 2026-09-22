use crate::{IntentBindingStatus, IntentBundleBinding, IntentProjectionError, IntentSemanticModel};
use mrr_identity::ReasoningBundleId;

const INTENT: &str = include_str!("../../../../fixtures/org/runtime-lifecycle.org");

#[test]
fn constraint_change_marks_old_bundle_binding_stale() {
    let first = IntentSemanticModel::project_org(INTENT).expect("intent fixture");
    let bundle = ReasoningBundleId::from_canonical_bytes(b"bundle:runtime").unwrap();
    let binding = IntentBundleBinding::select(&first, bundle);
    assert_eq!(binding.status(&first), IntentBindingStatus::Current);
    let changed = IntentSemanticModel::project_org(&INTENT.replace(
        "Never admit a stale generation response.",
        "Never admit a stale or incomplete generation response.",
    ))
    .expect("changed intent");
    assert_ne!(first.digest(), changed.digest());
    assert_eq!(binding.status(&changed), IntentBindingStatus::Stale);
}

#[test]
fn unknown_or_missing_sections_fail_closed() {
    assert!(matches!(
        IntentSemanticModel::project_org("* Intent\n** Unknown\n- value"),
        Err(IntentProjectionError::InvalidSection(_))
    ));
    assert_eq!(
        IntentSemanticModel::project_org("* Intent\n** Goal\n- only"),
        Err(IntentProjectionError::MissingRequiredContent)
    );
}
