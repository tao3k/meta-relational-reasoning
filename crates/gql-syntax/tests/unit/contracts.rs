//! Executable contracts for the lossless syntax frontend.

#[path = "parser_lib.rs"]
mod parser_lib;

#[path = "parser_fixtures.rs"]
mod parser_fixtures;

#[path = "rowan_kind_contract.rs"]
mod rowan_kind_contract;

#[path = "gerbil_grammar_contract.rs"]
mod gerbil_grammar_contract;

#[path = "character_string_contract.rs"]
mod character_string_contract;
