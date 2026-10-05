use super::configure_runtime_diagnostics;
use std::process::Command;
#[test]
fn package_launcher_inherits_real_runtime_diagnostics_and_sdk_mappings() {
    for program in ["gxi", "gxpkg"] {
        let mut command = Command::new(program);
        configure_runtime_diagnostics(&mut command, true, Some("~~=/sdk,~~lib=/sdk/lib".into()));
        assert_eq!(
            command
                .get_envs()
                .find(|(k, _)| *k == "GAMBOPT")
                .unwrap()
                .1
                .unwrap(),
            "~~=/sdk,~~lib=/sdk/lib,1n,2n,d5qQ"
        );
    }
    let mut quiet = Command::new("gxpkg");
    configure_runtime_diagnostics(&mut quiet, false, Some("~~=/sdk".into()));
    assert!(!quiet.get_envs().any(|(k, _)| k == "GAMBOPT"));
}
