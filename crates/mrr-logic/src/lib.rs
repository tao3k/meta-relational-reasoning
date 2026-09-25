// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Safe rules over language-neutral MRR atoms.
#![forbid(unsafe_code)]
mod api;
mod why_not;
pub use api::{Rule, RuleError};
pub use mrr_identity::RuleId;
pub use mrr_query::{Atom, RelationId, Term, Variable};
pub use why_not::{
    GroundAtom, WhyNotError, WhyNotIncomplete, WhyNotLimits, WhyNotReceipt, WhyNotStatus, why_not,
};
#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
