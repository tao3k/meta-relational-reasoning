use mrr_asp_rust_project_policy::mrr_workspace_member_policies;
use std::fs;
use std::path::Path;

const FORBIDDEN_POLICY_RULE_FILE: &str = "rust-project-harness-rules.toml";
pub(super) const LEGACY_ISO_LEDGER_FILE: &str = "iso-gql-feature-ledger.yaml";
pub(super) const ISO_NORMATIVE_SOURCES_FILE: &str = "conformance/iso/normative-sources.yaml";
const ISO_CORE_CRATES: &[&str] = &[
    "gql-ast",
    "gql-catalog",
    "gql-compiler",
    "gql-core",
    "gql-ir",
    "gql-sema",
    "gql-source",
    "gql-syntax",
    "gql-types",
];
const FORBIDDEN_CORE_DEPENDENCIES: &[&str] = &[
    "agent-semantic-protocols",
    "ascent",
    "duckdb",
    "gql-ascent",
    "gql-reasoning",
    "graph-turbo",
    "turso",
    "wendao",
];
const FORBIDDEN_LEGACY_REASONING_SYMBOLS: &[&str] = &[
    "ClosureStatus",
    "DerivationError",
    "DerivationId",
    "DerivationLimits",
    "DerivationRequest",
    "DerivationResult",
    "DerivationWitness",
    "DerivedPredicateDescriptor",
    "DerivedRelationProvider",
    "DerivedTuple",
    "FactId",
    "RelationName",
    "RuleId",
];
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
    (
        "mrr-frontends",
        &["gql-ast", "gql-source", "gql-syntax", "mrr-query"],
    ),
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

fn is_generated_or_vendor_directory(file_name: &str) -> bool {
    matches!(
        file_name,
        ".git" | ".data" | "target" | "node_modules" | ".devenv" | ".gerbil" | ".venv"
    )
}

pub(super) fn collect_forbidden_policy_rule_files(dir: &Path, files: &mut Vec<std::path::PathBuf>) {
    let entries = std::fs::read_dir(dir).expect("workspace source directory exists");
    for entry in entries {
        let entry = entry.expect("workspace source directory entry");
        let file_type = entry
            .file_type()
            .expect("workspace source directory entry file type");
        let path = entry.path();
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        if file_type.is_symlink() {
            continue;
        }

        if file_type.is_file() {
            if file_name == FORBIDDEN_POLICY_RULE_FILE {
                files.push(path);
            }
            continue;
        }

        if is_generated_or_vendor_directory(file_name) {
            continue;
        }

        if file_type.is_dir() {
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
            entry.file_name()?.to_str().map(|name| name.to_string())
        })
        .collect();
    declared_in_crates_dir.sort_unstable();

    assert_eq!(registered, declared_in_crates_dir);
}

#[test]
fn repository_source_and_fixtures_are_ascii_only() {
    let workspace_root = workspace_root_from_manifest();
    let mut pending = vec![workspace_root];
    let mut hits = Vec::new();

    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).expect("workspace source directory exists") {
            let entry = entry.expect("workspace source directory entry");
            let file_type = entry.file_type().expect("workspace entry file type");
            let path = entry.path();
            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");

            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                if !is_generated_or_vendor_directory(file_name) {
                    pending.push(path);
                }
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            if !text.is_ascii() {
                hits.push(path);
            }
        }
    }

    assert!(
        hits.is_empty(),
        "repository source and fixtures must be ASCII-only: {hits:?}"
    );
}

#[test]
fn uv_project_environment_is_generated_vendor_state_not_repository_source() {
    assert!(is_generated_or_vendor_directory(".venv"));
    assert!(is_generated_or_vendor_directory(".gerbil"));
    assert!(!is_generated_or_vendor_directory("proofs"));
    assert!(!is_generated_or_vendor_directory("src"));
}

#[test]
fn iso_core_crates_reject_backend_dependencies_and_feature_activation() {
    let workspace_root = workspace_root_from_manifest();

    for crate_name in ISO_CORE_CRATES {
        let manifest_path = workspace_root
            .join("crates")
            .join(crate_name)
            .join("Cargo.toml");
        let manifest = fs::read_to_string(&manifest_path).unwrap_or_else(|_| {
            panic!(
                "core manifest should be readable: {}",
                manifest_path.display()
            )
        });
        let mut section = "";

        for line in manifest.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                section = trimmed;
                continue;
            }

            let is_dependency_section = section.ends_with("dependencies]");
            let is_feature_section = section == "[features]";
            if !is_dependency_section && !is_feature_section {
                continue;
            }

            let normalized = trimmed.to_ascii_lowercase();
            for forbidden in FORBIDDEN_CORE_DEPENDENCIES {
                assert!(
                    !normalized.contains(forbidden),
                    "ISO core crate {crate_name} references forbidden backend dependency {forbidden} in {section}: {trimmed}"
                );
            }
        }
    }
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
        || dependency.starts_with("gql-")
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
                        || trimmed.contains("\"gql-")
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
fn legacy_gql_reasoning_authorities_are_absent() {
    let workspace_root = workspace_root_from_manifest();
    let catalog_source = workspace_root.join("crates/gql-catalog/src");
    assert!(
        !workspace_root.join("crates/gql-reasoning").exists(),
        "legacy gql-reasoning authority must not exist"
    );
    assert!(
        !workspace_root.join("crates/gql-ascent").exists(),
        "legacy gql-ascent authority must not exist"
    );

    for symbol in FORBIDDEN_LEGACY_REASONING_SYMBOLS {
        for entry in fs::read_dir(&catalog_source).expect("catalog source directory exists") {
            let path = entry.expect("catalog source entry").path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
                continue;
            }
            let source = fs::read_to_string(&path).unwrap_or_else(|_| {
                panic!("catalog source should be readable: {}", path.display())
            });
            assert!(
                !source.contains(symbol),
                "gql-catalog must not revive legacy reasoning contract {symbol}: {}",
                path.display()
            );
        }
    }
}

#[test]
fn gql_frontend_statement_surface_is_evidence_owned_without_fabricated_ast_sentinels() {
    let workspace_root = workspace_root_from_manifest();
    let types = fs::read_to_string(workspace_root.join("crates/gql-ast/src/api/types.rs"))
        .expect("GQL AST types are readable");
    for forbidden in ["DataStatement", "Data(DataStatement)"] {
        assert!(
            !types.contains(forbidden),
            "unimplemented future AST surface must not exist: {forbidden}"
        );
    }

    for (relative, required) in [
        (
            "scheme/grammar/gql-declaration.ss",
            &[
                "CreateGraphStatement",
                "DropGraphStatement",
                "CreateGraphTypeStatement",
                "DropGraphTypeStatement",
                "GraphTypeSource",
                "SessionSetStatement",
            ][..],
        ),
        (
            "crates/gql-catalog/src/api.rs",
            &["GraphType", "graph_types", "with_graph_types"][..],
        ),
        (
            "crates/gql-ast/src/api/data_management_lowering.rs",
            &[
                "CreateGraph",
                "DropGraph",
                "CreateGraphType",
                "DropGraphType",
                "GraphTypeSource",
                "SessionCommand",
            ][..],
        ),
        (
            "crates/gql-sema/src/data_management.rs",
            &[
                "CreateGraph",
                "DropGraph",
                "CreateGraphType",
                "DropGraphType",
                "IrGraphTypeSource",
                "IrSessionCommand",
            ][..],
        ),
        (
            "crates/gql-ir/src/api.rs",
            &[
                "CreateGraph",
                "DropGraph",
                "CreateGraphType",
                "DropGraphType",
                "GraphTypeSource",
                "SessionCommand",
            ][..],
        ),
        (
            "crates/gql/tests/unit/data_management_contract.rs",
            &[
                "CreateGraph",
                "DropGraph",
                "CreateGraphType",
                "DropGraphType",
                "GraphTypeSource",
                "SessionCommand",
            ][..],
        ),
    ] {
        let source = fs::read_to_string(workspace_root.join(relative))
            .unwrap_or_else(|_| panic!("frontend statement owner is readable: {relative}"));
        for symbol in required {
            assert!(
                source.contains(symbol),
                "frontend statement surface {symbol} must have executable owner evidence in {relative}"
            );
        }
    }

    let lowering = fs::read_to_string(workspace_root.join("crates/gql-ast/src/api/lowering.rs"))
        .expect("GQL AST lowering is readable");
    assert!(
        !lowering.contains("text: String::new()"),
        "failed lowering must not fabricate an empty identifier"
    );
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

#[test]
fn gql_ast_hot_path_has_an_asp_rust_scenario_and_real_benchmark() {
    let workspace_root = workspace_root_from_manifest();
    let scenario = fs::read_to_string(
        workspace_root.join("crates/gql-ast/tests/unit/ast_performance_scenario.rs"),
    )
    .expect("GQL AST performance Scenario owner is readable");
    for required in [
        "asp_rust_scenario!",
        "asp_rust_scenario_package!",
        "lossless-cst-to-ast-hot-path-v1",
        "warmup_iterations",
        "measure_iterations",
        "diagnostic_count",
        "fallback_count",
    ] {
        assert!(
            scenario.contains(required),
            "GQL AST performance Scenario must own {required}"
        );
    }

    let contract = fs::read_to_string(
        workspace_root.join("crates/gql-ast/tests/unit/performance_contract.rs"),
    )
    .expect("GQL AST performance contract is readable");
    for required in [
        "measure_asp_rust_scenario",
        "gql_syntax::parse",
        "lower_from_syntax",
        "parse",
        "ast_lowering",
        "total_p95",
    ] {
        assert!(
            contract.contains(required),
            "GQL AST performance contract must execute {required}"
        );
    }

    let benchmark = fs::read_to_string(workspace_root.join("crates/gql-ast/benches/ast_perf.rs"))
        .expect("GQL AST Criterion benchmark is readable");
    for required in [
        "gql_syntax::parse",
        "lower_from_syntax",
        "Throughput::Bytes",
        "black_box",
    ] {
        assert!(
            benchmark.contains(required),
            "GQL AST Criterion benchmark must execute {required}"
        );
    }
    assert!(
        !benchmark.contains("ast_smoke"),
        "arithmetic smoke work is not an AST benchmark"
    );

    for relative in [
        "crates/gql-ast/tests/unit/scenarios/lossless-cst-to-ast-hot-path-v1/scenario.toml",
        "crates/gql-ast/tests/unit/scenarios/lossless-cst-to-ast-hot-path-v1/benchmark.toml",
        "crates/gql-ast/tests/unit/scenarios/lossless-cst-to-ast-hot-path-v1/inputs/query.gql",
        "crates/gql-ast/tests/unit/scenarios/lossless-cst-to-ast-hot-path-v1/inputs/catalog.gql",
        "crates/gql-ast/tests/unit/scenarios/lossless-cst-to-ast-hot-path-v1/inputs/mutation.gql",
    ] {
        assert!(
            workspace_root.join(relative).is_file(),
            "missing GQL AST Scenario evidence: {relative}"
        );
    }

    let scenario_root =
        workspace_root.join("crates/gql-ast/tests/unit/scenarios/lossless-cst-to-ast-hot-path-v1");
    let receipt = asp_rust::validate_rust_scenario_benchmark(&scenario_root)
        .expect("validate the GQL AST Scenario with the ASP Rust contract owner");
    assert_eq!(
        receipt.status,
        asp_rust::RustScenarioBenchmarkStatus::Pass,
        "{receipt:?}"
    );
    assert!(receipt.violations.is_empty(), "{:?}", receipt.violations);
}
