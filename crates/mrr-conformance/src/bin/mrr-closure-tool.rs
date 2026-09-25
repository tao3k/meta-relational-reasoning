// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

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
