//! Central Rust harness policy registry for MRR workspace member crates.

/// A source owner covered by a member crate harness policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MrrRustProjectHarnessOwnerPolicy {
    pub path: &'static str,
    pub rationale: &'static str,
}

/// Declarative Rust harness policy for one MRR workspace member crate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MrrRustProjectHarnessMemberPolicy {
    pub package_name: &'static str,
    pub crate_root: &'static str,
    pub cargo_check_advice_allow_explanation: &'static str,
    pub verification_label: Option<&'static str>,
    pub criterion_performance_verification: bool,
    pub latency_sensitive_performance_owners: &'static [MrrRustProjectHarnessOwnerPolicy],
    pub availability_stability_owners: &'static [MrrRustProjectHarnessOwnerPolicy],
}

impl MrrRustProjectHarnessMemberPolicy {
    /// Builds the `rust-lang-project-harness` config for this member crate.
    pub fn to_harness_config(self) -> rust_lang_project_harness::RustHarnessConfig {
        let mut config = rust_lang_project_harness::RustHarnessConfig {
            cargo_check_advice_allow_explanation: Some(
                self.cargo_check_advice_allow_explanation.to_string(),
            ),
            ..Default::default()
        };

        if self.criterion_performance_verification {
            config = config.with_criterion_performance_verification();
        }

        for owner in self.latency_sensitive_performance_owners {
            config = config.with_latency_sensitive_performance_owner(owner.path, owner.rationale);
        }

        for owner in self.availability_stability_owners {
            config = config.with_availability_stability_owner(owner.path, owner.rationale);
        }

        config
    }
}

macro_rules! workspace_member_policy {
    ($package_name:expr, $crate_root:expr) => {
        MrrRustProjectHarnessMemberPolicy {
            package_name: $package_name,
            crate_root: $crate_root,
            cargo_check_advice_allow_explanation: concat!(
                "scope=",
                $package_name,
                " cargo-check; owner=mrr-runtime-guard; finding_category=docs_purity; why_safe_now=Default workspace crate policy is enforced; cleanup_trigger=Remove all warning-eligible contract exceptions in this policy branch only after evidence closure"
            ),
            verification_label: None,
            criterion_performance_verification: true,
            latency_sensitive_performance_owners: MRR_DEFAULT_PERFORMANCE_OWNERS,
            availability_stability_owners: MRR_DEFAULT_STABILITY_OWNERS,
        }
    };
}

const MRR_DEFAULT_PERFORMANCE_OWNERS: &[MrrRustProjectHarnessOwnerPolicy] =
    &[MrrRustProjectHarnessOwnerPolicy {
        path: "src/lib.rs",
        rationale: "public crate facade and API surface requires ongoing performance baseline",
    }];

const MRR_DEFAULT_STABILITY_OWNERS: &[MrrRustProjectHarnessOwnerPolicy] =
    &[MrrRustProjectHarnessOwnerPolicy {
        path: "src/lib.rs",
        rationale: "crate public entrypoints participate in stability guarantees",
    }];

const MRR_WORKSPACE_MEMBER_POLICIES: &[MrrRustProjectHarnessMemberPolicy] = &[
    workspace_member_policy!("gql-core", "crates/gql-core"),
    workspace_member_policy!("gql-ast", "crates/gql-ast"),
    workspace_member_policy!("gql-source", "crates/gql-source"),
    workspace_member_policy!("gql-types", "crates/gql-types"),
    workspace_member_policy!("gql-ir", "crates/gql-ir"),
    workspace_member_policy!("gql-sema", "crates/gql-sema"),
    workspace_member_policy!("gql-syntax", "crates/gql-syntax"),
    workspace_member_policy!("gql-catalog", "crates/gql-catalog"),
    workspace_member_policy!("gql-compiler", "crates/gql-compiler"),
    workspace_member_policy!("gql", "crates/gql"),
    workspace_member_policy!("mrr-identity", "crates/mrr-identity"),
    workspace_member_policy!("mrr-intent", "crates/mrr-intent"),
    workspace_member_policy!("mrr-relation", "crates/mrr-relation"),
    workspace_member_policy!("mrr-revision", "crates/mrr-revision"),
    workspace_member_policy!("mrr-query", "crates/mrr-query"),
    workspace_member_policy!("mrr-frontends", "crates/mrr-frontends"),
    workspace_member_policy!("mrr-logic", "crates/mrr-logic"),
    workspace_member_policy!("mrr-mvp-acceptance", "crates/mrr-mvp-acceptance"),
    workspace_member_policy!("mrr-lineage", "crates/mrr-lineage"),
    workspace_member_policy!("mrr-transition", "crates/mrr-transition"),
    workspace_member_policy!("mrr-bundle", "crates/mrr-bundle"),
    workspace_member_policy!("mrr-ascent", "crates/mrr-ascent"),
    workspace_member_policy!("mrr-gerbil", "crates/mrr-gerbil"),
    workspace_member_policy!(
        "meta-relational-reasoning",
        "crates/meta-relational-reasoning"
    ),
];

/// Returns MRR workspace member policies used by this harness policy crate.
pub fn mrr_workspace_member_policies() -> &'static [MrrRustProjectHarnessMemberPolicy] {
    MRR_WORKSPACE_MEMBER_POLICIES
}

/// Returns the registered MRR member policy for `package_name`.
pub fn mrr_workspace_member_policy_for(
    package_name: &str,
) -> Option<&'static MrrRustProjectHarnessMemberPolicy> {
    mrr_workspace_member_policies()
        .iter()
        .find(|policy| policy.package_name == package_name)
}
