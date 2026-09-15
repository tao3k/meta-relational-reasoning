//! GQL and Cypher frontend adapters into the shared `MetaQueryIr` contract.
#![forbid(unsafe_code)]

mod lowering;
mod parser_owned;
mod value_type_identity;

pub use lowering::{
    FrontendError, PARSER_OWNED_COMPILATION_SCHEMA_V1, ParserOwnedCompilation,
    ParserOwnedCompilationReceipt, QueryFrontend, QueryLanguage,
};
pub use parser_owned::lower_parser_owned_query;

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
