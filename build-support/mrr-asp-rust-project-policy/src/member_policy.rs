//! Central Rust harness policy registry for MRR workspace member crates.

/// Declarative identity of one crate governed by the shared workspace policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MrrAspRustMemberPolicy {
    pub package_name: &'static str,
    pub crate_root: &'static str,
}

macro_rules! workspace_member_policy {
    ($package_name:expr, $crate_root:expr) => {
        MrrAspRustMemberPolicy {
            package_name: $package_name,
            crate_root: $crate_root,
        }
    };
}

const MRR_WORKSPACE_MEMBER_POLICIES: &[MrrAspRustMemberPolicy] = &[
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
    workspace_member_policy!("mrr-conformance", "crates/mrr-conformance"),
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
pub fn mrr_workspace_member_policies() -> &'static [MrrAspRustMemberPolicy] {
    MRR_WORKSPACE_MEMBER_POLICIES
}

/// Returns the registered MRR member policy for `package_name`.
pub fn mrr_workspace_member_policy_for(
    package_name: &str,
) -> Option<&'static MrrAspRustMemberPolicy> {
    mrr_workspace_member_policies()
        .iter()
        .find(|policy| policy.package_name == package_name)
}
