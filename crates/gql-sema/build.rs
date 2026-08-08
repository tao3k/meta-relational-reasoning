fn main() {
    gql_rust_project_harness_policy::assert_gql_rust_project_harness_member_policy_from_env(env!(
        "CARGO_PKG_NAME"
    ));
}
