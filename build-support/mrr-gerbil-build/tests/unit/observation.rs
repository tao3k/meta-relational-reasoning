use crate::program_archive::{ObservationChannel, observation_channel};
use std::path::Path;
use std::time::Duration;

#[test]
fn generated_build_inputs_do_not_invalidate_the_completed_archive() {
    use crate::program_archive::retain_cargo_directive;
    let output = Path::new("/target/build/mrr/out");
    assert!(!retain_cargo_directive(
        "cargo:rerun-if-changed=/target/build/mrr/out/gerbil-package/lib/static/input.scm",
        output,
    ));
    for directive in [
        "cargo:rerun-if-changed=/workspace/scheme/input.ss",
        "cargo:rerun-if-changed=/sdk/lib/static/input.scm",
        "cargo:rerun-if-changed=/target/build/mrr/outside/input.scm",
        "cargo:rerun-if-env-changed=GERBIL_HOME",
        "cargo:rustc-link-search=native=/target/build/mrr/out",
    ] {
        assert!(retain_cargo_directive(directive, output), "{directive}");
    }
}

#[test]
fn successful_observations_never_use_the_cargo_warning_channel() {
    assert_eq!(
        observation_channel("module-c-batch", "complete", None, 0),
        ObservationChannel::Trace
    );
    assert_eq!(
        observation_channel("module-c-batch", "complete", None, 1),
        ObservationChannel::Trace
    );
    assert_eq!(
        observation_channel("module-c", "cached", None, 9),
        ObservationChannel::Trace
    );
    assert_eq!(
        observation_channel("module-c", "complete", Some(Duration::from_secs(6)), 1),
        ObservationChannel::Trace
    );
}

#[test]
fn only_failed_observations_use_the_cargo_warning_channel() {
    for verbose_level in [0, 1, 9] {
        assert_eq!(
            observation_channel("module-c", "failed", None, verbose_level),
            ObservationChannel::Warning
        );
    }
}

#[test]
fn quiet_build_retains_phase_boundaries_but_not_per_module_noise() {
    for phase in [
        "package-build",
        "program-stage",
        "program-plan",
        "module-c-batch",
        "gsc-link",
        "native-object-batch",
        "static-archive",
    ] {
        for state in ["started", "complete", "cached"] {
            assert_eq!(
                observation_channel(phase, state, None, 0),
                ObservationChannel::Trace
            );
        }
    }
    assert_eq!(
        observation_channel("module-c", "complete", None, 0),
        ObservationChannel::Silent
    );
}
