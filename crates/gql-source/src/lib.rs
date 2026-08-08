//! Public facade for GQL source text and diagnostic primitives.
//!
//! This module owns the canonical source/diagnostic primitives used by frontend crates
//! and keeps their boundaries explicit for build policy checks.

#![forbid(unsafe_code)]

mod api;

pub use api::{Diagnostic, Severity, SourceText, Span};

