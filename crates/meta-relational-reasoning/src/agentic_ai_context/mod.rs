//! Optional facade-owned Context admission and presentation stages.

mod admission;
mod compose;
mod composition_receipt;
mod error;
mod exact_restore;
mod manifest;
mod restore;
mod revision;
mod selection;
mod selection_restore;
#[cfg(feature = "agentic-ai-context-tokens")]
mod tokens;

pub use admission::{
    AdmittedAgenticAiContext, AdmittedAgenticAiContextMaterialization,
    AgenticAiContextAdmissionRequest, AgenticAiContextMaterializationRequest,
    admit_agentic_ai_context,
};
pub use error::AgenticAiContextAdmissionError;
pub use manifest::{
    AGENTIC_AI_CONTEXT_MANIFEST_SCHEMA, AgenticAiContextManifest, AgenticAiContextManifestRecord,
};
pub use restore::{AgenticAiContextRestoreRequest, restore_agentic_ai_context};
#[cfg(feature = "agentic-ai-context-tokens")]
pub use tokens::{
    AgenticAiContextTokenBindingRequest, AgenticAiContextTokenizationError,
    AgenticAiContextTokenizationRequest, SourceBoundAgenticAiContextTokens,
};

pub use compose::{
    AgenticAiContextComposedMaterializationRequest, AgenticAiContextCompositionError,
    AgenticAiContextCompositionRequest,
};
pub use composition_receipt::AgenticAiContextCompositionReceipt;
pub use revision::{
    AgenticAiContextRevisionReceipt, AgenticAiContextRevisionRequest,
    compare_agentic_ai_context_revision,
};

pub use selection::{
    AdmittedAgenticAiContextQuerySelection, AgenticAiContextQuerySelectionRequest,
    select_agentic_ai_context_from_query,
};

pub use selection_restore::{
    AGENTIC_AI_CONTEXT_QUERY_SELECTION_SCHEMA, AgenticAiContextQuerySelectionRecord,
    AgenticAiContextQuerySelectionRestoreRequest, restore_agentic_ai_context_query_selection,
};

pub use exact_restore::{
    AdmittedExactAgenticAiContextQuerySelection, AgenticAiContextExactSelectionRestoreRequest,
    restore_agentic_ai_context_query_selection_exact, select_agentic_ai_context_from_query_exact,
};
