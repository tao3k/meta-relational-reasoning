// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

//! External-consumer proof that domain variation changes bundles, not the kernel.
#![forbid(unsafe_code)]

mod tool;
mod tool_cli;

pub use tool::{ClosureToolError, ClosureToolInput, ClosureToolReceipt, run_closure_tool};
pub use tool_cli::run_closure_tool_cli;

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
