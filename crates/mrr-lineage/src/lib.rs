//! Derivation lineage and support sets for MRR facts.
#![forbid(unsafe_code)]
mod api;
pub use api::{
    Derivation, ExplanationGraph, ImpactError, ImpactGraph, LineageEdge, LineageEdgeKind,
    LineageError, LineageGraph, LineageGraphError, LineageNode, LineageNodeKind, WhyError, impact,
    why, why_one_witness,
};
pub use mrr_identity::{
    DerivationId, FactId, GenerationId, LineageEdgeId, LineageNodeId, QueryOperatorId, RuleId,
    StateId, TransitionId,
};
pub use mrr_relation::Fact;
#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
