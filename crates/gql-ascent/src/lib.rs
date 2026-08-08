#![forbid(unsafe_code)]

use ascent::ascent;
use gql_catalog::{
    ClosureStatus, DerivationError, DerivationRequest, DerivationResult, DerivationWitness,
    DerivedRelationProvider, DerivedTuple, Fact, PredicateDescriptor, RelationAuthority,
    RelationName,
};
use gql_types::{Value, ValueType};

ascent! {
    struct ReachabilityProgram;
    relation edge(String, String);
    relation reachable(String, String);
    reachable(x, y) <-- edge(x, y);
    reachable(x, z) <-- edge(x, y), reachable(y, z);
}

#[derive(Clone, Debug)]
pub struct AscentTransitiveClosure {
    source: RelationName,
    predicates: Vec<PredicateDescriptor>,
    ruleset: String,
}

impl AscentTransitiveClosure {
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
            authority: RelationAuthority::Derived {
                provider: "ascent".into(),
                ruleset: ruleset.clone(),
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
        let mut program = ReachabilityProgram::default();
        program.edge = request
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
            .collect();
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
        reachable.sort();
        let bounded = reachable.len() > request.limits.max_results;
        reachable.truncate(request.limits.max_results);
        let support: Vec<Fact> = request
            .facts
            .iter()
            .filter(|fact| fact.predicate == self.source)
            .cloned()
            .collect();
        let tuples = reachable
            .into_iter()
            .map(|(left, right)| DerivedTuple {
                values: vec![Value::String(left), Value::String(right)],
                witness: DerivationWitness {
                    provider: "ascent".into(),
                    ruleset: self.ruleset.clone(),
                    snapshot: request.snapshot.into(),
                    support: support.clone(),
                },
            })
            .collect();
        Ok(DerivationResult {
            tuples,
            closure: if bounded {
                ClosureStatus::ResultBoundReached
            } else {
                ClosureStatus::Complete
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gql_catalog::{DerivationLimits, DerivedRelationProvider};

    #[test]
    fn derives_transitive_relation_with_snapshot_witness() {
        let provider =
            AscentTransitiveClosure::new("CALLS", "DEPENDS_TRANSITIVELY", "dependency.v1");
        let calls = RelationName("CALLS".into());
        let facts = vec![
            Fact {
                predicate: calls.clone(),
                values: vec![Value::String("A".into()), Value::String("B".into())],
            },
            Fact {
                predicate: calls,
                values: vec![Value::String("B".into()), Value::String("C".into())],
            },
        ];
        let result = provider
            .derive(DerivationRequest {
                predicate: &RelationName("DEPENDS_TRANSITIVELY".into()),
                bindings: &[None, None],
                facts: &facts,
                snapshot: "snapshot-1",
                limits: DerivationLimits::default(),
            })
            .expect("registered predicate derives");
        assert!(result.tuples.iter().any(
            |tuple| tuple.values == vec![Value::String("A".into()), Value::String("C".into())]
        ));
        assert!(
            result
                .tuples
                .iter()
                .all(|tuple| tuple.witness.snapshot == "snapshot-1")
        );
    }
}
