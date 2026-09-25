// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Parser-owned GQL and Cypher compilation into the shared `MetaQueryIr` contract.
#![forbid(unsafe_code)]

mod lexical;
mod lowering;
mod parser_owned;
mod projection;
mod result_lowering;

pub use lowering::{
    FrontendError, PARSER_OWNED_COMPILATION_SCHEMA_V1, ParserOwnedCompilation,
    ParserOwnedCompilationReceipt, QueryFrontend,
};
pub use mrr_gerbil::ParserLanguage;

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
