use mrr_rust_project_harness_policy::mrr_workspace_member_policies;
use std::fs;
use std::path::Path;

const FORBIDDEN_POLICY_RULE_FILE: &str = "rust-project-harness-rules.toml";
const ISO_LEDGER_FILE: &str = "iso-gql-feature-ledger.yaml";
const ISO_NORMATIVE_SOURCES_FILE: &str = "conformance/iso/normative-sources.yaml";
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
        "mrr-mvp-acceptance",
        &["meta-relational-reasoning", "mrr-intent", "mrr-revision"],
    ),
];

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

fn is_generated_or_vendor_directory(file_name: &str) -> bool {
    matches!(
        file_name,
        ".git" | ".data" | "target" | "node_modules" | ".devenv" | ".gerbil" | ".venv"
    )
}

fn collect_forbidden_policy_rule_files(dir: &Path, files: &mut Vec<std::path::PathBuf>) {
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
fn central_policy_uses_uniform_workspace_default() {
    let policies = mrr_workspace_member_policies();
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
            !policy
                .cargo_check_advice_allow_explanation
                .contains("per-crate exception"),
            "package {} should not keep per-crate exception text",
            policy.package_name
        );
        assert!(
            policy
                .cargo_check_advice_allow_explanation
                .contains("workspace crate policy is enforced"),
            "policy rationale should be workspace-wide for {}",
            policy.package_name
        );
        assert!(
            policy
                .cargo_check_advice_allow_explanation
                .contains("cleanup_trigger="),
            "policy should include cleanup trigger for {}",
            policy.package_name
        );
        assert!(
            policy.cargo_check_advice_allow_explanation.is_ascii(),
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

#[derive(Default)]
struct IsoLedgerFeature {
    id: String,
    priority: Option<u16>,
    dependencies: Option<Vec<String>>,
    clause_evidence_status: Option<String>,
    syntax_status: Option<String>,
    ast_status: Option<String>,
    sema_status: Option<String>,
    ir_status: Option<String>,
    catalog_status: Option<String>,
    has_positive_fixtures: bool,
    has_negative_fixtures: bool,
    positive_fixture_count: usize,
    negative_fixture_count: usize,
    has_validation: bool,
}

#[derive(Default)]
struct IsoLedgerCapability {
    id: String,
    status: Option<String>,
    has_positives: bool,
    has_negatives: bool,
    positive_count: usize,
    negative_count: usize,
    has_validation: bool,
}

fn ledger_scalar(line: &str, key: &str) -> Option<String> {
    line.trim()
        .strip_prefix(key)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.trim_matches('"').to_owned())
}

fn ledger_inline_values(line: &str, key: &str) -> Option<Vec<String>> {
    let value = line.trim().strip_prefix(key)?.trim();
    let values = value.strip_prefix('[')?.strip_suffix(']')?;
    Some(
        values
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect(),
    )
}

fn finish_ledger_feature(
    feature: &mut Option<IsoLedgerFeature>,
    features: &mut Vec<IsoLedgerFeature>,
) {
    if let Some(feature) = feature.take() {
        features.push(feature);
    }
}

fn finish_ledger_capability(
    capability: &mut Option<IsoLedgerCapability>,
    capabilities: &mut Vec<IsoLedgerCapability>,
) {
    if let Some(capability) = capability.take() {
        capabilities.push(capability);
    }
}

#[test]
fn iso_feature_ledger_has_unique_complete_machine_checkable_entries() {
    let workspace_root = workspace_root_from_manifest();
    let ledger_path = workspace_root.join(ISO_LEDGER_FILE);
    let ledger = fs::read_to_string(&ledger_path).expect("ISO feature ledger is readable");
    let normative_sources = fs::read_to_string(workspace_root.join(ISO_NORMATIVE_SOURCES_FILE))
        .expect("ISO normative source registry is readable");
    let verified_feature_mappings =
        normative_sources::assert_normative_source_registry(&normative_sources);
    assert!(
        ledger
            .lines()
            .any(|line| line.trim() == "schema_version: 5"),
        "ISO feature ledger schema version must be explicit"
    );

    let valid_statuses = [
        "implemented",
        "partial",
        "not_implemented",
        "not_applicable",
    ];
    let mut in_registry = false;
    let mut fixture_section = None;
    let mut current = None;
    let mut features = Vec::new();

    for line in ledger.lines() {
        if line == "feature_registry:" {
            in_registry = true;
            continue;
        }
        if line == "quality_capabilities:" {
            finish_ledger_feature(&mut current, &mut features);
            break;
        }
        if !in_registry {
            continue;
        }
        if let Some(id) = line.strip_prefix("  - feature_id: ") {
            finish_ledger_feature(&mut current, &mut features);
            current = Some(IsoLedgerFeature {
                id: id.trim().to_owned(),
                ..IsoLedgerFeature::default()
            });
            fixture_section = None;
            continue;
        }
        let Some(feature) = current.as_mut() else {
            continue;
        };

        if let Some(value) = ledger_scalar(line, "priority:") {
            feature.priority = Some(value.parse().expect("feature priority is an integer"));
        }
        if let Some(values) = ledger_inline_values(line, "dependencies:") {
            feature.dependencies = Some(values);
        }
        if let Some(value) = ledger_scalar(line, "clause_evidence_status:") {
            feature.clause_evidence_status = Some(value);
        }

        for (key, target) in [
            ("syntax_status:", &mut feature.syntax_status),
            ("ast_status:", &mut feature.ast_status),
            ("sema_status:", &mut feature.sema_status),
            ("ir_status:", &mut feature.ir_status),
            ("catalog_status:", &mut feature.catalog_status),
        ] {
            if let Some(value) = ledger_scalar(line, key) {
                *target = Some(value);
            }
        }
        if line.trim().starts_with("positive_fixtures:") {
            feature.has_positive_fixtures = true;
            fixture_section = (line.trim() == "positive_fixtures:").then_some(true);
        } else if line.trim().starts_with("negative_fixtures:") {
            feature.has_negative_fixtures = true;
            fixture_section = (line.trim() == "negative_fixtures:").then_some(false);
        } else if line.starts_with("    validation:") {
            feature.has_validation = ledger_scalar(line, "validation:").is_some();
            fixture_section = None;
        } else if line.starts_with("      - ") {
            match fixture_section {
                Some(true) => feature.positive_fixture_count += 1,
                Some(false) => feature.negative_fixture_count += 1,
                None => {}
            }
        } else if line.starts_with("    ") && !line.starts_with("      ") {
            fixture_section = None;
        }
    }
    finish_ledger_feature(&mut current, &mut features);

    assert!(
        !features.is_empty(),
        "ISO feature registry must not be empty"
    );
    let mut feature_ids = std::collections::BTreeSet::new();
    let mut priorities = std::collections::BTreeSet::new();
    for feature in &features {
        assert!(
            feature_ids.insert(feature.id.as_str()),
            "duplicate ISO feature id: {}",
            feature.id
        );
        let priority = feature
            .priority
            .unwrap_or_else(|| panic!("{} is missing priority", feature.id));
        assert!(
            priorities.insert(priority),
            "duplicate ISO feature priority: {priority}"
        );
        assert!(
            feature.dependencies.is_some(),
            "{} is missing dependencies",
            feature.id
        );
        assert!(
            matches!(
                feature.clause_evidence_status.as_deref(),
                Some("pending_licensed_clause" | "verified_licensed_clause")
            ),
            "{} has invalid clause_evidence_status",
            feature.id
        );
        for (layer, status) in [
            ("syntax", &feature.syntax_status),
            ("ast", &feature.ast_status),
            ("sema", &feature.sema_status),
            ("ir", &feature.ir_status),
            ("catalog", &feature.catalog_status),
        ] {
            let status = status
                .as_deref()
                .unwrap_or_else(|| panic!("{} is missing {layer}_status", feature.id));
            assert!(
                valid_statuses.contains(&status),
                "{} has invalid {layer}_status: {status}",
                feature.id
            );
        }
        assert!(
            feature.has_positive_fixtures,
            "{} has no positive_fixtures field",
            feature.id
        );
        assert!(
            feature.has_negative_fixtures,
            "{} has no negative_fixtures field",
            feature.id
        );
        assert!(
            feature.positive_fixture_count > 0,
            "{} has no positive fixture",
            feature.id
        );
        let statuses = [
            feature.syntax_status.as_deref(),
            feature.ast_status.as_deref(),
            feature.sema_status.as_deref(),
            feature.ir_status.as_deref(),
            feature.catalog_status.as_deref(),
        ];
        if statuses.contains(&Some("implemented")) {
            assert_eq!(
                feature.clause_evidence_status.as_deref(),
                Some("verified_licensed_clause"),
                "{} marks an ISO layer implemented without verified licensed clause evidence",
                feature.id
            );
            assert!(
                verified_feature_mappings.contains(feature.id.as_str()),
                "{} marks an ISO layer implemented without a feature-specific verified normative mapping",
                feature.id
            );
        }
        let fully_implemented = statuses
            .iter()
            .all(|status| matches!(status, Some("implemented" | "not_applicable")));
        if fully_implemented {
            assert!(
                feature.negative_fixture_count > 0,
                "implemented feature {} has no negative fixture",
                feature.id
            );
        }
        assert!(
            feature.has_validation,
            "{} has no validation evidence",
            feature.id
        );
    }
    let feature_priorities = features
        .iter()
        .map(|feature| {
            (
                feature.id.as_str(),
                feature.priority.expect("priority was validated"),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    for feature in &features {
        let priority = feature.priority.expect("priority was validated");
        let mut dependencies = std::collections::BTreeSet::new();
        for dependency in feature
            .dependencies
            .as_ref()
            .expect("dependencies were validated")
        {
            assert!(
                dependencies.insert(dependency.as_str()),
                "{} repeats dependency {dependency}",
                feature.id
            );
            let dependency_priority =
                feature_priorities
                    .get(dependency.as_str())
                    .unwrap_or_else(|| {
                        panic!("{} references unknown dependency {dependency}", feature.id)
                    });
            assert!(
                dependency_priority < &priority,
                "{} dependency {dependency} must have an earlier priority",
                feature.id
            );
        }
    }

    let mut module_ids = std::collections::BTreeSet::new();
    let mut module_feature_ids = std::collections::BTreeSet::new();
    let mut in_modules = false;
    for line in ledger.lines() {
        if line == "modules:" {
            in_modules = true;
            continue;
        }
        if line == "profiles:" {
            break;
        }
        if in_modules && let Some(id) = line.strip_prefix("  - module_id: ") {
            let id = id.trim();
            assert!(module_ids.insert(id), "duplicate ISO module id: {id}");
        } else if in_modules && line.starts_with("      - gql-") {
            let id = line.trim_start_matches("      - ").trim();
            assert!(
                module_feature_ids.insert(id),
                "ISO feature belongs to more than one module: {id}"
            );
        }
    }
    assert_eq!(
        feature_ids, module_feature_ids,
        "module feature inventory and feature registry must match exactly"
    );

    let mut profiles = std::collections::BTreeMap::<
        &str,
        (
            std::collections::BTreeSet<&str>,
            std::collections::BTreeSet<&str>,
        ),
    >::new();
    let mut in_profiles = false;
    let mut current_profile = None;
    let mut module_list = None;
    for line in ledger.lines() {
        if line == "profiles:" {
            in_profiles = true;
            continue;
        }
        if in_profiles && !line.is_empty() && !line.starts_with(' ') {
            break;
        }
        if !in_profiles {
            continue;
        }
        if let Some(id) = line.strip_prefix("  - profile_id: ") {
            let id = id.trim();
            assert!(
                profiles
                    .insert(
                        id,
                        (
                            std::collections::BTreeSet::new(),
                            std::collections::BTreeSet::new(),
                        ),
                    )
                    .is_none(),
                "duplicate ISO profile id: {id}"
            );
            current_profile = Some(id);
            module_list = None;
        } else if line.trim() == "included_modules:" {
            module_list = Some(true);
        } else if line.trim() == "deferred_modules:" {
            module_list = Some(false);
        } else if let (Some(profile_id), Some(is_included), Some(module_id)) = (
            current_profile,
            module_list,
            line.strip_prefix("      - ").map(str::trim),
        ) {
            assert!(
                module_ids.contains(module_id),
                "profile {profile_id} references unknown module: {module_id}"
            );
            let (included, deferred) = profiles
                .get_mut(profile_id)
                .expect("current profile is registered");
            let inserted = if is_included {
                assert!(
                    !deferred.contains(module_id),
                    "profile {profile_id} both includes and defers module {module_id}"
                );
                included.insert(module_id)
            } else {
                assert!(
                    !included.contains(module_id),
                    "profile {profile_id} both includes and defers module {module_id}"
                );
                deferred.insert(module_id)
            };
            assert!(inserted, "profile {profile_id} repeats module {module_id}");
        } else if line.starts_with("    ") && !line.starts_with("      ") {
            module_list = None;
        }
    }
    assert!(
        !profiles.is_empty(),
        "ISO ledger must declare at least one profile"
    );

    let mut capabilities = Vec::new();
    let mut current_capability = None;
    let mut capability_list = None;
    let mut in_capabilities = false;
    for line in ledger.lines() {
        if line == "quality_capabilities:" {
            in_capabilities = true;
            continue;
        }
        if !in_capabilities {
            continue;
        }
        if let Some(id) = line.strip_prefix("  - capability_id: ") {
            finish_ledger_capability(&mut current_capability, &mut capabilities);
            current_capability = Some(IsoLedgerCapability {
                id: id.trim().to_owned(),
                ..IsoLedgerCapability::default()
            });
            capability_list = None;
            continue;
        }
        let Some(capability) = current_capability.as_mut() else {
            continue;
        };
        if let Some(status) = ledger_scalar(line, "status:") {
            capability.status = Some(status);
        } else if line.trim().starts_with("positives:") {
            capability.has_positives = true;
            capability_list = (line.trim() == "positives:").then_some(true);
        } else if line.trim().starts_with("negatives:") {
            capability.has_negatives = true;
            capability_list = (line.trim() == "negatives:").then_some(false);
        } else if line.starts_with("    validation:") {
            capability.has_validation = ledger_scalar(line, "validation:").is_some();
            capability_list = None;
        } else if line.starts_with("      - ") {
            match capability_list {
                Some(true) => capability.positive_count += 1,
                Some(false) => capability.negative_count += 1,
                None => {}
            }
        } else if line.starts_with("    ") && !line.starts_with("      ") {
            capability_list = None;
        }
    }
    finish_ledger_capability(&mut current_capability, &mut capabilities);

    let mut capability_ids = std::collections::BTreeSet::new();
    for capability in &capabilities {
        assert!(
            capability_ids.insert(capability.id.as_str()),
            "duplicate quality capability id: {}",
            capability.id
        );
        assert_eq!(
            capability.status.as_deref(),
            Some("implemented"),
            "quality capability {} must use implemented status",
            capability.id
        );
        assert!(
            capability.has_positives,
            "{} has no positives field",
            capability.id
        );
        assert!(
            capability.has_negatives,
            "{} has no negatives field",
            capability.id
        );
        assert!(
            capability.positive_count > 0,
            "{} has no positive evidence",
            capability.id
        );
        assert!(
            capability.has_validation,
            "{} has no validation evidence",
            capability.id
        );
    }
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
    if dependency == "mrr-rust-project-harness-policy" {
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
            if in_section && line.trim() == "mrr-rust-project-harness-policy.workspace = true" {
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
            manifest_text.contains("mrr-rust-project-harness-policy.workspace = true"),
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
            manifest_path
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .display()
        );
        let build_rs_text = fs::read_to_string(&build_rs).unwrap_or_else(|_| {
            panic!(
                "build.rs should exist in {}",
                manifest_path.parent().unwrap().display()
            )
        });
        assert!(
            build_rs_text.contains("assert_mrr_rust_project_harness_member_policy_from_env"),
            "missing policy gate call in {}",
            build_rs.display()
        );
    }
}
mod normative_sources;
mod override_rules;
