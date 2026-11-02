//! Finite projection of admitted transformation edges into MRR ASCENT.
use super::transformation::digest;
use super::transformation::{
    TransformationAdmission, TransformationBinding, TransformationEndpoint, TransformationError,
    TransformationLimits,
};
use super::transformation_projection::{
    TransformationRouteCandidate, verify_transformation_projection,
};
use mrr_relation::ValueSchema;
/// Bounded ASCENT consequence output. Routes remain candidates; they carry no
/// concrete forward inputs, parameters, premise checks or solver admission.
#[derive(Clone, Debug)]
pub struct TransformationRouteSearch {
    binding: TransformationBinding,
    closure: mrr_deduction::ClosureReceipt,
    routes: Vec<TransformationRouteCandidate>,
    checked_projection: [u8; 32],
}
impl TransformationRouteSearch {
    #[must_use]
    pub fn binding(&self) -> &TransformationBinding {
        &self.binding
    }
    #[must_use]
    pub fn closure(&self) -> &mrr_deduction::ClosureReceipt {
        &self.closure
    }
    #[must_use]
    pub fn routes(&self) -> &[TransformationRouteCandidate] {
        self.routes.as_slice()
    }
    /// Independent finite adjacency/path validation receipt.
    #[must_use]
    pub fn checked_projection(&self) -> &[u8; 32] {
        &self.checked_projection
    }
    /// Existence within exactly this enumerated admitted cut, never solvability
    /// or reachability in an unenumerated external graph.
    #[must_use]
    pub fn route_in_cut_truth(
        &self,
        source: &TransformationEndpoint,
        target: &TransformationEndpoint,
    ) -> crate::TruthStatus {
        if self
            .routes
            .iter()
            .any(|route| &route.source == source && &route.target == target)
        {
            return crate::TruthStatus::True;
        }
        match self.closure.status() {
            mrr_deduction::ClosureStatus::Complete => crate::TruthStatus::False,
            mrr_deduction::ClosureStatus::OutputTruncated => crate::TruthStatus::Incomplete,
        }
    }
}

/// Project exactly one admitted cut into the existing MRR ASCENT closure
/// adapter. Native ASCENT refinement remains the adapter owner's obligation.
/// Output truncation is retained in `closure().status()`; absence is not FALSE.
pub fn search_transformation_routes(
    edges: &[TransformationAdmission],
    binding: &TransformationBinding,
    limits: TransformationLimits,
    closure_limits: mrr_deduction::ClosureLimits,
) -> Result<TransformationRouteSearch, TransformationError> {
    use mrr_bundle::{ReasoningBundle, ReasoningBundleDeclaration, RulePack};
    use mrr_identity::{EntityId, FactId, RelationId, RuleId, RulePackId};
    use mrr_logic::Rule;
    use mrr_query::{Atom, Term, Variable};
    use mrr_relation::{
        EvidenceCompleteness, Fact, FactProvenance, FactValidity, RelationAuthority,
        RelationContext, RelationField, RelationSchema, Value,
    };
    use std::collections::BTreeMap;
    // Projection storage has an explicit bound before cloning any schema.
    if edges.len() > limits.max_dependencies.get() {
        return Err(TransformationError::Budget);
    }
    let edge_relation = RelationId::from_canonical_bytes(b"mrr.transformation.edge.v1")
        .map_err(|_| TransformationError::Encoding)?;
    let reach_relation = RelationId::from_canonical_bytes(b"mrr.transformation.reachable.v1")
        .map_err(|_| TransformationError::Encoding)?;
    let base = RuleId::from_canonical_bytes(b"mrr.transformation.base.v1")
        .map_err(|_| TransformationError::Encoding)?;
    let transitive = RuleId::from_canonical_bytes(b"mrr.transformation.transitive.v1")
        .map_err(|_| TransformationError::Encoding)?;
    let pack = RulePackId::from_canonical_bytes(b"mrr.transformation.rules.v1")
        .map_err(|_| TransformationError::Encoding)?;
    let authority = EntityId::from_canonical_bytes(binding.snapshot)
        .map_err(|_| TransformationError::Encoding)?;
    let schema = |id, name| {
        RelationSchema::new(
            id,
            name,
            vec![
                RelationField::new("source", ValueSchema::String, false)
                    .map_err(|_| TransformationError::InvalidSchema)?,
                RelationField::new("target", ValueSchema::String, false)
                    .map_err(|_| TransformationError::InvalidSchema)?,
            ],
            vec![],
        )
        .map_err(|_| TransformationError::InvalidSchema)
    };
    let atom = |relation, names: &[&str]| -> Result<Atom, TransformationError> {
        Ok(Atom {
            relation,
            terms: names
                .iter()
                .map(|name| {
                    Variable::new(*name)
                        .map(Term::Variable)
                        .ok_or(TransformationError::InvalidSchema)
                })
                .collect::<Result<_, _>>()?,
        })
    };
    let rules = vec![
        Rule::new(
            base,
            atom(reach_relation, &["x", "y"])?,
            vec![atom(edge_relation, &["x", "y"])?],
        )
        .map_err(|_| TransformationError::InvalidSchema)?,
        Rule::new(
            transitive,
            atom(reach_relation, &["x", "z"])?,
            vec![
                atom(reach_relation, &["x", "y"])?,
                atom(edge_relation, &["y", "z"])?,
            ],
        )
        .map_err(|_| TransformationError::InvalidSchema)?,
    ];
    let mut facts = Vec::new();
    let mut endpoints = BTreeMap::new();
    let mut supports = BTreeMap::new();
    for edge in edges {
        if edge.binding != *binding {
            return Err(TransformationError::BindingMismatch);
        }
        let mut names = Vec::new();
        for endpoint in [&edge.definition.source, &edge.definition.target] {
            let node = digest(&("mrr.transformation-endpoint.v1", endpoint), limits)?;
            let name = EntityId::from_canonical_bytes(node)
                .map_err(|_| TransformationError::Encoding)?
                .to_string();
            endpoints.insert(name.clone(), endpoint.clone());
            names.push(Value::String(name));
        }
        let fact =
            FactId::from_canonical_bytes(edge.digest).map_err(|_| TransformationError::Encoding)?;
        if supports.insert(fact, edge.digest).is_some() {
            return Err(TransformationError::DuplicateDependency);
        }
        let context = RelationContext::new(
            binding.generation,
            RelationAuthority::Entity(authority),
            FactProvenance::Source(authority),
            EvidenceCompleteness::Complete,
            FactValidity::Valid,
        )
        .map_err(|_| TransformationError::InvalidSchema)?;
        facts.push(Fact::new(fact, edge_relation, names, context));
    }
    let bundle = ReasoningBundle::admit(ReasoningBundleDeclaration {
        relations: vec![
            schema(edge_relation, "transformation-edge")?,
            schema(reach_relation, "transformation-reachable")?,
        ],
        facts,
        rule_packs: vec![RulePack::new(pack, rules)],
        ..ReasoningBundleDeclaration::default()
    })
    .map_err(|_| TransformationError::InvalidSchema)?;
    let closure = mrr_deduction::evaluate_transitive_closure(
        &bundle,
        mrr_deduction::ClosureConfig::new(edge_relation, reach_relation, pack, base, transitive),
        binding.generation,
        closure_limits,
    )
    .map_err(|error| match error {
        mrr_deduction::ClosureError::InputFactBudgetExceeded { .. }
        | mrr_deduction::ClosureError::DerivedPairBudgetExceeded { .. } => {
            TransformationError::Budget
        }
        _ => TransformationError::Rejected,
    })?;
    let mut routes = Vec::new();
    for candidate in closure.candidates() {
        if candidate.support().len() > limits.max_steps.get() {
            return Err(TransformationError::Budget);
        }
        let [Value::String(source), Value::String(target)] = candidate.values() else {
            return Err(TransformationError::Rejected);
        };
        routes.push(TransformationRouteCandidate {
            source: endpoints
                .get(source)
                .ok_or(TransformationError::Rejected)?
                .clone(),
            target: endpoints
                .get(target)
                .ok_or(TransformationError::Rejected)?
                .clone(),
            edges: candidate
                .support()
                .iter()
                .map(|id| {
                    supports
                        .get(id)
                        .copied()
                        .ok_or(TransformationError::Rejected)
                })
                .collect::<Result<_, _>>()?,
        });
    }
    let projection = verify_transformation_projection(edges, &routes, closure.status(), limits)?;
    let checked_projection = digest(&(projection, binding), limits)?;
    Ok(TransformationRouteSearch {
        binding: binding.clone(),
        closure,
        routes,
        checked_projection,
    })
}
