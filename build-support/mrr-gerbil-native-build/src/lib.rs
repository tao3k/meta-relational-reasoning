// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Target-native archive/link adapter for the canonical AOT artifact built by `build.ss`.

mod native_archive;

pub use native_archive::build_native_archive;

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
