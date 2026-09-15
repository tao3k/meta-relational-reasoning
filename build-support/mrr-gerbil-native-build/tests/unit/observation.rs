use crate::native_archive::{ObservationChannel, observation_channel};
use std::time::Duration;

#[test]
fn successful_observations_never_use_the_cargo_warning_channel() {
    assert_eq!(
        observation_channel("module-c-batch", "complete", None, 0),
        ObservationChannel::Silent
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
