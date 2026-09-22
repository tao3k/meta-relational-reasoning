//! Typed command adapter for the live MRR experiment.

use std::{env, process::ExitCode};

fn main() -> ExitCode {
    match mrr_conformance::run_closure_tool_cli(&env::args().skip(1).collect::<Vec<_>>()) {
        Ok(receipt) => {
            println!("{receipt}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("mrr-closure-tool: {error}");
            ExitCode::FAILURE
        }
    }
}
