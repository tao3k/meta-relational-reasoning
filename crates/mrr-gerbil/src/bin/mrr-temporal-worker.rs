//! Explicit process boundary for the canonical Temporal native owner.
fn main() -> std::process::ExitCode {
    match mrr_gerbil::run_temporal_worker() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("mrr-temporal-worker: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
