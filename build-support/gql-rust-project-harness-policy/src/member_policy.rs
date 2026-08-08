//! Central Rust harness policy registry for GQL workspace member crates.

/// A source owner covered by a member crate harness policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GqlRustProjectHarnessOwnerPolicy {
    pub path: &'static str,
    pub rationale: &'static str,
}

/// One diagnostic severity override for a workspace member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GqlRustProjectHarnessSeverityPolicy {
    pub rule_code: &'static str,
    pub severity: rust_lang_project_harness::RustDiagnosticSeverity,
}

/// Declarative Rust harness policy for one GQL workspace member crate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GqlRustProjectHarnessMemberPolicy {
    pub package_name: &'static str,
    pub crate_root: &'static str,
    pub cargo_check_advice_allow_explanation: &'static str,
    pub verification_label: Option<&'static str>,
    pub rule_severity_overrides: &'static [GqlRustProjectHarnessSeverityPolicy],
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

        for severity_override in self.rule_severity_overrides {
            config =
                config.with_rule_severity(severity_override.rule_code, severity_override.severity);
        }

        config
    }
}

const GQL_CORE_PERFORMANCE_OWNERS: &[GqlRustProjectHarnessOwnerPolicy] = &[
    GqlRustProjectHarnessOwnerPolicy {
        path: "src/lib.rs",
        rationale: "gql-core public API surface is the highest-frequency change path",
    },
    GqlRustProjectHarnessOwnerPolicy {
        path: "src/lib.rs",
        rationale: "parser and graph core helpers gate end-to-end latency and stability",
    },
];

const GQL_SEMA_STABILITY_OWNERS: &[GqlRustProjectHarnessOwnerPolicy] =
    &[GqlRustProjectHarnessOwnerPolicy {
        path: "src/lib.rs",
        rationale: "semantic validation must remain stable under incremental query loads",
    }];

const GQL_DEFAULT_PERFORMANCE_OWNERS: &[GqlRustProjectHarnessOwnerPolicy] =
    &[GqlRustProjectHarnessOwnerPolicy {
        path: "src/lib.rs",
        rationale: "crate public facade and API surface requires ongoing performance baseline",
    }];

const GQL_DEFAULT_STABILITY_OWNERS: &[GqlRustProjectHarnessOwnerPolicy] =
    &[GqlRustProjectHarnessOwnerPolicy {
        path: "src/lib.rs",
        rationale: "crate public entrypoints participate in stability guarantees",
    }];

const GQL_NOISY_RULE_OVERRIDES: &[GqlRustProjectHarnessSeverityPolicy] = &[
    GqlRustProjectHarnessSeverityPolicy {
        rule_code: "RUST-MOD-R004",
        severity: rust_lang_project_harness::RustDiagnosticSeverity::Info,
    },
    GqlRustProjectHarnessSeverityPolicy {
        rule_code: "RUST-MOD-R010",
        severity: rust_lang_project_harness::RustDiagnosticSeverity::Info,
    },
    GqlRustProjectHarnessSeverityPolicy {
        rule_code: "RUST-AGENT-DOCS-MODULE-001",
        severity: rust_lang_project_harness::RustDiagnosticSeverity::Info,
    },
    GqlRustProjectHarnessSeverityPolicy {
        rule_code: "RUST-AGENT-DOCS-PUBLIC-002",
        severity: rust_lang_project_harness::RustDiagnosticSeverity::Info,
    },
    GqlRustProjectHarnessSeverityPolicy {
        rule_code: "RUST-AGENT-PROJECT-003",
        severity: rust_lang_project_harness::RustDiagnosticSeverity::Info,
    },
    GqlRustProjectHarnessSeverityPolicy {
        rule_code: "RUST-AGENT-CFG-PUBLIC-015",
        severity: rust_lang_project_harness::RustDiagnosticSeverity::Info,
    },
];

const GQL_ASSEMBLY_PERFORMANCE_OWNERS: &[GqlRustProjectHarnessOwnerPolicy] =
    &[GqlRustProjectHarnessOwnerPolicy {
        path: "src/lib.rs",
        rationale: "compiler entrypoints are hot for facade build and query shape verification",
    }];

const GQL_WORKSPACE_MEMBER_POLICIES: &[GqlRustProjectHarnessMemberPolicy] = &[
    GqlRustProjectHarnessMemberPolicy {
        package_name: "gql-core",
        crate_root: "crates/gql-core",
        cargo_check_advice_allow_explanation: "scope=gql-core cargo-check; owner=gql-runtime-guard; finding_category=docs_purity; why_safe_now=Migration baseline includes existing warnings that are non-functional; cleanup_trigger=Refactor lib.rs and tests to satisfy RUST-AGENT module/docs rules",
        verification_label: Some("gql core"),
        rule_severity_overrides: GQL_NOISY_RULE_OVERRIDES,
        criterion_performance_verification: true,
        latency_sensitive_performance_owners: GQL_CORE_PERFORMANCE_OWNERS,
        availability_stability_owners: GQL_DEFAULT_STABILITY_OWNERS,
    },
    GqlRustProjectHarnessMemberPolicy {
        package_name: "gql-ast",
        crate_root: "crates/gql-ast",
        cargo_check_advice_allow_explanation: "scope=gql-ast cargo-check; owner=gql-runtime-guard; finding_category=docs_purity; why_safe_now=Migration baseline includes existing warnings that are non-functional; cleanup_trigger=Refactor exported AST entrypoints to satisfy RUST-AGENT module/docs rules",
        verification_label: None,
        rule_severity_overrides: GQL_NOISY_RULE_OVERRIDES,
        criterion_performance_verification: true,
        latency_sensitive_performance_owners: GQL_DEFAULT_PERFORMANCE_OWNERS,
        availability_stability_owners: GQL_DEFAULT_STABILITY_OWNERS,
    },
    GqlRustProjectHarnessMemberPolicy {
        package_name: "gql-source",
        crate_root: "crates/gql-source",
        cargo_check_advice_allow_explanation: "scope=gql-source cargo-check; owner=gql-runtime-guard; finding_category=docs_purity; why_safe_now=Migration baseline includes existing warnings that are non-functional; cleanup_trigger=Refactor sources and tests to satisfy RUST-AGENT module/docs rules",
        verification_label: Some("gql source"),
        rule_severity_overrides: GQL_NOISY_RULE_OVERRIDES,
        criterion_performance_verification: true,
        latency_sensitive_performance_owners: GQL_DEFAULT_PERFORMANCE_OWNERS,
        availability_stability_owners: GQL_CORE_PERFORMANCE_OWNERS,
    },
    GqlRustProjectHarnessMemberPolicy {
        package_name: "gql-types",
        crate_root: "crates/gql-types",
        cargo_check_advice_allow_explanation: "scope=gql-types cargo-check; owner=gql-runtime-guard; finding_category=docs_purity; why_safe_now=Migration baseline includes existing warnings that are non-functional; cleanup_trigger=Refactor public types module to satisfy RUST-AGENT module/docs rules",
        verification_label: None,
        rule_severity_overrides: GQL_NOISY_RULE_OVERRIDES,
        criterion_performance_verification: true,
        latency_sensitive_performance_owners: GQL_DEFAULT_PERFORMANCE_OWNERS,
        availability_stability_owners: GQL_DEFAULT_STABILITY_OWNERS,
    },
    GqlRustProjectHarnessMemberPolicy {
        package_name: "gql-ir",
        crate_root: "crates/gql-ir",
        cargo_check_advice_allow_explanation: "scope=gql-ir cargo-check; owner=gql-runtime-guard; finding_category=docs_purity; why_safe_now=Migration baseline includes existing warnings that are non-functional; cleanup_trigger=Refactor public IR modules to satisfy RUST-AGENT module/docs rules",
        verification_label: Some("gql IR"),
        rule_severity_overrides: GQL_NOISY_RULE_OVERRIDES,
        criterion_performance_verification: true,
        latency_sensitive_performance_owners: GQL_DEFAULT_PERFORMANCE_OWNERS,
        availability_stability_owners: GQL_SEMA_STABILITY_OWNERS,
    },
    GqlRustProjectHarnessMemberPolicy {
        package_name: "gql-sema",
        crate_root: "crates/gql-sema",
        cargo_check_advice_allow_explanation: "scope=gql-sema cargo-check; owner=gql-runtime-guard; finding_category=docs_purity; why_safe_now=Migration baseline includes existing warnings that are non-functional; cleanup_trigger=Refactor public sema modules to satisfy RUST-AGENT module/docs rules",
        verification_label: Some("gql semantic"),
        rule_severity_overrides: GQL_NOISY_RULE_OVERRIDES,
        criterion_performance_verification: true,
        latency_sensitive_performance_owners: GQL_DEFAULT_PERFORMANCE_OWNERS,
        availability_stability_owners: GQL_SEMA_STABILITY_OWNERS,
    },
    GqlRustProjectHarnessMemberPolicy {
        package_name: "gql-syntax",
        crate_root: "crates/gql-syntax",
        cargo_check_advice_allow_explanation: "scope=gql-syntax cargo-check; owner=gql-runtime-guard; finding_category=docs_purity; why_safe_now=Migration baseline includes existing warnings that are non-functional; cleanup_trigger=Refactor parser modules and docs to satisfy RUST-AGENT module/docs rules",
        verification_label: Some("gql syntax"),
        rule_severity_overrides: GQL_NOISY_RULE_OVERRIDES,
        criterion_performance_verification: true,
        latency_sensitive_performance_owners: GQL_DEFAULT_PERFORMANCE_OWNERS,
        availability_stability_owners: GQL_DEFAULT_STABILITY_OWNERS,
    },
    GqlRustProjectHarnessMemberPolicy {
        package_name: "gql-catalog",
        crate_root: "crates/gql-catalog",
        cargo_check_advice_allow_explanation: "scope=gql-catalog cargo-check; owner=gql-runtime-guard; finding_category=docs_purity; why_safe_now=Migration baseline includes existing warnings that are non-functional; cleanup_trigger=Refactor catalog ownership modules to satisfy RUST-AGENT module/docs rules",
        verification_label: Some("gql catalog"),
        rule_severity_overrides: GQL_NOISY_RULE_OVERRIDES,
        criterion_performance_verification: true,
        latency_sensitive_performance_owners: GQL_DEFAULT_PERFORMANCE_OWNERS,
        availability_stability_owners: GQL_DEFAULT_STABILITY_OWNERS,
    },
    GqlRustProjectHarnessMemberPolicy {
        package_name: "gql-compiler",
        crate_root: "crates/gql-compiler",
        cargo_check_advice_allow_explanation: "scope=gql-compiler cargo-check; owner=gql-runtime-guard; finding_category=docs_purity; why_safe_now=Migration baseline includes existing warnings that are non-functional; cleanup_trigger=Refactor public compiler modules to satisfy RUST-AGENT module/docs rules",
        verification_label: Some("gql compiler"),
        rule_severity_overrides: GQL_NOISY_RULE_OVERRIDES,
        criterion_performance_verification: true,
        latency_sensitive_performance_owners: GQL_ASSEMBLY_PERFORMANCE_OWNERS,
        availability_stability_owners: GQL_DEFAULT_STABILITY_OWNERS,
    },
    GqlRustProjectHarnessMemberPolicy {
        package_name: "gql-ascent",
        crate_root: "crates/gql-ascent",
        cargo_check_advice_allow_explanation: "scope=gql-ascent cargo-check; owner=gql-runtime-guard; finding_category=docs_purity; why_safe_now=Migration baseline includes existing warnings that are non-functional; cleanup_trigger=Refactor ascent boundary modules to satisfy RUST-AGENT module/docs rules",
        verification_label: Some("gql ascent"),
        rule_severity_overrides: GQL_NOISY_RULE_OVERRIDES,
        criterion_performance_verification: true,
        latency_sensitive_performance_owners: GQL_DEFAULT_PERFORMANCE_OWNERS,
        availability_stability_owners: GQL_DEFAULT_STABILITY_OWNERS,
    },
    GqlRustProjectHarnessMemberPolicy {
        package_name: "gql",
        crate_root: "crates/gql",
        cargo_check_advice_allow_explanation: "scope=gql cargo-check; owner=gql-runtime-guard; finding_category=docs_purity; why_safe_now=Migration baseline includes existing warnings that are non-functional; cleanup_trigger=Refactor facade entrypoints to satisfy RUST-AGENT module/docs rules",
        verification_label: Some("gql facade"),
        rule_severity_overrides: GQL_NOISY_RULE_OVERRIDES,
        criterion_performance_verification: true,
        latency_sensitive_performance_owners: GQL_DEFAULT_PERFORMANCE_OWNERS,
        availability_stability_owners: GQL_CORE_PERFORMANCE_OWNERS,
    },
];

/// Returns GQL workspace member policies used by this harness policy crate.
pub fn gql_workspace_member_policies() -> &'static [GqlRustProjectHarnessMemberPolicy] {
    GQL_WORKSPACE_MEMBER_POLICIES
}

/// Backward-compatible alias for older callers.
pub fn asp_workspace_member_policies() -> &'static [GqlRustProjectHarnessMemberPolicy] {
    gql_workspace_member_policies()
}

/// Returns the registered GQL member policy for `package_name`.
pub fn gql_workspace_member_policy_for(
    package_name: &str,
) -> Option<&'static GqlRustProjectHarnessMemberPolicy> {
    gql_workspace_member_policies()
        .iter()
        .find(|policy| policy.package_name == package_name)
}

/// Backward-compatible alias for older callers.
pub fn asp_workspace_member_policy_for(
    package_name: &str,
) -> Option<&'static GqlRustProjectHarnessMemberPolicy> {
    gql_workspace_member_policy_for(package_name)
}
