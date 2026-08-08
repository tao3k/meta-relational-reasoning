#![forbid(unsafe_code)]

use std::process::ExitCode;

use gql_rust_project_harness_policy::{
    GqlRustProjectHarnessMemberPolicy, RustProjectHarnessDownstreamPolicy,
    assert_rust_project_harness_downstream_policy_from_env, gql_workspace_member_policies,
};

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        None | Some("verify") => {
            if let Err(err) = run_verify() {
                eprintln!("xtask verify failed: {err}");
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Some("help") | Some("-h") | Some("--help") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("unknown xtask command: {other}");
            print_help();
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    println!("xtask");
    println!("  verify   Run workspace policy and publish-readiness verification");
}

fn run_verify() -> Result<(), String> {
    let policies = gql_workspace_member_policies();
    if policies.is_empty() {
        return Err("no workspace policies discovered".into());
    }

    for policy in policies {
        if std::path::Path::new(policy.crate_root).exists() {
            println!("policy-ok: {}", policy.package_name);
        } else {
            return Err(format!(
                "policy crate_root missing: {} ({})",
                policy.package_name, policy.crate_root
            ));
        }
    }

    if std::env::var("GQL_HARNESS_VERIFY")
        .map(|v| v == "1")
        .unwrap_or(false)
    {
        for policy in policies {
            run_harness_gate(policy)?;
        }
        println!("harness gate complete");
    } else {
        println!("GQL_HARNESS_VERIFY != 1, skipping harness gate assertions");
    }

    Ok(())
}

fn run_harness_gate(policy: &GqlRustProjectHarnessMemberPolicy) -> Result<(), String> {
    let downstream_policy = RustProjectHarnessDownstreamPolicy::new(
        policy.verification_label.unwrap_or(policy.package_name),
        policy.to_harness_config(),
    );
    assert_rust_project_harness_downstream_policy_from_env(&downstream_policy);
    Ok(())
}
