//! Target-native archive/link adapter for the canonical AOT artifact built by `build.ss`.

mod program_archive;

pub use program_archive::build_program_archive;

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
