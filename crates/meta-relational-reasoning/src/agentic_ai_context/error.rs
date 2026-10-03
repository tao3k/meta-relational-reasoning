//! Typed rejection shared by Context admission, manifests and adapters.

use crate::{AgenticAiContextError, QueryCatalogBindingError};
use std::fmt;

/// Reasons a Context selection cannot retain the supplied MRR source binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgenticAiContextAdmissionError {
    QueryBinding(QueryCatalogBindingError),
    QueryBindingMismatch,
    SourceBundleMismatch,
    Context(AgenticAiContextError),
    ManifestEncoding(String),
    ManifestSchemaMismatch,
    ManifestMismatch,
    ManifestBudget,
    CompositionEncoding(String),
    CompositionProducerMismatch,
    CompositionMismatch,
    RevisionEncoding(String),
    RevisionMismatch,
}

impl fmt::Display for AgenticAiContextAdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AgenticAiContextAdmissionError {}
