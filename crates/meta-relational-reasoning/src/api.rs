//! Backend-hidden public engine for the language-neutral MRR contract graph.

use core::num::NonZeroUsize;

use mrr_ascent::{ClosureConfig, ClosureLimits, evaluate_transitive_closure};

use crate::{
    CandidateIdentities, ClosureAdmissionError, MaterializedClosure, admit_closure_candidates,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeductionPlan(ClosureConfig);

impl DeductionPlan {
    #[must_use]
    pub const fn transitive_closure(
        source_relation: mrr_identity::RelationId,
        derived_relation: mrr_identity::RelationId,
        rule_pack: mrr_identity::RulePackId,
        base_rule: mrr_identity::RuleId,
        transitive_rule: mrr_identity::RuleId,
    ) -> Self {
        Self(ClosureConfig::new(
            source_relation,
            derived_relation,
            rule_pack,
            base_rule,
            transitive_rule,
        ))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeductionLimits(ClosureLimits);

impl DeductionLimits {
    #[must_use]
    pub const fn new(
        max_input_facts: NonZeroUsize,
        max_derived_pairs: NonZeroUsize,
        max_results: NonZeroUsize,
    ) -> Self {
        Self(ClosureLimits::new(
            max_input_facts,
            max_derived_pairs,
            max_results,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct MrrEngineBuilder {
    bundle: Option<mrr_bundle::ReasoningBundle>,
}

impl MrrEngineBuilder {
    #[must_use]
    pub const fn new() -> Self {
        Self { bundle: None }
    }

    #[must_use]
    pub fn with_bundle(mut self, bundle: mrr_bundle::ReasoningBundle) -> Self {
        self.bundle = Some(bundle);
        self
    }

    pub fn build(self) -> Result<MrrEngine, EngineBuildError> {
        self.bundle
            .map(|bundle| MrrEngine { bundle })
            .ok_or(EngineBuildError::MissingBundle)
    }
}

impl Default for MrrEngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EngineBuildError {
    MissingBundle,
}

#[derive(Clone, Debug)]
pub struct MrrEngine {
    bundle: mrr_bundle::ReasoningBundle,
}

impl MrrEngine {
    #[must_use]
    pub const fn builder() -> MrrEngineBuilder {
        MrrEngineBuilder::new()
    }

    #[must_use]
    pub const fn bundle(&self) -> &mrr_bundle::ReasoningBundle {
        &self.bundle
    }

    pub fn admit(&self, facts: Vec<mrr_relation::Fact>) -> Result<Self, mrr_bundle::BundleError> {
        let mut declaration = self.bundle.declaration().clone();
        declaration.facts.extend(facts);
        mrr_bundle::ReasoningBundle::admit(declaration).map(|bundle| Self { bundle })
    }

    pub fn query(
        &self,
        query: mrr_identity::QueryId,
    ) -> Result<&mrr_query::MetaQueryIr, EngineQueryError> {
        self.bundle
            .query_templates()
            .iter()
            .find(|template| template.id() == query)
            .map(mrr_bundle::QueryTemplate::query)
            .ok_or(EngineQueryError::UnknownQuery(query))
    }

    pub fn derive(
        &self,
        plan: DeductionPlan,
        generation: mrr_identity::GenerationId,
        limits: DeductionLimits,
    ) -> Result<mrr_ascent::ClosureReceipt, mrr_ascent::ClosureError> {
        evaluate_transitive_closure(&self.bundle, plan.0, generation, limits.0)
    }

    pub fn materialize(
        &self,
        receipt: &mrr_ascent::ClosureReceipt,
        from: mrr_identity::GenerationId,
        to: mrr_identity::GenerationId,
        identities: &[CandidateIdentities],
    ) -> Result<MaterializedClosure, ClosureAdmissionError> {
        admit_closure_candidates(receipt, from, to, identities)
    }

    pub fn why(
        &self,
        graph: &mrr_lineage::LineageGraph,
        result: mrr_identity::LineageNodeId,
    ) -> Result<mrr_lineage::ExplanationGraph, mrr_lineage::WhyError> {
        mrr_lineage::why(graph, result)
    }

    pub fn why_one_witness(
        &self,
        graph: &mrr_lineage::LineageGraph,
        result: mrr_identity::LineageNodeId,
    ) -> Result<mrr_lineage::ExplanationGraph, mrr_lineage::WhyError> {
        mrr_lineage::why_one_witness(graph, result)
    }

    pub fn why_not(
        &self,
        goal: &mrr_query::Atom,
        generation: mrr_identity::GenerationId,
        limits: mrr_logic::WhyNotLimits,
    ) -> Result<mrr_logic::WhyNotReceipt, mrr_logic::WhyNotError> {
        let rules: Vec<_> = self.bundle.rules().cloned().collect();
        mrr_logic::why_not(goal, generation, &rules, self.bundle.facts(), limits)
    }

    pub fn impact(
        &self,
        graph: &mrr_lineage::LineageGraph,
        fact: mrr_identity::FactId,
    ) -> Result<mrr_lineage::ImpactGraph, mrr_lineage::ImpactError> {
        mrr_lineage::impact(graph, fact)
    }

    pub fn check_safety(
        &self,
        system: &mrr_transition::TransitionSystem,
        limits: mrr_transition::SafetyLimits,
    ) -> Result<mrr_transition::SafetyCheckReceipt, mrr_transition::TransitionModelError> {
        mrr_transition::check_safety(system, limits)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EngineQueryError {
    UnknownQuery(mrr_identity::QueryId),
}
