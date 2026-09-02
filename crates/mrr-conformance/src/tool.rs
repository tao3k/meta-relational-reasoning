//! Production-facing bounded closure adapter over the public MRR facade.

use core::num::NonZeroUsize;
use std::{error::Error, fmt};

use meta_relational_reasoning::{
    Atom, CandidateIdentities, ClosureStatus, DeductionLimits, DeductionPlan, DerivationId,
    EntityId, EvidenceCompleteness, Fact, FactId, FactProvenance, FactValidity, GenerationId,
    MaterializedClosure, MrrEngine, ReasoningBundle, ReasoningBundleDeclaration, RelationAuthority,
    RelationCardinality, RelationContext, RelationField, RelationId, RelationSchema, Rule, RuleId,
    RulePack, RulePackId, Term, Value, ValueType, Variable,
};

/// Provider-normalized graph closure request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClosureToolInput {
    /// Start node for the reachability decision.
    pub source: String,
    /// Target node for the reachability decision.
    pub target: String,
    /// Directed visible-world edges.
    pub edges: Vec<(String, String)>,
}

/// Atomic MRR materialization and the requested decision.
#[derive(Debug)]
pub struct ClosureToolReceipt {
    /// Whether the complete closure contains the requested pair.
    pub reachable: bool,
    /// Fully admitted closure; no result exists before this value does.
    pub closure: MaterializedClosure,
}

/// Fail-closed adapter error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClosureToolError(String);

impl fmt::Display for ClosureToolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for ClosureToolError {}

macro_rules! live_id {
    ($kind:ident, $domain:expr, $value:expr) => {
        $kind::from_canonical_bytes(format!("mrr-live:{}:{}", $domain, $value).as_bytes())
            .map_err(|error| ClosureToolError(format!("invalid identity: {error:?}")))?
    };
}

fn variable(name: &str) -> Result<Variable, ClosureToolError> {
    Variable::new(name).ok_or_else(|| ClosureToolError("invalid variable".into()))
}

fn atom(relation: RelationId, names: &[&str]) -> Result<Atom, ClosureToolError> {
    Ok(Atom {
        relation,
        terms: names
            .iter()
            .map(|name| variable(name).map(Term::Variable))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn schema(relation: RelationId, name: &str) -> Result<RelationSchema, ClosureToolError> {
    RelationSchema::new(
        relation,
        name,
        ["from", "to"]
            .into_iter()
            .map(|field| RelationField::new(field, ValueType::String))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| ClosureToolError(format!("invalid relation field: {error:?}")))?,
        RelationCardinality::ManyToMany,
    )
    .map_err(|error| ClosureToolError(format!("invalid relation schema: {error:?}")))
}

fn execute_closure_tool(input: &ClosureToolInput) -> Result<ClosureToolReceipt, ClosureToolError> {
    if input.edges.is_empty() {
        return Err(ClosureToolError("at least one edge is required".into()));
    }
    let edge = live_id!(RelationId, "relation", "edge");
    let reachable = live_id!(RelationId, "relation", "reachable");
    let generation = live_id!(GenerationId, "generation", 1);
    let base = live_id!(RuleId, "rule", "base");
    let transitive = live_id!(RuleId, "rule", "transitive");
    let rule_pack = live_id!(RulePackId, "rule-pack", "closure");
    let authority = live_id!(EntityId, "authority", "live-tool");
    let facts = input
        .edges
        .iter()
        .enumerate()
        .map(|(index, (from, to))| {
            Ok(Fact::new(
                live_id!(FactId, "input-fact", index),
                edge,
                vec![Value::String(from.clone()), Value::String(to.clone())],
                RelationContext::new(
                    generation,
                    RelationAuthority::Entity(authority),
                    FactProvenance::Source(authority),
                    EvidenceCompleteness::Complete,
                    FactValidity::Valid,
                ),
            ))
        })
        .collect::<Result<Vec<_>, ClosureToolError>>()?;
    let rules = vec![
        Rule::new(
            base,
            atom(reachable, &["x", "y"])?,
            vec![atom(edge, &["x", "y"])?],
        ),
        Rule::new(
            transitive,
            atom(reachable, &["x", "z"])?,
            vec![atom(reachable, &["x", "y"])?, atom(edge, &["y", "z"])?],
        ),
    ]
    .into_iter()
    .collect::<Result<Vec<_>, _>>()
    .map_err(|error| ClosureToolError(format!("invalid closure rule: {error:?}")))?;
    let bundle = ReasoningBundle::admit(ReasoningBundleDeclaration {
        relations: vec![schema(edge, "edge")?, schema(reachable, "reachable")?],
        facts,
        rule_packs: vec![RulePack::new(rule_pack, rules)],
        ..ReasoningBundleDeclaration::default()
    })
    .map_err(|error| ClosureToolError(format!("bundle admission failed: {error:?}")))?;
    let engine = MrrEngine::builder()
        .with_bundle(bundle)
        .build()
        .map_err(|error| ClosureToolError(format!("engine construction failed: {error:?}")))?;
    let closure = engine
        .derive(
            DeductionPlan::transitive_closure(edge, reachable, rule_pack, base, transitive),
            generation,
            DeductionLimits::new(
                NonZeroUsize::new(64).expect("constant is non-zero"),
                NonZeroUsize::new(4096).expect("constant is non-zero"),
                NonZeroUsize::new(4096).expect("constant is non-zero"),
            ),
        )
        .map_err(|error| ClosureToolError(format!("closure failed: {error:?}")))?;
    if closure.status() != ClosureStatus::Complete {
        return Err(ClosureToolError("closure is incomplete".into()));
    }
    let result_is_reachable = closure.candidates().iter().any(|candidate| {
        *candidate.values()
            == [
                Value::String(input.source.clone()),
                Value::String(input.target.clone()),
            ]
    });
    let identities = closure
        .candidates()
        .iter()
        .enumerate()
        .map(|(index, _)| {
            Ok(CandidateIdentities::new(
                live_id!(FactId, "derived-fact", index),
                live_id!(DerivationId, "derivation", index),
            ))
        })
        .collect::<Result<Vec<_>, ClosureToolError>>()?;
    let from = live_id!(GenerationId, "generation", 0);
    let materialized = engine
        .materialize(&closure, from, generation, &identities)
        .map_err(|error| ClosureToolError(format!("materialization failed: {error:?}")))?;
    Ok(ClosureToolReceipt {
        reachable: result_is_reachable,
        closure: materialized,
    })
}

/// Execute one bounded closure using only the public MRR facade.
pub fn run_closure_tool(input: &ClosureToolInput) -> Result<ClosureToolReceipt, ClosureToolError> {
    execute_closure_tool(input)
}
