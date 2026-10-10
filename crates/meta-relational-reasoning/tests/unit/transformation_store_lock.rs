use super::TransformationResultStore;

#[test]
fn owner_drop_releases_lock_even_when_a_child_retains_its_description() {
    let path = std::env::temp_dir().join(format!("mrr-inherited-lock-{}.cbor", std::process::id()));
    let limits = crate::TransformationLimits {
        max_bytes: 16384.try_into().unwrap(),
        max_schema_nodes: 32.try_into().unwrap(),
        max_schema_depth: 8.try_into().unwrap(),
        max_dependencies: 16.try_into().unwrap(),
        max_steps: 4.try_into().unwrap(),
    };
    let store = TransformationResultStore::open(&path, limits).unwrap();
    // File cloning shares the same open description, as fork does before exec.
    let inherited = store._lock.0.try_clone().unwrap();
    drop(store);
    let reopened = TransformationResultStore::open(&path, limits).unwrap();
    assert!(TransformationResultStore::open(&path, limits).is_err());
    drop(reopened);
    drop(inherited);
    std::fs::remove_file(path.with_extension("lock")).unwrap();
}
