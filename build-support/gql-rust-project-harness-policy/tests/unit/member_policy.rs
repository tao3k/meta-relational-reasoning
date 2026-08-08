use gql_rust_project_harness_policy::gql_workspace_member_policies;

#[test]
fn central_policy_registry_contains_migrated_member_crates() {
    let package_names: Vec<_> = gql_workspace_member_policies()
        .iter()
        .map(|policy| policy.package_name)
        .collect();

    assert_eq!(
        package_names,
        vec![
            "gql-core",
            "gql-ast",
            "gql-source",
            "gql-types",
            "gql-ir",
            "gql-sema",
            "gql-syntax",
            "gql-catalog",
            "gql-compiler",
            "gql-ascent",
            "gql",
        ]
    );
}

#[test]
fn central_policy_preserves_member_specific_verification_owners() {
    let policies = gql_workspace_member_policies();
    let client_db = policies
        .iter()
        .find(|policy| policy.package_name == "gql-ascent")
        .expect("gql-ascent policy");
    let client = policies
        .iter()
        .find(|policy| policy.package_name == "gql")
        .expect("gql facade policy");

    assert_eq!(client_db.verification_label, Some("gql ascent"));
    assert_eq!(client_db.latency_sensitive_performance_owners.len(), 1);
    assert_eq!(client.availability_stability_owners.len(), 2);
    assert_eq!(client.verification_label, Some("gql facade"));
}
