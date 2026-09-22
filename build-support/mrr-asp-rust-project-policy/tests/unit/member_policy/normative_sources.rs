use std::collections::BTreeSet;

pub(super) fn assert_normative_source_registry(normative_sources: &str) -> BTreeSet<String> {
    assert_required_contract(normative_sources);
    let entries = normative_sources
        .split("\n  - release_id: ")
        .skip(1)
        .collect::<Vec<_>>();
    assert_eq!(
        entries.len(),
        3,
        "ISO normative source registry must track baseline, correction, and future delta"
    );
    for entry in &entries {
        assert_source_entry(entry);
    }
    collect_verified_feature_mappings(normative_sources)
}

fn assert_required_contract(normative_sources: &str) {
    for required in [
        "schema_version: 1",
        "baseline_release: iso-39075-2024",
        "licensed_clause_required_for_implemented_status: true",
        "public_compatibility_evidence_may_expand_partial_features: true",
        "licensed_clause_required_for_iso_conformance_claim: true",
        "public_product_metadata_is_clause_evidence: false",
        "differential_oracles_are_clause_evidence: false",
        "future_drafts_are_default_language_evidence: false",
        "role: normative_baseline",
        "role: versioned_correction",
        "role: non_normative_future_delta",
        "default_language_enabled: false",
    ] {
        assert!(
            normative_sources
                .lines()
                .any(|line| line.trim() == required),
            "ISO normative source registry is missing required contract: {required}"
        );
    }
    assert!(
        !normative_sources.contains(".data/reference-sources"),
        "ISO normative evidence must never resolve through oracle snapshots"
    );
}

fn assert_source_entry(entry: &str) {
    let release_id = entry
        .lines()
        .next()
        .expect("normative source has a release id")
        .trim();
    let status = required_scalar(entry, release_id, "status:");
    let repository_path = required_scalar(entry, release_id, "repository_path:");
    let digest = required_scalar(entry, release_id, "digest:");
    assert!(
        matches!(
            status.as_str(),
            "unavailable" | "verified" | "prohibited_for_default_language"
        ),
        "normative source {release_id} has invalid artifact status: {status}"
    );
    if status == "verified" {
        assert_ne!(repository_path, "null", "verified artifact requires a path");
        assert!(
            repository_path.starts_with("conformance/iso/licensed/"),
            "verified artifact must remain under the licensed ISO evidence boundary"
        );
        assert!(
            digest.strip_prefix("sha256:").is_some_and(|value| {
                value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
            }),
            "verified artifact requires an exact SHA-256 digest"
        );
    } else {
        assert_eq!(
            repository_path, "null",
            "non-verified artifact must not claim a path"
        );
        assert_eq!(
            digest, "null",
            "non-verified artifact must not claim a digest"
        );
        assert!(
            entry
                .lines()
                .any(|line| line.trim() == "feature_mappings: []"),
            "non-verified artifact must not map ISO features"
        );
    }
}

fn required_scalar(entry: &str, release_id: &str, key: &str) -> String {
    entry
        .lines()
        .find_map(|line| yaml_scalar(line, key))
        .unwrap_or_else(|| panic!("normative source {release_id} has no {key}"))
}

fn yaml_scalar(line: &str, key: &str) -> Option<String> {
    line.trim()
        .strip_prefix(key)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.trim_matches('"').to_owned())
}

fn collect_verified_feature_mappings(normative_sources: &str) -> BTreeSet<String> {
    let mut mappings = BTreeSet::new();
    let mut artifact_is_verified = false;
    let mut in_mappings = false;
    for line in normative_sources.lines() {
        if line.starts_with("  - release_id: ") {
            artifact_is_verified = false;
            in_mappings = false;
        } else if line.trim() == "status: verified" {
            artifact_is_verified = true;
        } else if line.trim() == "feature_mappings:" {
            in_mappings = true;
        } else if in_mappings && line.starts_with("      - gql-") {
            assert!(
                artifact_is_verified,
                "feature mapping requires a verified artifact"
            );
            let feature_id = line.trim_start_matches("      - ").trim();
            assert!(
                mappings.insert(feature_id.to_owned()),
                "ISO feature is mapped by more than one normative artifact: {feature_id}"
            );
        } else if in_mappings && line.starts_with("    ") && !line.starts_with("      ") {
            in_mappings = false;
        }
    }
    mappings
}
