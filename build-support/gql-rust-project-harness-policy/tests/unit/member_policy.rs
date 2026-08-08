use std::fs;
use std::path::Path;
use gql_rust_project_harness_policy::gql_workspace_member_policies;

const FORBIDDEN_POLICY_RULE_FILE: &str = "rust-project-harness-rules.toml";
const PROHIBITED_POLICY_DISABLE_TOKENS: [&str; 3] = [
    ".with_disabled_rule_pack",
    ".with_disabled_rules",
    ".with_disabled_rule",
];

fn is_policy_scan_exempt_file(path: &Path) -> bool {
    let mut components = path.components().map(|c| c.as_os_str()).collect::<Vec<_>>();
    use std::ffi::OsStr;
    let expected = [
        OsStr::new("build-support"),
        OsStr::new("gql-rust-project-harness-policy"),
        OsStr::new("tests"),
        OsStr::new("unit"),
        OsStr::new("member_policy.rs"),
    ];
    if components.len() < expected.len() {
        return false;
    }
    components
        .split_off(components.len() - expected.len())
        .iter()
        .zip(expected.iter())
        .all(|(left, right)| left == right)
}

fn workspace_root_from_manifest() -> std::path::PathBuf {
    let support_manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    support_manifest
        .ancestors()
        .find(|dir| dir.join("Cargo.toml").exists() && dir.join("crates").is_dir())
        .unwrap_or_else(|| {
            panic!(
                "expected to find workspace root from {}",
                support_manifest.display()
            )
        })
        .to_owned()
}

fn scan_workspace_rs_sources(dir: &Path, files: &mut Vec<std::path::PathBuf>) {
    let entries = std::fs::read_dir(dir).expect("workspace source directory exists");
    for entry in entries {
        let entry = entry.expect("workspace source directory entry");
        let path = entry.path();
        let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or("");
        if file_name == ".git" || file_name == "target" || file_name == "node_modules" {
            continue;
        }
        if path.is_dir() {
            scan_workspace_rs_sources(&path, files);
            continue;
        }
        if is_policy_scan_exempt_file(&path) {
            continue;
        }
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext == "rs")
        {
            files.push(path);
        }
    }
}

fn collect_forbidden_policy_rule_files(dir: &Path, files: &mut Vec<std::path::PathBuf>) {
    let entries = std::fs::read_dir(dir).expect("workspace source directory exists");
    for entry in entries {
        let entry = entry.expect("workspace source directory entry");
        let path = entry.path();
        let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or("");

        if path.is_dir() {
            if file_name == ".git" || file_name == "target" || file_name == "node_modules" {
                continue;
            }
            collect_forbidden_policy_rule_files(&path, files);
            continue;
        }

        if file_name == FORBIDDEN_POLICY_RULE_FILE {
            files.push(path);
        }
    }
}

#[test]
fn central_policy_registry_contains_migrated_member_crates() {
    let support_manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = support_manifest
        .ancestors()
        .find(|dir| dir.join("Cargo.toml").exists() && dir.join("crates").is_dir())
        .unwrap_or_else(|| {
            panic!(
                "expected to find workspace root from {}",
                support_manifest.display()
            )
        });
    let crates_dir = workspace_root.join("crates");

    let mut registered: Vec<String> = gql_workspace_member_policies()
        .iter()
        .map(|policy| policy.package_name.to_string())
        .collect();
    registered.sort_unstable();

    let mut declared_in_crates_dir: Vec<String> = fs::read_dir(&crates_dir)
        .expect("crates directory exists")
        .filter_map(|entry| {
            let entry = entry.expect("crates dir entry").path();
            if !entry.is_dir() {
                return None;
            }
            if !entry.join("Cargo.toml").exists() {
                return None;
            }
            entry
                .file_name()?
                .to_str()
                .map(|name| name.to_string())
        })
        .collect();
    declared_in_crates_dir.sort_unstable();

    assert_eq!(registered, declared_in_crates_dir);
}

#[test]
fn central_policy_uses_uniform_workspace_default() {
    let policies = gql_workspace_member_policies();
    assert!(!policies.is_empty());
    for policy in policies {
        assert!(
            policy.verification_label.is_none(),
            "policy label is explicit for {}",
            policy.package_name
        );
        assert!(
            policy.criterion_performance_verification,
            "performance verification is disabled for {}",
            policy.package_name
        );
        assert!(
            !policy.cargo_check_advice_allow_explanation.contains("per-crate exception"),
            "package {} should not keep per-crate exception text",
            policy.package_name
        );
        assert!(
            policy.cargo_check_advice_allow_explanation.contains("workspace crate policy is enforced"),
            "policy rationale should be workspace-wide for {}",
            policy.package_name
        );
        assert!(
            policy.cargo_check_advice_allow_explanation.contains("cleanup_trigger="),
            "policy should include cleanup trigger for {}",
            policy.package_name
        );
        assert!(
            policy
                .cargo_check_advice_allow_explanation
                .is_ascii(),
            "policy explanation should remain ASCII-only for {}",
            policy.package_name
        );
        assert!(
            policy.latency_sensitive_performance_owners.len() == 1,
            "uniform latency owners should be a single shared owner for {}",
            policy.package_name
        );
        assert!(
            policy.availability_stability_owners.len() == 1,
            "uniform stability owners should be a single shared owner for {}",
            policy.package_name
        );
        assert_eq!(
            policy.crate_root,
            format!("crates/{}", policy.package_name),
            "policy crate_root is inconsistent for {}",
            policy.package_name
        );
    }
}

#[test]
fn all_workspace_code_paths_keep_policy_enforcement_enabled() {
    let workspace_root = workspace_root_from_manifest();
    let mut source_files = Vec::new();
    scan_workspace_rs_sources(&workspace_root.join("build-support/gql-rust-project-harness-policy"), &mut source_files);
    scan_workspace_rs_sources(&workspace_root.join("crates"), &mut source_files);

    for path in source_files {
        let text = fs::read_to_string(&path).expect("workspace rust source readable");
        for token in &PROHIBITED_POLICY_DISABLE_TOKENS {
            assert!(
                !text.contains(token),
                "found disabled-policy token {} in {}",
                token,
                path.display()
            );
        }
    }
}

#[test]
fn no_policy_override_rules_file_is_present() {
    let workspace_root = workspace_root_from_manifest();
    let mut hits = Vec::new();
    collect_forbidden_policy_rule_files(&workspace_root, &mut hits);
    assert!(hits.is_empty(), "escape hatch file exists: {:?}", hits);
}

#[test]
fn all_workspace_crate_manifests_enable_workspace_policy_in_build_and_dev_dependencies() {
    let support_manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = support_manifest
        .ancestors()
        .find(|dir| dir.join("Cargo.toml").exists() && dir.join("crates").is_dir())
        .unwrap_or_else(|| {
            panic!(
                "expected to find workspace root from {}",
                support_manifest.display()
            )
        });
    let crates_dir = workspace_root.join("crates");
    let has_policy_in_section = |manifest_text: &str, section: &str| -> bool {
        let mut in_section = false;
        for line in manifest_text.lines() {
            if line.starts_with('[') {
                in_section = line.trim() == section;
                continue;
            }
            if in_section && line.trim() == "gql-rust-project-harness-policy.workspace = true" {
                return true;
            }
        }
        false
    };

    for crate_dir in fs::read_dir(&crates_dir).expect("crates directory exists") {
        let crate_dir = crate_dir.expect("crates dir entry").path();
        if !crate_dir.is_dir() {
            continue;
        }
        let entry = crate_dir.join("Cargo.toml");
        if !entry.exists() {
            continue;
        }

        let manifest_text = fs::read_to_string(&entry).expect("manifest readable");
        assert!(
            manifest_text.contains("gql-rust-project-harness-policy.workspace = true"),
            "missing policy dependency in {}",
            entry.display()
        );
        assert!(
            manifest_text.contains("[build-dependencies]"),
            "missing [build-dependencies] section in {}",
            entry.display()
        );
        assert!(
            has_policy_in_section(&manifest_text, "[build-dependencies]"),
            "missing build-dependency policy in {}",
            entry.display()
        );
        assert!(
            has_policy_in_section(&manifest_text, "[dev-dependencies]"),
            "missing dev-dependency policy in {}",
            entry.display()
        );
    }
}

#[test]
fn all_workspace_crate_build_scripts_call_workspace_policy_gate() {
    let support_manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = support_manifest
        .ancestors()
        .find(|dir| dir.join("Cargo.toml").exists() && dir.join("crates").is_dir())
        .unwrap_or_else(|| {
            panic!(
                "expected to find workspace root from {}",
                support_manifest.display()
            )
        });
    let crates_dir = workspace_root.join("crates");
    for crate_dir in fs::read_dir(&crates_dir).expect("crates directory exists") {
        let crate_dir = crate_dir.expect("crates dir entry").path();
        if !crate_dir.is_dir() {
            continue;
        }
        let manifest_path = crate_dir.join("Cargo.toml");
        if !manifest_path.exists() {
            continue;
        }

        let build_rs = manifest_path.with_file_name("build.rs");
        assert!(
            build_rs.exists(),
            "missing build.rs in {}",
            manifest_path.parent().unwrap_or_else(|| Path::new("")).display()
        );
        let build_rs_text = fs::read_to_string(&build_rs)
            .unwrap_or_else(|_| panic!("build.rs should exist in {}", manifest_path.parent().unwrap().display()));
        assert!(
            build_rs_text
                .contains("assert_gql_rust_project_harness_member_policy_from_env"),
            "missing policy gate call in {}",
            build_rs.display()
        );
        for token in &PROHIBITED_POLICY_DISABLE_TOKENS {
            assert!(
                !build_rs_text.contains(token),
                "disabled policy token {} in {}",
                token,
                build_rs.display()
            );
        }
    }
}
