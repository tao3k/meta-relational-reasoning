// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Strict projection of one bounded Org intent vocabulary into MRR bundle selection.
#![forbid(unsafe_code)]
mod api;
pub use api::{
    IntentBindingStatus, IntentBundleBinding, IntentProjectionError, IntentSemanticModel,
};
#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
