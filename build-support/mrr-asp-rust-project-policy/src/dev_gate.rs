// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Shared MRR configuration for ASP Rust test-only policy gates.

use asp_rust::{AspRustConfig, AspRustWorkspacePolicy, default_asp_rust_config};

const ADVICE_ALLOW_EXPLANATION: &str = "scope=meta-relational-reasoning workspace; owner=mrr-asp-rust-build-support; finding_category=docs_purity; why_safe_now=MRR workspace policy remains mandatory and advisory findings stay visible in receipts; cleanup_trigger=remove this allowance when every advisory owner is closed";

/// Return the shared configuration used by package-scoped Dev Gates.
pub fn mrr_member_policy_config() -> AspRustConfig {
    default_asp_rust_config().with_cargo_test_advice_allow_explanation(ADVICE_ALLOW_EXPLANATION)
}

/// Return the single workspace policy used by Build Support's workspace gate.
pub fn mrr_workspace_policy() -> AspRustWorkspacePolicy {
    AspRustWorkspacePolicy::new("meta-relational-reasoning", mrr_member_policy_config())
}

/// Mount the shared MRR package policy in a member's Cargo test target.
#[macro_export]
macro_rules! mrr_asp_rust_member_dev_gate {
    () => {
        $crate::asp_rust::asp_rust_cargo_test_gate!(
            advice = allow,
            config = $crate::mrr_member_policy_config()
        );
    };
}

/// Mount the shared MRR workspace policy once from Build Support tests.
#[macro_export]
macro_rules! mrr_asp_rust_workspace_dev_gate {
    () => {
        $crate::asp_rust::asp_rust_workspace_dev_gate!(
            mode = deny,
            policy = $crate::mrr_workspace_policy()
        );
    };
}
