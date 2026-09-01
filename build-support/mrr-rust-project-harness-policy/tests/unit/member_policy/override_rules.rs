use super::{collect_forbidden_policy_rule_files, workspace_root_from_manifest};

#[test]
fn no_policy_override_rules_file_is_present() {
    let workspace_root = workspace_root_from_manifest();
    let mut hits = Vec::new();
    collect_forbidden_policy_rule_files(&workspace_root, &mut hits);
    assert!(hits.is_empty(), "escape hatch file exists: {:?}", hits);
}
