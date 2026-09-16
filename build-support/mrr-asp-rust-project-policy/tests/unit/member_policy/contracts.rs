use mrr_asp_rust_project_policy::mrr_workspace_member_policies;
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const FORBIDDEN_POLICY_RULE_FILE: &str = "rust-project-harness-rules.toml";
pub(super) const ISO_NORMATIVE_SOURCES_FILE: &str = "conformance/iso/normative-sources.yaml";
const MRR_DEPENDENCY_POLICY: &[(&str, &[&str])] = &[
    ("mrr-identity", &[]),
    ("mrr-intent", &["mrr-identity"]),
    ("mrr-relation", &["mrr-identity"]),
    ("mrr-revision", &["mrr-identity"]),
    ("mrr-query", &["mrr-identity", "mrr-relation"]),
    ("mrr-logic", &["mrr-identity", "mrr-query", "mrr-relation"]),
    ("mrr-lineage", &["mrr-identity", "mrr-relation"]),
    ("mrr-transition", &["mrr-identity", "mrr-relation"]),
    (
        "mrr-bundle",
        &[
            "mrr-identity",
            "mrr-logic",
            "mrr-query",
            "mrr-relation",
            "mrr-transition",
        ],
    ),
    (
        "mrr-ascent",
        &[
            "ascent",
            "mrr-bundle",
            "mrr-identity",
            "mrr-logic",
            "mrr-query",
            "mrr-relation",
        ],
    ),
    (
        "mrr-gerbil",
        &[
            "mrr-bundle",
            "mrr-gerbil-native-build",
            "mrr-identity",
            "mrr-logic",
            "mrr-query",
            "mrr-relation",
        ],
    ),
    ("mrr-frontends", &["mrr-gerbil", "mrr-query"]),
    (
        "meta-relational-reasoning",
        &[
            "mrr-ascent",
            "mrr-bundle",
            "mrr-identity",
            "mrr-intent",
            "mrr-lineage",
            "mrr-logic",
            "mrr-query",
            "mrr-relation",
            "mrr-revision",
            "mrr-transition",
        ],
    ),
    (
        "mrr-conformance",
        &["meta-relational-reasoning", "mrr-intent", "mrr-revision"],
    ),
];

pub(super) fn workspace_root_from_manifest() -> std::path::PathBuf {
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

fn repository_files(workspace_root: &Path) -> Vec<std::path::PathBuf> {
    let output = Command::new("git")
        .args([
            "-C",
            ".",
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .current_dir(workspace_root)
        .output()
        .expect("git must list repository-owned files");
    assert!(
        output.status.success(),
        "git ls-files failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let relative = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| {
            std::str::from_utf8(path)
                .expect("repository paths must be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    let ignored = ignored_paths(workspace_root, &relative);
    relative
        .into_iter()
        .filter(|path| !ignored.contains(path))
        .map(|path| workspace_root.join(path))
        .collect()
}

fn ignored_paths(workspace_root: &Path, paths: &[String]) -> BTreeSet<String> {
    let mut child = Command::new("git")
        .args(["-C", ".", "check-ignore", "--no-index", "-z", "--stdin"])
        .current_dir(workspace_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("git ignore engine must start");
    {
        let input = child.stdin.as_mut().expect("git ignore stdin");
        for path in paths {
            input.write_all(path.as_bytes()).expect("write path");
            input.write_all(&[0]).expect("write path delimiter");
        }
    }
    let output = child
        .wait_with_output()
        .expect("git ignore engine must finish");
    assert!(
        matches!(output.status.code(), Some(0 | 1)),
        "git check-ignore failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| {
            std::str::from_utf8(path)
                .expect("ignored repository paths must be valid UTF-8")
                .to_owned()
        })
        .collect()
}

pub(super) fn collect_forbidden_policy_rule_files(
    workspace_root: &Path,
    files: &mut Vec<std::path::PathBuf>,
) {
    for path in repository_files(workspace_root) {
        if path.file_name().and_then(|name| name.to_str()) == Some(FORBIDDEN_POLICY_RULE_FILE) {
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

    let mut registered: Vec<String> = mrr_workspace_member_policies()
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
            entry.file_name()?.to_str().and_then(|name| {
                (name == "meta-relational-reasoning" || name.starts_with("mrr-"))
                    .then(|| name.to_string())
            })
        })
        .collect();
    declared_in_crates_dir.sort_unstable();

    assert_eq!(registered, declared_in_crates_dir);
}

#[test]
fn repository_source_and_fixtures_are_ascii_only() {
    let workspace_root = workspace_root_from_manifest();
    let mut hits = Vec::new();

    for path in repository_files(&workspace_root) {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        if !text.is_ascii() {
            hits.push(path);
        }
    }

    assert!(
        hits.is_empty(),
        "repository source and fixtures must be ASCII-only: {hits:?}"
    );
}

#[test]
fn gitignore_is_the_generated_and_vendor_source_boundary() {
    let workspace_root = workspace_root_from_manifest();
    let candidates = [
        ".ci/gerbil-src/src/std/io.ss",
        ".gerbil/lib/module.ssi",
        ".venv/bin/python",
        "target/debug/example",
        "crates/mrr-frontends/target/criterion/report.svg",
    ]
    .map(str::to_owned);
    let ignored = ignored_paths(&workspace_root, &candidates);
    assert_eq!(ignored, candidates.into_iter().collect());
}

fn assert_mrr_dependency_allowed(
    crate_name: &str,
    dependency: &str,
    section: &str,
    allowed_dependencies: &[&str],
) {
    if dependency == "mrr-asp-rust-project-policy" {
        return;
    }
    let is_architecture_dependency = dependency == "ascent"
        || dependency == "meta-relational-reasoning"
        || dependency.starts_with("mrr-");
    if !is_architecture_dependency {
        return;
    }
    assert!(
        allowed_dependencies.contains(&dependency),
        "MRR crate {crate_name} has forbidden architecture dependency {dependency} in {section}; allowed={allowed_dependencies:?}"
    );
}

#[test]
fn mrr_dependency_direction_is_fail_closed() {
    let workspace_root = workspace_root_from_manifest();

    for (crate_name, allowed_dependencies) in MRR_DEPENDENCY_POLICY {
        let manifest_path = workspace_root
            .join("crates")
            .join(crate_name)
            .join("Cargo.toml");
        let manifest = fs::read_to_string(&manifest_path).unwrap_or_else(|_| {
            panic!(
                "MRR manifest should be readable: {}",
                manifest_path.display()
            )
        });
        let mut section = "";

        for line in manifest.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                section = trimmed;
                let unwrapped = section.trim_start_matches('[').trim_end_matches(']');
                if let Some((_, dependency)) = unwrapped.split_once("dependencies.") {
                    assert_mrr_dependency_allowed(
                        crate_name,
                        dependency.trim_matches('"'),
                        section,
                        allowed_dependencies,
                    );
                }
                continue;
            }
            if !section.ends_with("dependencies]") || trimmed.starts_with('#') {
                continue;
            }

            assert!(
                !(trimmed.contains("package")
                    && (trimmed.contains("\"mrr-")
                        || trimmed.contains("\"meta-relational-reasoning\"")
                        || trimmed.contains("\"ascent\""))),
                "MRR crate {crate_name} must not rename architecture dependencies in {section}: {trimmed}"
            );

            let Some((dependency, _)) = trimmed.split_once('=') else {
                continue;
            };
            let dependency = dependency
                .trim()
                .split_once('.')
                .map_or(dependency.trim(), |(name, _)| name);
            assert_mrr_dependency_allowed(crate_name, dependency, section, allowed_dependencies);
        }
    }
}

#[test]
fn differential_oracles_are_not_workspace_dependencies() {
    let workspace_root = workspace_root_from_manifest();
    for relative in ["Cargo.toml", "Cargo.lock"] {
        let source = fs::read_to_string(workspace_root.join(relative))
            .unwrap_or_else(|_| panic!("workspace dependency source is readable: {relative}"));
        let normalized = source.to_ascii_lowercase();
        for forbidden in ["selenedb", "selene-gql", "grafeo", "frogql"] {
            assert!(
                !normalized.contains(forbidden),
                "differential oracle must not become a workspace dependency: {forbidden} in {relative}"
            );
        }
    }
}

#[test]
fn all_workspace_crate_manifests_enable_workspace_policy_only_as_a_build_dependency() {
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
            if in_section && line.trim() == "mrr-asp-rust-project-policy.workspace = true" {
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
            manifest_text.contains("mrr-asp-rust-project-policy.workspace = true"),
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
            !has_policy_in_section(&manifest_text, "[dev-dependencies]"),
            "policy must not be a dev-dependency in {}",
            entry.display()
        );
    }
}

#[test]
fn shared_build_support_is_the_only_workspace_policy_build_gate() {
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
        if build_rs.exists() {
            let build_rs_text = fs::read_to_string(&build_rs)
                .unwrap_or_else(|_| panic!("build.rs should be readable: {}", build_rs.display()));
            assert!(
                !build_rs_text.contains("mrr_asp_rust_project_policy"),
                "member build script must not become a second policy authority: {}",
                build_rs.display()
            );
        }
    }

    let support_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let shared_build_gate =
        fs::read_to_string(support_root.join("build.rs")).expect("shared build gate readable");
    assert!(shared_build_gate.contains(
        "assert_asp_rust_workspace_policy_from_env as assert_mrr_asp_rust_harness_policy_from_env"
    ));
    assert!(
        shared_build_gate
            .contains("assert_mrr_asp_rust_harness_policy_from_env(&workspace_policy)")
    );
    assert!(!shared_build_gate.contains("AspRustDownstreamPolicy"));
    assert!(!support_root.join("src/build_gate.rs").exists());
}
