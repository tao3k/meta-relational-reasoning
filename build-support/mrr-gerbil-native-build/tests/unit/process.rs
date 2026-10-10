use crate::native_archive::run_with_progress;
use std::process::Command;

#[test]
fn quiet_execution_preserves_failed_child_diagnostics() {
    let error = run_with_progress(
        Command::new("/bin/sh").args(["-c", "printf 'typed-child-refusal' >&2; exit 7"]),
        "native fixture",
        false,
    )
    .unwrap_err();
    assert!(error.contains("native fixture"));
    assert!(error.contains("7"));
    assert!(error.contains("typed-child-refusal"));
}
#[test]
fn streaming_execution_preserves_failed_exit_status() {
    let error = run_with_progress(
        Command::new("/bin/sh").args(["-c", "printf 'streamed-child-refusal\n' >&2; exit 7"]),
        "native fixture",
        true,
    )
    .unwrap_err();
    assert!(error.contains("native fixture"));
    assert!(error.contains("7"));
}
// Run this fixture under the consumer's genuine-output watchdog. The six-second
// child would fail a five-second policy if either command output were buffered.
#[test]
#[ignore = "requires the consumer process supervisor to check live phase events"]
fn streaming_execution_forwards_events_before_child_exit() {
    run_with_progress(Command::new("/bin/sh").args(["-c",
        "printf 'CHILD-PHASE started\n'; sleep 3; printf 'CHILD-PHASE intermediate\n'; sleep 3; printf 'CHILD-PHASE complete\n'"]),
        "native progress fixture", true).unwrap();
}
