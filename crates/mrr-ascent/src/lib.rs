#![forbid(unsafe_code)]
// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Bounded Ascent evaluation for already validated MRR bundles.

mod api;

pub use api::{
    ClosureConfig, ClosureError, ClosureLimits, ClosureReceipt, ClosureStatus, DerivationCandidate,
    DerivationReceiptDigest, evaluate_transitive_closure,
};
