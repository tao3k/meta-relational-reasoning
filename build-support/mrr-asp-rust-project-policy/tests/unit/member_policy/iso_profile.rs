use std::fs;

use super::contracts::{
    ISO_NORMATIVE_SOURCES_FILE, LEGACY_ISO_LEDGER_FILE, workspace_root_from_manifest,
};
use super::normative_sources;

const ISO_PROFILE_SCHEME_FILE: &str = "scheme/grammar/gql-profile.ss";
const ISO_PROFILE_ORG_FILE: &str = "docs/architecture/0024-iso-gql-language-profile.org";

#[test]
fn iso_profile_is_gerbil_owned_and_has_no_yaml_ledger() {
    let workspace_root = workspace_root_from_manifest();
    assert!(
        !workspace_root.join(LEGACY_ISO_LEDGER_FILE).exists(),
        "the legacy YAML feature ledger must not remain a second ISO profile authority"
    );

    let scheme = fs::read_to_string(workspace_root.join(ISO_PROFILE_SCHEME_FILE))
        .expect("Gerbil ISO profile declaration is readable");
    assert!(scheme.contains("(defsyntax (with-mrr-iso-gql-profile"));
    assert!(scheme.contains("(schema mrr.iso-gql-profile.v1)"));
    assert!(scheme.contains("gql-query-language-frontend-v1"));
    assert!(scheme.contains("gql-iso-language-frontend-v1"));

    let org = fs::read_to_string(workspace_root.join(ISO_PROFILE_ORG_FILE))
        .expect("Org ISO profile audit is readable");
    assert!(org.contains("#+name: rfc-0024-profile-admission"));
    assert!(org.contains("#+begin_src mermaid"));
    assert!(org.contains("#+name: rfc-0024-profile-mathematics"));
    assert!(org.contains("#+begin_src typst"));
}

#[test]
fn iso_corrigendum_and_full_language_target_profile_are_frozen() {
    let workspace_root = workspace_root_from_manifest();
    let profile = fs::read_to_string(workspace_root.join(ISO_PROFILE_SCHEME_FILE))
        .expect("Gerbil ISO profile declaration is readable");
    let normative_sources = fs::read_to_string(workspace_root.join(ISO_NORMATIVE_SOURCES_FILE))
        .expect("ISO normative source registry is readable");
    let verified_mappings = normative_sources::assert_normative_source_registry(&normative_sources);
    assert!(
        verified_mappings.is_empty(),
        "no ISO feature may be verified before licensed clause evidence is present"
    );

    let corrigendum = normative_sources
        .split("\n  - release_id: iso-39075-2024-cor-1")
        .nth(1)
        .and_then(|entry| entry.split("\n  - release_id: ").next())
        .expect("ISO/IEC 39075:2024/Cor 1:2026 source entry exists");
    assert!(
        corrigendum
            .lines()
            .any(|line| line.trim() == "publication_status: published"),
        "Cor 1 public metadata must record the published lifecycle state"
    );
    assert!(
        corrigendum
            .lines()
            .any(|line| line.trim() == "publication_stage: \"60.60\""),
        "Cor 1 public metadata must record ISO stage 60.60"
    );
    assert!(
        corrigendum
            .lines()
            .any(|line| line.trim() == "status: unavailable"),
        "published metadata must not invent a licensed corrigendum artifact"
    );

    assert!(profile.contains("gql-query-language-frontend-v1"));
    assert!(profile.contains("gql-iso-language-frontend-v1"));
    assert!(profile.contains("(gql-iso-language-frontend-v1 iso-39075-2024-cor-1)"));
    for module in [
        "language-foundation",
        "graph-model",
        "path-patterns",
        "query-core",
        "query-advanced",
        "data-management",
    ] {
        assert!(
            profile.contains(&format!("(gql-iso-language-frontend-v1 included {module})")),
            "target profile omits ISO module {module}"
        );
    }
    assert!(
        !profile.contains("(gql-iso-language-frontend-v1 deferred "),
        "the complete ISO language frontend target cannot defer a module"
    );
    assert!(
        profile.contains("independent-full-iso-language-frontend-target"),
        "target profile must be a language-frontend target, not an ISO certification claim"
    );
}
