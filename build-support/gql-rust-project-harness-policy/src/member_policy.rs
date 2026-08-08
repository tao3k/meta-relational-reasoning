//! Central Rust harness policy registry for GQL workspace member crates.

/// A source owner covered by a member crate harness policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GqlRustProjectHarnessOwnerPolicy {
    pub path: &'static str,
    pub rationale: &'static str,
}

/// Declarative Rust harness policy for one GQL workspace member crate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GqlRustProjectHarnessMemberPolicy {
    pub package_name: &'static str,
    pub crate_root: &'static str,
    pub cargo_check_advice_allow_explanation: &'static str,
    pub verification_label: Option<&'static str>,
    pub criterion_performance_verification: bool,
    pub latency_sensitive_performance_owners: &'static [GqlRustProjectHarnessOwnerPolicy],
    pub availability_stability_owners: &'static [GqlRustProjectHarnessOwnerPolicy],
}

impl GqlRustProjectHarnessMemberPolicy {
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
        GqlRustProjectHarnessMemberPolicy {
            package_name: $package_name,
            crate_root: $crate_root,
            cargo_check_advice_allow_explanation: concat!(
                "scope=",
                $package_name,
                " cargo-check; owner=gql-runtime-guard; finding_category=docs_purity; why_safe_now=Default workspace crate policy is enforced; cleanup_trigger=Remove all warning-eligible contract exceptions in this policy branch only after evidence closure"
            ),
            verification_label: None,
            criterion_performance_verification: true,
            latency_sensitive_performance_owners: GQL_DEFAULT_PERFORMANCE_OWNERS,
            availability_stability_owners: GQL_DEFAULT_STABILITY_OWNERS,
        }
    };
}

const GQL_DEFAULT_PERFORMANCE_OWNERS: &[GqlRustProjectHarnessOwnerPolicy] =
    &[GqlRustProjectHarnessOwnerPolicy {
        path: "src/lib.rs",
        rationale: "public crate facade and API surface requires ongoing performance baseline",
    }];

const GQL_DEFAULT_STABILITY_OWNERS: &[GqlRustProjectHarnessOwnerPolicy] =
    &[GqlRustProjectHarnessOwnerPolicy {
        path: "src/lib.rs",
        rationale: "crate public entrypoints participate in stability guarantees",
    }];

const GQL_WORKSPACE_MEMBER_POLICIES: &[GqlRustProjectHarnessMemberPolicy] = &[
    workspace_member_policy!("gql-core", "crates/gql-core"),
    workspace_member_policy!("gql-ast", "crates/gql-ast"),
    workspace_member_policy!("gql-source", "crates/gql-source"),
    workspace_member_policy!("gql-types", "crates/gql-types"),
    workspace_member_policy!("gql-ir", "crates/gql-ir"),
    workspace_member_policy!("gql-sema", "crates/gql-sema"),
    workspace_member_policy!("gql-reasoning", "crates/gql-reasoning"),
    workspace_member_policy!("gql-syntax", "crates/gql-syntax"),
    workspace_member_policy!("gql-catalog", "crates/gql-catalog"),
    workspace_member_policy!("gql-compiler", "crates/gql-compiler"),
    workspace_member_policy!("gql-ascent", "crates/gql-ascent"),
    workspace_member_policy!("gql", "crates/gql"),
];

/// Returns GQL workspace member policies used by this harness policy crate.
pub fn gql_workspace_member_policies() -> &'static [GqlRustProjectHarnessMemberPolicy] {
    GQL_WORKSPACE_MEMBER_POLICIES
}

/// Returns the registered GQL member policy for `package_name`.
pub fn gql_workspace_member_policy_for(
    package_name: &str,
) -> Option<&'static GqlRustProjectHarnessMemberPolicy> {
    gql_workspace_member_policies()
        .iter()
        .find(|policy| policy.package_name == package_name)
}
