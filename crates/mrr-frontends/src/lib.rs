//! Parser-owned ISO GQL compilation into the shared `MetaQueryIr` contract.
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

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
