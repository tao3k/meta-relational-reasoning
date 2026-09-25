// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Process boundary for the Scheme AOT outer reasoning scheduler.

use std::{env, process::ExitCode};

fn main() -> ExitCode {
    match mrr_gerbil::run_driver_cli(&env::args().skip(1).collect::<Vec<_>>()) {
        Ok(value) => {
            println!("{value}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("mrr-scheme-driver: {error}");
            ExitCode::FAILURE
        }
    }
}
