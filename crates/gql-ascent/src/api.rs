//! Ascent-backed provider for transitive closure derived relations.
use ascent::ascent;
use gql_catalog::{
    CatalogName, GraphName, PredicateDescriptor, RelationIdentity, RelationName,
};
use gql_reasoning::{
    ClosureStatus, DerivationError, DerivationRequest, DerivationResult,
    DerivationWitness, DerivedRelationProvider, DerivedTuple, Fact,
};
use gql_types::{Value, ValueType};

/// Ascent-based transitive-closure relation provider.
#[derive(Clone, Debug)]
pub struct AscentTransitiveClosure {
    source: RelationName,
    predicates: Vec<PredicateDescriptor>,
    ruleset: String,
}

ascent! {
    struct ReachabilityProgram;
    relation edge(String, String);
    relation reachable(String, String);
    reachable(x, y) <-- edge(x, y);
    reachable(x, z) <-- edge(x, y), reachable(y, z);
}

impl AscentTransitiveClosure {
    /// Constructs a reachability provider for one derived relation.
    #[must_use]
    pub fn new(
        source: impl Into<String>,
        derived: impl Into<String>,
        ruleset: impl Into<String>,
    ) -> Self {
        let source = RelationName(source.into());
        let ruleset = ruleset.into();
        let predicates = vec![PredicateDescriptor {
            name: RelationName(derived.into()),
            columns: vec![ValueType::String, ValueType::String],
            relation_identity: RelationIdentity {
                catalog: CatalogName("ascent".into()),
                graph: GraphName("derived-graph".into()),
                schema: None,
                node_types: Vec::new(),
                edge_types: Vec::new(),
            },
            authority: gql_catalog::RelationAuthority::Asserted {
                source: "ascent".into(),
            },
        }];
        Self {
            source,
            predicates,
            ruleset,
        }
    }
}

impl DerivedRelationProvider for AscentTransitiveClosure {
    fn predicates(&self) -> &[PredicateDescriptor] {
        &self.predicates
    }

    /// Runs derivation for the provided request.
    fn derive(&self, request: DerivationRequest<'_>) -> Result<DerivationResult, DerivationError> {
        if request.predicate != &self.predicates[0].name {
            return Err(DerivationError::UnknownPredicate(request.predicate.clone()));
        }
        if request.bindings.len() != 2 {
            return Err(DerivationError::InvalidBindingArity {
                expected: 2,
                actual: request.bindings.len(),
            });
        }
        let mut program = ReachabilityProgram {
            edge: request
                .facts
                .iter()
                .filter_map(|fact| {
                    if fact.predicate != self.source {
                        return None;
                    }
                    match fact.values.as_slice() {
                        [Value::String(left), Value::String(right)] => {
                            Some((left.clone(), right.clone()))
                        }
                        _ => None,
                    }
                })
                .collect(),
            ..Default::default()
        };
        program.run();

        let matches_binding = |position: usize, value: &str| {
            request.bindings[position]
                .as_ref()
                .is_none_or(|binding| binding == &Value::String(value.to_owned()))
        };
        let mut reachable: Vec<_> = program
            .reachable
            .into_iter()
            .filter(|(left, right)| matches_binding(0, left) && matches_binding(1, right))
            .collect();
        let truncated_output =
            request.limits.max_results > 0 && reachable.len() > request.limits.max_results;
        reachable.sort();
        if request.limits.max_results > 0 {
            reachable.truncate(request.limits.max_results);
        } else {
            reachable.clear();
        }

        let support: Vec<Fact> = request
            .facts
            .iter()
            .filter(|fact| fact.predicate == self.source)
            .cloned()
            .collect();
        let rule_budget_exhausted = request.limits.max_rule_firings == 0
            || request.limits.max_depth == 0
            || request.limits.wall_budget_ms == 0
            || reachable.len() > request.limits.max_derived_facts;
        let tuples = reachable
            .into_iter()
            .enumerate()
            .map(|(index, (left, right))| DerivedTuple {
                values: vec![Value::String(left), Value::String(right)],
                witness: DerivationWitness {
                    provider: "ascent".into(),
                    ruleset: self.ruleset.clone(),
                    snapshot: request.snapshot.into(),
                    derivation_id: index as u64,
                    support_set_id: 0,
                    support: support.clone(),
                },
            })
            .collect();
        Ok(DerivationResult {
            tuples,
            closure: if rule_budget_exhausted {
                ClosureStatus::BudgetExhausted
            } else if truncated_output {
                ClosureStatus::OutputTruncated
            } else {
                ClosureStatus::Complete
            },
        })
    }
}

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
