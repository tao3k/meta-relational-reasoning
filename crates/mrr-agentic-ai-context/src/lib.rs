//! MRR-owned, storage-neutral Agentic AI Context contracts.
//!
//! This first slice selects explicit fact identities and their declared required
//! closure. Source/query admission stays in the MRR facade; temporal meaning,
//! rendering, tokenization and actual serving-cache observations stay with their
//! existing owners. Supplied precedence is checked for exact closure membership,
//! not recomputed by a second Rust C4 implementation.
#![forbid(unsafe_code)]

mod composition;
mod evidence;
mod materialization;
#[cfg(feature = "token-layout")]
mod reuse;
mod state;
mod worklist;

pub use composition::{
    AgenticAiContextComposer, AgenticAiContextCompositionGraph,
    AgenticAiContextCompositionGraphInput, AgenticAiContextCompositionNode,
    AgenticAiContextCompositionProducer,
};
pub use materialization::{
    AgenticAiContextComposition, AgenticAiContextMaterialization, AgenticAiContextRenderedElement,
    AgenticAiContextSpan,
};
#[cfg(feature = "token-layout")]
pub use reuse::{
    AgenticAiContextComputationalIdentity, AgenticAiContextReuseEligibility,
    AgenticAiContextTokenLayout, AgenticAiContextTokenizer,
};
pub use state::{
    AgenticAiContextClosure, AgenticAiContextContract, AgenticAiContextElement,
    AgenticAiContextError, AgenticAiContextLimits, AgenticAiContextQuery, AgenticAiContextRevision,
    AgenticAiContextState, AgenticAiContextStateInput, SemanticReuseCertificate,
};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
