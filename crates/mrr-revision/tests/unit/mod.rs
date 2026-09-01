use crate::{
    ExternalRevisionIdentity, GenerationId, RevisionBinding, RevisionBindingError, StateId,
};

#[test]
fn revision_is_stable_but_distinct_from_generation_and_runtime_state() {
    let external =
        ExternalRevisionIdentity::new("jj", "change-abc", "commit-001").expect("external revision");
    let generation = GenerationId::from_canonical_bytes(b"generation-1").expect("generation");
    let first = RevisionBinding::admit(external.clone(), generation).expect("binding");
    let second = RevisionBinding::admit(external, generation).expect("same binding");
    let state = StateId::from_canonical_bytes(b"generation-1").expect("state");
    assert_eq!(first, second);
    assert_ne!(first.revision().to_string(), generation.to_string());
    assert_ne!(generation.to_string(), state.to_string());
}

#[test]
fn content_change_changes_revision_without_changing_logical_change() {
    let generation = GenerationId::from_canonical_bytes(b"generation-1").expect("generation");
    let first = RevisionBinding::admit(
        ExternalRevisionIdentity::new("jj", "change-abc", "commit-001").unwrap(),
        generation,
    )
    .unwrap();
    let second = RevisionBinding::admit(
        ExternalRevisionIdentity::new("jj", "change-abc", "commit-002").unwrap(),
        generation,
    )
    .unwrap();
    assert_eq!(
        first.external().logical_change(),
        second.external().logical_change()
    );
    assert_ne!(first.revision(), second.revision());
}

#[test]
fn malformed_external_coordinates_fail_closed() {
    assert_eq!(
        ExternalRevisionIdentity::new("", "change", "content"),
        Err(RevisionBindingError::InvalidExternalField("provider"))
    );
}
