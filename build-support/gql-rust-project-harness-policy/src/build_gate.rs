//! Build-time activation helpers for GQL Rust member harness policy.

use crate::RustProjectHarnessDownstreamPolicy;
use crate::assert_rust_project_harness_downstream_policy_from_env;
use crate::member_policy::{
    gql_workspace_member_policy_for,
};
use std::env;

fn ensure_harness_verify_default() {
    // Keep policy verification enabled by default during build script execution.
    // SAFETY: this runs in build-script context before multi-threaded policy work and is intended
    // to set a process-wide policy-default used by harness verification.
    unsafe {
        env::set_var("GQL_HARNESS_VERIFY", "1");
    }
}

/// Applies the registered GQL Rust member policy for `package_name` from `build.rs`.
pub fn assert_gql_rust_project_harness_member_policy_from_env(package_name: &str) {
    ensure_harness_verify_default();
    let _ = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| {
        panic!("CARGO_MANIFEST_DIR is required to derive default policy")
    });

    let (harness_config, verification_label) = gql_workspace_member_policy_for(package_name)
        .map(|member_policy| {
            (
                member_policy.to_harness_config(),
                member_policy.verification_label.unwrap_or(package_name),
            )
        })
        .unwrap_or_else(|| {
            panic!(
                "no GQL Rust project harness member policy registered for {package_name}"
            )
        });
    let downstream_policy =
        RustProjectHarnessDownstreamPolicy::new(verification_label, harness_config);
    assert_rust_project_harness_downstream_policy_from_env(&downstream_policy);
}

/// Compatibility alias for older callers using the ASP-prefixed helper name.
pub fn assert_asp_rust_project_harness_member_policy_from_env(package_name: &str) {
    assert_gql_rust_project_harness_member_policy_from_env(package_name);
}
