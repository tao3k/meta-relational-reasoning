use crate::{
    ExternalRevisionIdentity, GenerationId, RevisionBinding, RevisionBindingError,
    SemanticSnapshot, SemanticSnapshotError, StateId,
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

fn binding(
    generation: GenerationId,
    provider: &str,
    logical_change: &str,
    content_revision: &str,
) -> RevisionBinding {
    RevisionBinding::admit(
        ExternalRevisionIdentity::new(provider, logical_change, content_revision)
            .expect("external revision"),
        generation,
    )
    .expect("revision binding")
}

#[test]
fn semantic_snapshot_is_order_independent_and_generation_bound() {
    let generation = GenerationId::from_canonical_bytes(b"snapshot-generation").unwrap();
    let first = binding(generation, "git", "repository-a", "commit-a");
    let second = binding(generation, "git", "repository-b", "commit-b");
    let left = SemanticSnapshot::admit(generation, vec![first.clone(), second.clone()]).unwrap();
    let right = SemanticSnapshot::admit(generation, vec![second, first]).unwrap();
    assert_eq!(left, right);

    let other = GenerationId::from_canonical_bytes(b"other-generation").unwrap();
    let wrong = binding(other, "git", "repository-c", "commit-c");
    assert!(matches!(
        SemanticSnapshot::admit(generation, vec![wrong]),
        Err(SemanticSnapshotError::GenerationMismatch {
            expected,
            actual,
            ..
        }) if expected == generation && actual == other
    ));
}

#[test]
fn semantic_snapshot_rejects_duplicate_and_ambiguous_sources() {
    let generation = GenerationId::from_canonical_bytes(b"snapshot-duplicates").unwrap();
    let first = binding(generation, "git", "repository", "commit-a");
    assert_eq!(
        SemanticSnapshot::admit(generation, vec![first.clone(), first.clone()]),
        Err(SemanticSnapshotError::DuplicateRevision(first.revision()))
    );

    let next = binding(generation, "git", "repository", "commit-b");
    assert_eq!(
        SemanticSnapshot::admit(generation, vec![first, next]),
        Err(SemanticSnapshotError::ConflictingLogicalChange {
            provider: "git".into(),
            logical_change: "repository".into(),
        })
    );
}
