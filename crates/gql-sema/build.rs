fn main() {
    mrr_rust_project_harness_policy::assert_mrr_rust_project_harness_member_policy_from_env(env!(
        "CARGO_PKG_NAME"
    ));
}
