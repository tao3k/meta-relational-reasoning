//! Executes native Scheme inference and retains MRR admission bindings.

use super::{
    ClosureConfig, ClosureError, ClosureLimits, ClosureReceipt, ClosureStatus, DerivationCandidate,
    DerivationReceiptDigest,
};
use mrr_identity::{FactId, GenerationId, RelationId, RuleId, RulePackId};
use mrr_relation::Value;

use mrr_bundle::ReasoningBundle;
use mrr_gerbil::evaluate_finite_relations;
use mrr_query::{Atom, Term, Variable};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
const RECEIPT_SCHEMA: &[u8] = b"mrr.derivation-receipt.v1";
#[derive(Clone, Debug)]
struct Edge {
    to: String,
    fact: FactId,
}

struct PreparedSourceGraph {
    input_fact_ids: Vec<FactId>,
    edges: Vec<(String, String, FactId)>,
    adjacency: BTreeMap<String, Vec<Edge>>,
}

/// Evaluates the supported closure rules without allocating identities or admitting lineage.
pub fn evaluate_transitive_closure(
    bundle: &ReasoningBundle,
    config: ClosureConfig,
    generation: GenerationId,
    limits: ClosureLimits,
) -> Result<ClosureReceipt, ClosureError> {
    validate_execution_contract(bundle, config)?;
    let prepared = prepare_source_graph(bundle, config, generation, limits)?;
    let paths = run_native(&prepared.edges)?;
    build_receipt(paths, prepared, config, generation, limits)
}

fn validate_execution_contract(
    bundle: &ReasoningBundle,
    config: ClosureConfig,
) -> Result<(), ClosureError> {
    bundle
        .validate()
        .map_err(|error| ClosureError::BundleRejected {
            reason: format!("{error:?}"),
        })?;
    if bundle.transition_count() != 0 {
        return Err(ClosureError::TransitionsRequireMaterializedSnapshot {
            count: bundle.transition_count(),
        });
    }
    validate_relation(bundle, config.source_relation)?;
    validate_relation(bundle, config.derived_relation)?;
    validate_rules(bundle, config)?;
    if let Some(fact) = bundle
        .facts()
        .iter()
        .find(|fact| fact.relation() == config.derived_relation)
    {
        return Err(ClosureError::SeededDerivedRelationUnsupported { fact: fact.id() });
    }
    Ok(())
}

fn prepare_source_graph(
    bundle: &ReasoningBundle,
    config: ClosureConfig,
    generation: GenerationId,
    limits: ClosureLimits,
) -> Result<PreparedSourceGraph, ClosureError> {
    let source_facts: Vec<_> = bundle
        .facts()
        .iter()
        .filter(|fact| fact.relation() == config.source_relation)
        .collect();
    if source_facts.len() > limits.max_input_facts.get() {
        return Err(ClosureError::InputFactBudgetExceeded {
            required: source_facts.len(),
            limit: limits.max_input_facts.get(),
        });
    }

    let mut nodes = BTreeSet::new();
    let mut input_fact_ids = Vec::with_capacity(source_facts.len());
    let mut edges = Vec::with_capacity(source_facts.len());
    let mut adjacency: BTreeMap<String, Vec<Edge>> = BTreeMap::new();
    for fact in source_facts {
        let actual = fact.context().generation();
        if actual != generation {
            return Err(ClosureError::SourceGenerationMismatch {
                fact: fact.id(),
                expected: generation,
                actual,
            });
        }
        let [Value::String(from), Value::String(to)] = fact.values() else {
            return Err(ClosureError::UnsupportedSourceFact { fact: fact.id() });
        };
        nodes.insert(from.clone());
        nodes.insert(to.clone());
        input_fact_ids.push(fact.id());
        edges.push((from.clone(), to.clone(), fact.id()));
        adjacency.entry(from.clone()).or_default().push(Edge {
            to: to.clone(),
            fact: fact.id(),
        });
    }
    input_fact_ids.sort_unstable();
    for outgoing in adjacency.values_mut() {
        outgoing.sort_by(|left, right| left.fact.cmp(&right.fact).then(left.to.cmp(&right.to)));
    }

    let required = nodes.len().saturating_mul(nodes.len());
    if required > limits.max_derived_pairs.get() {
        return Err(ClosureError::DerivedPairBudgetExceeded {
            required,
            limit: limits.max_derived_pairs.get(),
        });
    }

    Ok(PreparedSourceGraph {
        input_fact_ids,
        edges,
        adjacency,
    })
}

fn run_native(
    edges: &[(String, String, FactId)],
) -> Result<Vec<(String, String, usize)>, ClosureError> {
    let nodes: Vec<_> = edges
        .iter()
        .flat_map(|(from, to, _)| [from.clone(), to.clone()])
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let index: BTreeMap<_, _> = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node, index))
        .collect();
    let projected = edges
        .iter()
        .map(|(from, to, _)| (index[from], index[to]))
        .collect();
    let candidate = evaluate_finite_relations(nodes.len(), projected, vec![])
        .map_err(ClosureError::NativeInference)?;
    Ok(candidate
        .paths
        .into_iter()
        .map(|(from, to, distance)| (nodes[from].clone(), nodes[to].clone(), distance))
        .collect())
}

fn build_receipt(
    paths: Vec<(String, String, usize)>,
    prepared: PreparedSourceGraph,
    config: ClosureConfig,
    generation: GenerationId,
    limits: ClosureLimits,
) -> Result<ClosureReceipt, ClosureError> {
    let mut candidates = Vec::with_capacity(paths.len());
    for (from, to, distance) in paths {
        let support = shortest_support(&prepared.adjacency, &from, &to)
            .ok_or(ClosureError::InternalWitnessMismatch)?;
        if support.len() != distance {
            return Err(ClosureError::InternalWitnessMismatch);
        }
        candidates.push(DerivationCandidate {
            relation: config.derived_relation,
            values: [Value::String(from), Value::String(to)],
            rule: if distance == 1 {
                config.base_rule
            } else {
                config.transitive_rule
            },
            generation,
            support,
        });
    }
    candidates.sort_by(|left, right| string_pair(left).cmp(&string_pair(right)));
    let status = if candidates.len() > limits.max_results.get() {
        candidates.truncate(limits.max_results.get());
        ClosureStatus::OutputTruncated
    } else {
        ClosureStatus::Complete
    };

    let digest = receipt_digest(
        config.rule_pack,
        generation,
        &prepared.input_fact_ids,
        status,
        &candidates,
    );
    Ok(ClosureReceipt {
        rule_pack: config.rule_pack,
        input_generation: generation,
        input_fact_ids: prepared.input_fact_ids,
        status,
        candidates,
        digest,
    })
}

fn validate_relation(bundle: &ReasoningBundle, relation: RelationId) -> Result<(), ClosureError> {
    let schema = bundle
        .relations()
        .iter()
        .find(|schema| schema.id() == relation)
        .ok_or(ClosureError::RelationMissing { relation })?;
    if schema.fields().len() != 2 {
        return Err(ClosureError::RelationMustBeBinary {
            relation,
            arity: schema.fields().len(),
        });
    }
    Ok(())
}

fn validate_rules(bundle: &ReasoningBundle, config: ClosureConfig) -> Result<(), ClosureError> {
    let rule_pack = bundle
        .rule_packs()
        .iter()
        .find(|pack| pack.id() == config.rule_pack)
        .ok_or(ClosureError::RulePackMissing {
            rule_pack: config.rule_pack,
        })?;
    let base = rule_pack
        .rules()
        .iter()
        .find(|rule| rule.id() == config.base_rule)
        .ok_or_else(|| rule_membership_error(bundle, config.base_rule, config.rule_pack))?;
    let transitive = rule_pack
        .rules()
        .iter()
        .find(|rule| rule.id() == config.transitive_rule)
        .ok_or_else(|| rule_membership_error(bundle, config.transitive_rule, config.rule_pack))?;

    if !is_base_rule(base.head(), base.body(), config)
        || !is_transitive_rule(transitive.head(), transitive.body(), config)
    {
        return Err(ClosureError::RuleShapeMismatch {
            rule: if !is_base_rule(base.head(), base.body(), config) {
                config.base_rule
            } else {
                config.transitive_rule
            },
        });
    }
    Ok(())
}

fn rule_membership_error(
    bundle: &ReasoningBundle,
    rule: RuleId,
    rule_pack: RulePackId,
) -> ClosureError {
    if bundle.rules().any(|candidate| candidate.id() == rule) {
        ClosureError::RuleNotInPack { rule, rule_pack }
    } else {
        ClosureError::RuleMissing { rule }
    }
}

fn receipt_digest(
    rule_pack: RulePackId,
    generation: GenerationId,
    input_fact_ids: &[FactId],
    status: ClosureStatus,
    candidates: &[DerivationCandidate],
) -> DerivationReceiptDigest {
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, RECEIPT_SCHEMA);
    hash_field(&mut hasher, rule_pack.digest_bytes());
    hash_field(&mut hasher, generation.digest_bytes());
    hash_len(&mut hasher, input_fact_ids.len());
    for fact in input_fact_ids {
        hash_field(&mut hasher, fact.digest_bytes());
    }
    hash_field(
        &mut hasher,
        &[match status {
            ClosureStatus::Complete => 0,
            ClosureStatus::OutputTruncated => 1,
        }],
    );
    hash_len(&mut hasher, candidates.len());
    for candidate in candidates {
        hash_field(&mut hasher, candidate.relation.digest_bytes());
        for value in &candidate.values {
            match value {
                Value::String(value) => hash_field(&mut hasher, value.as_bytes()),
                _ => unreachable!("closure candidates are always strings"),
            }
        }
        hash_field(&mut hasher, candidate.rule.digest_bytes());
        hash_field(&mut hasher, candidate.generation.digest_bytes());
        hash_len(&mut hasher, candidate.support.len());
        for fact in &candidate.support {
            hash_field(&mut hasher, fact.digest_bytes());
        }
    }
    DerivationReceiptDigest(hasher.finalize().into())
}

fn hash_len(hasher: &mut Sha256, len: usize) {
    hasher.update((len as u64).to_be_bytes());
}

fn hash_field(hasher: &mut Sha256, bytes: &[u8]) {
    hash_len(hasher, bytes.len());
    hasher.update(bytes);
}

fn is_base_rule(head: &Atom, body: &[Atom], config: ClosureConfig) -> bool {
    body.len() == 1
        && head.relation == config.derived_relation
        && body[0].relation == config.source_relation
        && same_binary_variables(head, &body[0])
}

fn is_transitive_rule(head: &Atom, body: &[Atom], config: ClosureConfig) -> bool {
    if head.relation != config.derived_relation || body.len() != 2 {
        return false;
    }
    let Some((head_left, head_right)) = binary_variables(head) else {
        return false;
    };
    [(&body[0], &body[1]), (&body[1], &body[0])]
        .into_iter()
        .any(|(closure, source)| {
            let (Some((closure_left, join_left)), Some((join_right, source_right))) =
                (binary_variables(closure), binary_variables(source))
            else {
                return false;
            };
            closure.relation == config.derived_relation
                && source.relation == config.source_relation
                && head_left == closure_left
                && join_left == join_right
                && head_right == source_right
        })
}

fn same_binary_variables(left: &Atom, right: &Atom) -> bool {
    binary_variables(left) == binary_variables(right)
}

fn binary_variables(atom: &Atom) -> Option<(&Variable, &Variable)> {
    match atom.terms.as_slice() {
        [Term::Variable(left), Term::Variable(right)] => Some((left, right)),
        _ => None,
    }
}

fn shortest_support(
    adjacency: &BTreeMap<String, Vec<Edge>>,
    start: &str,
    target: &str,
) -> Option<Vec<FactId>> {
    let mut queue = VecDeque::from([(start.to_owned(), Vec::new())]);
    let mut best_depth = BTreeMap::from([(start.to_owned(), 0_usize)]);
    while let Some((node, support)) = queue.pop_front() {
        if node == target && !support.is_empty() {
            return Some(support);
        }
        for edge in adjacency.get(&node).into_iter().flatten() {
            let next_depth = support.len() + 1;
            let mut next_support = support.clone();
            next_support.push(edge.fact);
            if edge.to == target {
                return Some(next_support);
            }
            if best_depth
                .get(&edge.to)
                .is_some_and(|depth| *depth < next_depth)
            {
                continue;
            }
            best_depth.insert(edge.to.clone(), next_depth);
            queue.push_back((edge.to.clone(), next_support));
        }
    }
    None
}

fn string_pair(candidate: &DerivationCandidate) -> (&str, &str) {
    let [Value::String(left), Value::String(right)] = &candidate.values else {
        unreachable!("closure candidates are always strings")
    };
    (left, right)
}
