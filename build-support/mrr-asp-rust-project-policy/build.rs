use asp_rust::assert_asp_rust_workspace_policy_from_env as assert_mrr_asp_rust_harness_policy_from_env;

fn main() {
    let mut config = asp_rust::default_asp_rust_config();
    config.cargo_check_advice_allow_explanation = Some(
        "scope=gql-rust workspace; owner=mrr-asp-rust-project-policy; finding_category=docs_purity; why_safe_now=parser-native workspace policy remains mandatory and advisory findings stay visible in receipts; cleanup_trigger=remove this allowance when every advisory owner is closed"
            .to_string(),
    );
    let workspace_policy = asp_rust::AspRustWorkspacePolicy::new("gql-rust", config);
    assert_mrr_asp_rust_harness_policy_from_env(&workspace_policy);
}
