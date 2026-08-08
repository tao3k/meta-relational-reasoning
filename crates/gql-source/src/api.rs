//! Owned API types for source locations and diagnostics.

use std::sync::Arc;

/// Source span in bytes.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Span {
    /// Inclusive start offset.
    pub start: u32,
    /// Exclusive end offset.
    pub end: u32,
}

impl Span {
    /// Creates a new span from byte offsets.
    #[must_use]
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    /// Returns whether this span is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// Source-text holder with shared ownership semantics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceText {
    name: Arc<str>,
    text: Arc<str>,
}

impl SourceText {
    /// Creates a shared source pair from `name` and `text`.
    #[must_use]
    pub fn new(name: impl Into<Arc<str>>, text: impl Into<Arc<str>>) -> Self {
        Self {
            name: name.into(),
            text: text.into(),
        }
    }

    /// Returns the source name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the source text body.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Slices text by [`Span`], returning `None` when out of bounds.
    #[must_use]
    pub fn slice(&self, span: Span) -> Option<&str> {
        self.text.get(span.start as usize..span.end as usize)
    }
}

/// Diagnostic severity for parser/lint-style findings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    /// Error-level finding.
    Error,
    /// Warning-level finding.
    Warning,
    /// Informational finding.
    Note,
}

/// Structured diagnostic payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    /// Diagnostic code.
    pub code: &'static str,
    /// Severity classification.
    pub severity: Severity,
    /// Human-readable message.
    pub message: String,
    /// Source span.
    pub span: Span,
}

impl Diagnostic {
    /// Constructs an error diagnostic.
    #[must_use]
    pub fn error(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            span,
        }
    }
}
