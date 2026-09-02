//! Target-native archive/link adapter for the AOT artifact staged by `build.ss`.

mod native_archive;

pub use native_archive::build_native_archive;
