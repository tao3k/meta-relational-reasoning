use core::num::NonZeroUsize;
use std::{
    collections::BTreeMap,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::{
    BundleBoundClosure, CandidateIdentities, ClosureAdmissionError,
    ClosureCandidateComparisonError, ClosureCandidateRow, ClosurePairComparisonError,
    DeductionError, DeductionLimits, DeductionPlan, DerivationId, EntityId, EvidenceCompleteness,
    ExternalRevisionIdentity, Fact, FactId, FactProvenance, FactValidity, GenerationId,
    GenerationTransitionError, MrrEngine, ReasoningBundle, ReasoningBundleDeclaration,
    RelationAuthority, RelationContext, RelationField, RelationId, RelationSchema, RevisionBinding,
    Rule, RuleId, RulePack, RulePackId, SemanticSnapshot, Term, Value, ValueSchema, Variable,
    admit_closure_candidates,
};
use mrr_lineage::LineageError;
use mrr_query::Atom;

macro_rules! id {
    ($kind:ident, $value:expr) => {
        $kind::from_canonical_bytes(format!("test:{}:{}", stringify!($kind), $value))
            .expect("test identity input is canonical")
    };
}

fn variable(name: &str) -> Variable {
    Variable::new(name).expect("test variable")
}

fn atom(relation: RelationId, variables: &[&str]) -> Atom {
    Atom {
        relation,
        terms: variables
            .iter()
            .map(|name| Term::Variable(variable(name)))
            .collect(),
    }
}

fn binary_schema(relation: RelationId, predicate: &str) -> RelationSchema {
    RelationSchema::new(
        relation,
        predicate,
        vec![
            RelationField::new("from", ValueSchema::String, false).expect("from field"),
            RelationField::new("to", ValueSchema::String, false).expect("to field"),
        ],
        Vec::new(),
    )
    .expect("binary schema")
}

fn source_fact(identity: u128, relation: RelationId, from: &str, to: &str) -> Fact {
    source_fact_at(identity, relation, from, to, id!(GenerationId, 51))
}

fn source_fact_at(
    identity: u128,
    relation: RelationId,
    from: &str,
    to: &str,
    generation: GenerationId,
) -> Fact {
    let authority = id!(EntityId, identity);
    Fact::new(
        id!(FactId, identity),
        relation,
        vec![Value::String(from.into()), Value::String(to.into())],
        RelationContext::new(
            generation,
            RelationAuthority::Entity(authority),
            FactProvenance::Source(authority),
            EvidenceCompleteness::Complete,
            FactValidity::Valid,
        )
        .expect("source context"),
    )
}

fn source_snapshot_at(facts: &[Fact], generation: GenerationId) -> Vec<Fact> {
    facts
        .iter()
        .map(|fact| {
            let context = fact.context();
            Fact::new(
                fact.id(),
                fact.relation(),
                fact.values().to_vec(),
                RelationContext::new(
                    generation,
                    context.authority(),
                    context.provenance(),
                    context.completeness(),
                    context.validity(),
                )
                .expect("source snapshot context"),
            )
        })
        .collect()
}

fn snapshot(generation: GenerationId) -> SemanticSnapshot {
    snapshot_at_revision(generation, "fixture")
}

fn snapshot_at_revision(generation: GenerationId, revision: &str) -> SemanticSnapshot {
    SemanticSnapshot::admit(
        generation,
        vec![
            RevisionBinding::admit(
                ExternalRevisionIdentity::new("test", "closure-source", revision).unwrap(),
                generation,
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

fn fixture() -> (ReasoningBundle, DeductionPlan) {
    let edge = id!(RelationId, 1);
    let reachable = id!(RelationId, 2);
    let base = id!(RuleId, 10);
    let transitive = id!(RuleId, 11);
    let relations = vec![
        binary_schema(edge, "edge"),
        binary_schema(reachable, "reachable"),
    ];
    let facts = vec![
        source_fact(100, edge, "Ada", "Bob"),
        source_fact(101, edge, "Bob", "Cy"),
    ];
    let rules = vec![
        Rule::new(
            base,
            atom(reachable, &["x", "y"]),
            vec![atom(edge, &["x", "y"])],
        )
        .expect("base rule"),
        Rule::new(
            transitive,
            atom(reachable, &["x", "z"]),
            vec![atom(reachable, &["x", "y"]), atom(edge, &["y", "z"])],
        )
        .expect("transitive rule"),
    ];
    (
        ReasoningBundle::admit(ReasoningBundleDeclaration {
            relations,
            facts,
            rule_packs: vec![RulePack::new(id!(RulePackId, 1), rules)],
            ..ReasoningBundleDeclaration::default()
        })
        .expect("bundle admission"),
        DeductionPlan::transitive_closure(edge, reachable, id!(RulePackId, 1), base, transitive),
    )
}

fn limits(results: usize) -> DeductionLimits {
    DeductionLimits::new(
        NonZeroUsize::new(8).unwrap(),
        NonZeroUsize::new(16).unwrap(),
        NonZeroUsize::new(results).unwrap(),
    )
}

fn evaluate_transitive_closure(
    bundle: &ReasoningBundle,
    plan: DeductionPlan,
    generation: GenerationId,
    limits: DeductionLimits,
) -> Result<BundleBoundClosure, DeductionError> {
    MrrEngine::builder()
        .with_bundle(bundle.clone())
        .build()
        .expect("engine")
        .derive(plan, &snapshot(generation), limits)
}

fn identities(count: usize) -> Vec<CandidateIdentities> {
    (0..count)
        .map(|offset| {
            CandidateIdentities::new(
                id!(FactId, 1_000 + offset as u128),
                id!(DerivationId, 2_000 + offset as u128),
            )
        })
        .collect()
}

fn candidate_rows(evaluation: &BundleBoundClosure) -> Vec<ClosureCandidateRow> {
    evaluation
        .closure()
        .candidates()
        .iter()
        .map(|candidate| {
            let [Value::String(from), Value::String(to)] = candidate.values() else {
                panic!("closure candidate has string endpoints")
            };
            ClosureCandidateRow::new(
                from.clone(),
                to.clone(),
                candidate.support().len(),
                candidate.rule(),
                candidate.support().to_vec(),
            )
        })
        .collect()
}

fn scheme_fixture_output(recipe: &str, request: &str) -> String {
    let root =
        PathBuf::from(std::env::var_os("MRR_POO_FLOW_ROOT").expect("POO Flow checkout path"));
    let mut command = Command::new("just");
    command
        .current_dir(&root)
        .arg(recipe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut loadpath = format!("{}:{}", root.display(), root.join(".gerbil/lib").display());
    if let Some(extra) = std::env::var_os("MRR_POO_FLOW_EXTRA_LOADPATH") {
        loadpath.push(':');
        loadpath.push_str(&extra.to_string_lossy());
    }
    command.env("GERBIL_PATH", root.join(".gerbil"));
    command.env("GERBIL_LOADPATH", loadpath);
    let mut child = command.spawn().expect("launch POO Flow Scheme evaluator");
    child
        .stdin
        .take()
        .expect("Scheme fixture stdin")
        .write_all(request.as_bytes())
        .expect("write source edges to Scheme evaluator");
    let output = child
        .wait_with_output()
        .expect("collect Scheme pair output");
    assert!(
        output.status.success(),
        "Scheme evaluator failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("Scheme output is UTF-8")
}

fn scheme_closure_pairs(edges: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut request = String::from("(8");
    for &(from, to) in edges {
        assert!(from.parse::<u32>().is_ok() && to.parse::<u32>().is_ok());
        request.push(' ');
        request.push_str(from);
        request.push(' ');
        request.push_str(to);
    }
    request.push_str(")\n");
    scheme_fixture_output("ascent-pairs", &request)
        .lines()
        .map(|line| {
            let (from, to) = line.split_once('\t').expect("Scheme pair has two columns");
            assert!(
                from.parse::<u32>().is_ok() && to.parse::<u32>().is_ok(),
                "Scheme pair has numeric nodes"
            );
            (from.to_owned(), to.to_owned())
        })
        .collect()
}

struct PriorSnapshot {
    result: BundleBoundClosure,
    generation: GenerationId,
    pairs: Vec<(String, String)>,
}

#[test]
#[ignore = "requires a built POO Flow checkout in MRR_POO_FLOW_ROOT"]
fn live_scheme_pairs_match_ascent_for_source_snapshots() {
    let (bundle, plan) = fixture();
    let edge = id!(RelationId, 1);
    let snapshots: &[&[(&str, &str)]] = &[
        &[("1", "2"), ("2", "3"), ("1", "4"), ("4", "3")],
        &[("1", "2"), ("1", "4"), ("4", "3")],
        &[("1", "2"), ("1", "4")],
        &[("1", "2"), ("2", "3"), ("3", "4"), ("4", "2"), ("1", "3")],
        &[("1", "2"), ("2", "3"), ("3", "4"), ("1", "3")],
    ];
    let mut prior: Option<PriorSnapshot> = None;
    for (snapshot_index, edges) in snapshots.iter().enumerate() {
        let generation = id!(GenerationId, 51 + snapshot_index);
        let mut declaration = bundle.declaration().clone();
        declaration.facts = edges
            .iter()
            .enumerate()
            .map(|(index, &(from, to))| {
                source_fact_at(100 + index as u128, edge, from, to, generation)
            })
            .collect();
        let engine = MrrEngine::builder()
            .with_bundle(ReasoningBundle::admit(declaration).expect("source snapshot"))
            .build()
            .expect("MRR engine");
        let result = engine
            .derive(plan, &snapshot(generation), limits(16))
            .expect("complete Ascent result");
        let pairs = scheme_closure_pairs(edges);
        assert_eq!(
            engine.compare_closure_pairs(&result, &snapshot(generation), &pairs),
            Ok(()),
            "Scheme/Ascent mismatch at source snapshot {snapshot_index}"
        );
        if let Some(old) = prior.take() {
            assert!(matches!(
                engine.compare_closure_pairs(&old.result, &snapshot(old.generation), &old.pairs),
                Err(ClosurePairComparisonError::SourceBundleMismatch { .. })
            ));
        }
        prior = Some(PriorSnapshot {
            result,
            generation,
            pairs,
        });
    }
}

#[derive(Debug, Eq, PartialEq)]
struct SupportRow {
    from: String,
    to: String,
    distance: usize,
    rule: String,
    support: Vec<usize>,
}

fn scheme_support_rows(
    edges: &[(u128, &str, &str)],
    ranks: &BTreeMap<FactId, usize>,
) -> Vec<SupportRow> {
    let mut request = String::from("(8");
    for &(identity, from, to) in edges {
        assert!(from.parse::<u32>().is_ok() && to.parse::<u32>().is_ok());
        request.push_str(&format!(" {from} {to} {}", ranks[&id!(FactId, identity)]));
    }
    request.push_str(")\n");
    scheme_fixture_output("ascent-candidates", &request)
        .lines()
        .map(|line| {
            let fields: Vec<_> = line.split('\t').collect();
            assert!(fields.len() >= 5, "Scheme support row has a witness");
            SupportRow {
                from: fields[0].to_owned(),
                to: fields[1].to_owned(),
                distance: fields[2].parse().expect("Scheme distance"),
                rule: fields[3].to_owned(),
                support: fields[4..]
                    .iter()
                    .map(|label| label.parse().expect("Scheme source rank"))
                    .collect(),
            }
        })
        .collect()
}

#[test]
#[ignore = "requires a built POO Flow checkout in MRR_POO_FLOW_ROOT"]
fn live_scheme_shortest_support_matches_ascent_for_source_snapshots() {
    let (bundle, plan) = fixture();
    let edge = id!(RelationId, 1);
    let snapshots: &[&[(u128, &str, &str)]] = &[
        &[
            (100, "1", "2"),
            (101, "2", "3"),
            (99, "1", "4"),
            (103, "4", "3"),
        ],
        &[(100, "1", "2"), (99, "1", "4"), (103, "4", "3")],
        &[(100, "1", "2"), (99, "1", "4")],
        &[
            (100, "1", "2"),
            (101, "2", "3"),
            (102, "3", "4"),
            (103, "4", "2"),
            (104, "1", "3"),
        ],
        &[
            (100, "1", "2"),
            (101, "2", "3"),
            (102, "3", "4"),
            (104, "1", "3"),
        ],
    ];
    for (snapshot_index, edges) in snapshots.iter().enumerate() {
        let generation = id!(GenerationId, 51 + snapshot_index);
        let mut fact_ids: Vec<_> = edges
            .iter()
            .map(|(identity, _, _)| id!(FactId, identity))
            .collect();
        fact_ids.sort_unstable();
        let ranks: BTreeMap<_, _> = fact_ids
            .iter()
            .copied()
            .enumerate()
            .map(|(rank, identity)| (identity, rank))
            .collect();
        let mut declaration = bundle.declaration().clone();
        declaration.facts = edges
            .iter()
            .map(|&(identity, from, to)| source_fact_at(identity, edge, from, to, generation))
            .collect();
        let engine = MrrEngine::builder()
            .with_bundle(ReasoningBundle::admit(declaration).expect("source snapshot"))
            .build()
            .expect("MRR engine");
        let receipt = engine
            .derive(plan, &snapshot(generation), limits(16))
            .expect("complete Ascent result");
        let actual: Vec<_> = scheme_support_rows(edges, &ranks)
            .into_iter()
            .map(|row| {
                let rule = match row.rule.as_str() {
                    "base" => id!(RuleId, 10),
                    "transitive" => id!(RuleId, 11),
                    other => panic!("unknown Scheme rule kind: {other}"),
                };
                ClosureCandidateRow::new(
                    row.from,
                    row.to,
                    row.distance,
                    rule,
                    row.support.into_iter().map(|rank| fact_ids[rank]).collect(),
                )
            })
            .collect();
        assert_eq!(
            engine.compare_closure_candidates(&receipt, &snapshot(generation), &actual),
            Ok(()),
            "support mismatch at snapshot {snapshot_index}"
        );
    }
}

#[test]
fn compares_external_pairs_only_against_the_exact_complete_bundle_generation() {
    let (bundle, plan) = fixture();
    let generation = id!(GenerationId, 51);
    let engine = MrrEngine::builder()
        .with_bundle(bundle.clone())
        .build()
        .expect("engine");
    let complete = engine
        .derive(plan, &snapshot(generation), limits(16))
        .expect("complete Ascent result");
    let pairs = vec![
        ("Bob".into(), "Cy".into()),
        ("Ada".into(), "Cy".into()),
        ("Ada".into(), "Bob".into()),
    ];
    assert_eq!(
        engine.compare_closure_pairs(&complete, &snapshot(generation), &pairs),
        Ok(())
    );

    assert_eq!(
        engine.compare_closure_pairs(&complete, &snapshot(id!(GenerationId, 52)), &pairs),
        Err(ClosurePairComparisonError::GenerationMismatch {
            expected: id!(GenerationId, 52),
            actual: generation,
        })
    );
    assert_eq!(
        engine.compare_closure_pairs(&complete, &snapshot(generation), &pairs[..2]),
        Err(ClosurePairComparisonError::PairCountMismatch {
            expected: 3,
            actual: 2,
        })
    );
    assert_eq!(
        engine.compare_closure_pairs(
            &complete,
            &snapshot(generation),
            &[pairs[0].clone(), pairs[0].clone(), pairs[2].clone()],
        ),
        Err(ClosurePairComparisonError::DuplicatePair(
            "Bob".into(),
            "Cy".into(),
        ))
    );
    assert_eq!(
        engine.compare_closure_pairs(
            &complete,
            &snapshot(generation),
            &[
                pairs[0].clone(),
                pairs[1].clone(),
                ("Ada".into(), "Eve".into())
            ],
        ),
        Err(ClosurePairComparisonError::PairSetMismatch)
    );

    let truncated = engine
        .derive(plan, &snapshot(generation), limits(1))
        .expect("bounded output");
    assert_eq!(
        engine.compare_closure_pairs(&truncated, &snapshot(generation), &pairs),
        Err(ClosurePairComparisonError::IncompleteReceipt)
    );

    let mut changed = bundle.declaration().clone();
    *changed
        .facts
        .iter_mut()
        .find(|fact| fact.id() == id!(FactId, 100))
        .expect("source fact 100") = source_fact(100, id!(RelationId, 1), "Ada", "Eve");
    let changed = MrrEngine::builder()
        .with_bundle(ReasoningBundle::admit(changed).expect("changed bundle"))
        .build()
        .expect("changed engine");
    assert_eq!(
        changed.compare_closure_pairs(&complete, &snapshot(generation), &pairs),
        Err(ClosurePairComparisonError::SourceBundleMismatch {
            expected: changed.bundle().id(),
            actual: bundle.id(),
        })
    );

    let mut empty_declaration = bundle.declaration().clone();
    empty_declaration.facts.clear();
    let empty_engine = MrrEngine::builder()
        .with_bundle(ReasoningBundle::admit(empty_declaration).expect("empty source bundle"))
        .build()
        .expect("empty engine");
    let empty_generation = id!(GenerationId, 53);
    let empty = empty_engine
        .derive(plan, &snapshot(empty_generation), limits(16))
        .expect("complete empty closure");
    assert_eq!(
        empty_engine.compare_closure_pairs(&empty, &snapshot(empty_generation), &[]),
        Ok(())
    );
    assert_eq!(
        empty_engine.compare_closure_pairs(&empty, &snapshot(generation), &[]),
        Err(ClosurePairComparisonError::GenerationMismatch {
            expected: generation,
            actual: empty_generation,
        })
    );
}

#[test]
fn rejects_same_generation_from_another_source_revision() {
    let (bundle, plan) = fixture();
    let generation = id!(GenerationId, 51);
    let engine = MrrEngine::builder().with_bundle(bundle).build().unwrap();
    let original = snapshot_at_revision(generation, "source-a");
    let revised = snapshot_at_revision(generation, "source-b");
    let result = engine.derive(plan, &original, limits(16)).unwrap();
    assert_eq!(result.snapshot_digest(), original.digest());
    assert_ne!(original.digest(), revised.digest());
    let expected = *revised.digest();
    let actual = *original.digest();
    assert_eq!(
        engine.compare_closure_pairs(&result, &revised, &[]),
        Err(ClosurePairComparisonError::SnapshotMismatch { expected, actual })
    );
    assert_eq!(
        engine.materialize(
            &result,
            id!(GenerationId, 50),
            &revised,
            &identities(result.closure().candidates().len()),
        ),
        Err(ClosureAdmissionError::SnapshotMismatch { expected, actual })
    );
}

#[test]
fn compares_complete_candidate_evidence_and_rejects_forged_fields() {
    let (bundle, plan) = fixture();
    let generation = id!(GenerationId, 51);
    let snapshot = snapshot(generation);
    let engine = MrrEngine::builder().with_bundle(bundle).build().unwrap();
    let evaluation = engine.derive(plan, &snapshot, limits(16)).unwrap();
    let mut rows = candidate_rows(&evaluation);
    rows.reverse();
    assert_eq!(
        engine.compare_closure_candidates(&evaluation, &snapshot, &rows),
        Ok(())
    );

    let expected = &evaluation.closure().candidates()[0];
    let [Value::String(from), Value::String(to)] = expected.values() else {
        panic!("closure candidate has string endpoints")
    };
    let index = rows.len() - 1;
    let baseline = rows[index].clone();
    rows[index] = ClosureCandidateRow::new(
        from.clone(),
        to.clone(),
        expected.support().len() + 1,
        expected.rule(),
        expected.support().to_vec(),
    );
    assert!(matches!(
        engine.compare_closure_candidates(&evaluation, &snapshot, &rows),
        Err(ClosureCandidateComparisonError::DistanceMismatch { .. })
    ));
    rows[index] = ClosureCandidateRow::new(
        from.clone(),
        to.clone(),
        expected.support().len(),
        id!(RuleId, 999),
        expected.support().to_vec(),
    );
    assert!(matches!(
        engine.compare_closure_candidates(&evaluation, &snapshot, &rows),
        Err(ClosureCandidateComparisonError::RuleMismatch { .. })
    ));
    rows[index] = ClosureCandidateRow::new(
        from.clone(),
        to.clone(),
        expected.support().len(),
        expected.rule(),
        vec![id!(FactId, 999)],
    );
    assert!(matches!(
        engine.compare_closure_candidates(&evaluation, &snapshot, &rows),
        Err(ClosureCandidateComparisonError::SupportMismatch { .. })
    ));
    rows[index] = rows[0].clone();
    assert!(matches!(
        engine.compare_closure_candidates(&evaluation, &snapshot, &rows),
        Err(ClosureCandidateComparisonError::Pairs(
            ClosurePairComparisonError::DuplicatePair(..)
        ))
    ));
    rows[index] = baseline;
    assert!(matches!(
        engine.compare_closure_candidates(
            &evaluation,
            &snapshot_at_revision(generation, "different"),
            &rows
        ),
        Err(ClosureCandidateComparisonError::Pairs(
            ClosurePairComparisonError::SnapshotMismatch { .. }
        ))
    ));
}

#[test]
fn rejects_unrelated_bundle_fact_from_another_snapshot_generation() {
    let (bundle, plan) = fixture();
    let mut declaration = bundle.declaration().clone();
    let other = id!(RelationId, 3);
    declaration.relations.push(binary_schema(other, "other"));
    declaration.facts.push(source_fact_at(
        102,
        other,
        "Ada",
        "Bob",
        id!(GenerationId, 50),
    ));
    let engine = MrrEngine::builder()
        .with_bundle(ReasoningBundle::admit(declaration).unwrap())
        .build()
        .unwrap();
    assert_eq!(
        engine.derive(plan, &snapshot(id!(GenerationId, 51)), limits(16)),
        Err(DeductionError::BundleGenerationMismatch {
            fact: id!(FactId, 102),
            expected: id!(GenerationId, 51),
            actual: id!(GenerationId, 50),
        })
    );
}

#[test]
fn compares_the_poo_cyclic_closure_and_withdrawn_snapshot() {
    let (bundle, plan) = fixture();
    let edge = id!(RelationId, 1);
    let mut declaration = bundle.declaration().clone();
    declaration.facts = vec![
        source_fact(100, edge, "1", "2"),
        source_fact(101, edge, "2", "3"),
        source_fact(102, edge, "3", "4"),
        source_fact(103, edge, "4", "2"),
        source_fact(104, edge, "1", "3"),
    ];
    let engine = MrrEngine::builder()
        .with_bundle(ReasoningBundle::admit(declaration.clone()).expect("cyclic bundle"))
        .build()
        .expect("cyclic engine");
    let first_generation = id!(GenerationId, 51);
    let first = engine
        .derive(plan, &snapshot(first_generation), limits(16))
        .expect("complete cyclic closure");
    let pairs = vec![
        ("1".into(), "2".into()),
        ("1".into(), "3".into()),
        ("1".into(), "4".into()),
        ("2".into(), "2".into()),
        ("2".into(), "3".into()),
        ("2".into(), "4".into()),
        ("3".into(), "2".into()),
        ("3".into(), "3".into()),
        ("3".into(), "4".into()),
        ("4".into(), "2".into()),
        ("4".into(), "3".into()),
        ("4".into(), "4".into()),
    ];
    assert_eq!(
        engine.compare_closure_pairs(&first, &snapshot(first_generation), &pairs),
        Ok(())
    );

    declaration
        .facts
        .retain(|fact| fact.id() != id!(FactId, 103));
    declaration.facts = source_snapshot_at(&declaration.facts, id!(GenerationId, 52));
    let withdrawn = MrrEngine::builder()
        .with_bundle(ReasoningBundle::admit(declaration).expect("withdrawn bundle"))
        .build()
        .expect("withdrawn engine");
    let next_generation = id!(GenerationId, 52);
    let second = withdrawn
        .derive(plan, &snapshot(next_generation), limits(16))
        .expect("complete closure after withdrawal");
    let remaining = vec![
        ("1".into(), "2".into()),
        ("1".into(), "3".into()),
        ("1".into(), "4".into()),
        ("2".into(), "3".into()),
        ("2".into(), "4".into()),
        ("3".into(), "4".into()),
    ];
    assert_eq!(
        withdrawn.compare_closure_pairs(&second, &snapshot(next_generation), &remaining),
        Ok(())
    );
    assert_eq!(
        withdrawn.compare_closure_pairs(&first, &snapshot(first_generation), &pairs),
        Err(ClosurePairComparisonError::SourceBundleMismatch {
            expected: withdrawn.bundle().id(),
            actual: engine.bundle().id(),
        })
    );
}

#[test]
fn admits_complete_candidates_into_exact_lineage_and_transition_outputs() {
    let (bundle, config) = fixture();
    let from = id!(GenerationId, 50);
    let to = id!(GenerationId, 51);
    let receipt =
        evaluate_transitive_closure(&bundle, config, to, limits(16)).expect("closure evaluation");
    let assigned = identities(receipt.closure().candidates().len());

    let materialized = admit_closure_candidates(&bundle, &receipt, from, &snapshot(to), &assigned)
        .expect("complete receipt admission");

    assert_eq!(materialized.transition().from(), from);
    assert_eq!(materialized.transition().to(), to);
    assert_eq!(
        materialized.transition().insertions().len(),
        receipt.closure().candidates().len()
    );
    assert_eq!(materialized.transition().retractions(), &[]);
    assert_eq!(materialized.derivations().len(), assigned.len());
    assert_eq!(materialized.receipt().rule_pack(), id!(RulePackId, 1));
    assert_eq!(receipt.source_bundle(), bundle.id());
    assert_eq!(materialized.receipt().input_generation(), to);
    assert_eq!(
        materialized.receipt().input_fact_ids(),
        receipt.closure().input_fact_ids()
    );
    assert_eq!(
        materialized.receipt().derived_fact_ids(),
        assigned
            .iter()
            .map(|identity| identity.fact())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        materialized.receipt().derivation_ids(),
        assigned
            .iter()
            .map(|identity| identity.derivation())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        materialized.receipt().closure_status(),
        crate::ClosureStatus::Complete
    );
    assert_ne!(materialized.receipt().digest(), &[0; 32]);
    for ((candidate, assigned), derivation) in receipt
        .closure()
        .candidates()
        .iter()
        .zip(&assigned)
        .zip(materialized.derivations())
    {
        assert_eq!(derivation.id(), assigned.derivation());
        assert_eq!(derivation.output().id(), assigned.fact());
        assert_eq!(derivation.output().relation(), candidate.relation());
        assert_eq!(derivation.output().values(), candidate.values());
        assert_eq!(derivation.output().context().generation(), to);
        assert_eq!(
            derivation.output().context().provenance(),
            FactProvenance::Derivation(assigned.derivation())
        );
        assert_eq!(derivation.rule(), candidate.rule());
        assert_eq!(derivation.generation(), to);
        assert_eq!(derivation.support(), candidate.support());
    }
}

#[test]
fn rejects_a_receipt_from_another_bundle_with_the_same_fact_ids() {
    let (bundle, plan) = fixture();
    let to = id!(GenerationId, 51);
    let receipt = evaluate_transitive_closure(&bundle, plan, to, limits(16))
        .expect("complete original closure");
    let mut declaration = bundle.declaration().clone();
    *declaration
        .facts
        .iter_mut()
        .find(|fact| fact.id() == id!(FactId, 100))
        .expect("source fact 100") = source_fact(100, id!(RelationId, 1), "Ada", "Eve");
    let changed = ReasoningBundle::admit(declaration).expect("changed source bundle");
    let error = ClosureAdmissionError::SourceBundleMismatch {
        expected: changed.id(),
        actual: bundle.id(),
    };
    let assigned = identities(receipt.closure().candidates().len());

    assert_eq!(
        admit_closure_candidates(
            &changed,
            &receipt,
            id!(GenerationId, 50),
            &snapshot(to),
            &assigned
        ),
        Err(error.clone())
    );
    let engine = MrrEngine::builder()
        .with_bundle(changed)
        .build()
        .expect("changed engine");
    assert_eq!(
        engine.materialize(&receipt, id!(GenerationId, 50), &snapshot(to), &assigned),
        Err(error)
    );
}

#[test]
fn rejects_an_empty_closure_receipt_for_another_generation() {
    let (bundle, plan) = fixture();
    let mut declaration = bundle.declaration().clone();
    declaration.facts.clear();
    let empty = ReasoningBundle::admit(declaration).expect("empty source bundle");
    let evaluated = id!(GenerationId, 51);
    let requested = id!(GenerationId, 52);
    let receipt = evaluate_transitive_closure(&empty, plan, evaluated, limits(16))
        .expect("complete empty closure");
    assert!(receipt.closure().candidates().is_empty());

    assert_eq!(
        admit_closure_candidates(
            &empty,
            &receipt,
            id!(GenerationId, 50),
            &snapshot(requested),
            &[]
        ),
        Err(ClosureAdmissionError::GenerationMismatch {
            expected: requested,
            actual: evaluated,
        })
    );
}

#[test]
fn rejects_truncated_or_cross_generation_receipts_before_materialization() {
    let (bundle, config) = fixture();
    let from = id!(GenerationId, 50);
    let evaluated_generation = id!(GenerationId, 51);
    let truncated = evaluate_transitive_closure(&bundle, config, evaluated_generation, limits(1))
        .expect("bounded closure evaluation");
    assert_eq!(
        admit_closure_candidates(
            &bundle,
            &truncated,
            from,
            &snapshot(evaluated_generation),
            &identities(1)
        ),
        Err(ClosureAdmissionError::IncompleteReceipt)
    );

    let complete = evaluate_transitive_closure(&bundle, config, evaluated_generation, limits(16))
        .expect("complete closure evaluation");
    let wrong_target = id!(GenerationId, 52);
    assert_eq!(
        admit_closure_candidates(
            &bundle,
            &complete,
            from,
            &snapshot(wrong_target),
            &identities(complete.closure().candidates().len()),
        ),
        Err(ClosureAdmissionError::GenerationMismatch {
            expected: wrong_target,
            actual: evaluated_generation,
        })
    );
}

#[test]
fn rejects_identity_cardinality_duplicates_and_self_support() {
    let (bundle, config) = fixture();
    let from = id!(GenerationId, 50);
    let to = id!(GenerationId, 51);
    let receipt = evaluate_transitive_closure(&bundle, config, to, limits(16))
        .expect("complete closure evaluation");

    assert_eq!(
        admit_closure_candidates(&bundle, &receipt, from, &snapshot(to), &[]),
        Err(ClosureAdmissionError::IdentityCountMismatch {
            candidates: receipt.closure().candidates().len(),
            identities: 0,
        })
    );

    let mut duplicate = identities(receipt.closure().candidates().len());
    duplicate[1] = CandidateIdentities::new(duplicate[0].fact(), duplicate[1].derivation());
    assert_eq!(
        admit_closure_candidates(&bundle, &receipt, from, &snapshot(to), &duplicate),
        Err(ClosureAdmissionError::DuplicateFactId(duplicate[0].fact()))
    );

    let mut duplicate = identities(receipt.closure().candidates().len());
    duplicate[1] = CandidateIdentities::new(duplicate[1].fact(), duplicate[0].derivation());
    assert_eq!(
        admit_closure_candidates(&bundle, &receipt, from, &snapshot(to), &duplicate),
        Err(ClosureAdmissionError::DuplicateDerivationId(
            duplicate[0].derivation()
        ))
    );

    assert_eq!(
        admit_closure_candidates(
            &bundle,
            &receipt,
            to,
            &snapshot(to),
            &identities(receipt.closure().candidates().len())
        ),
        Err(ClosureAdmissionError::Transition(
            GenerationTransitionError::SameGeneration
        ))
    );

    let mut self_support = identities(receipt.closure().candidates().len());
    self_support[0] = CandidateIdentities::new(
        receipt.closure().candidates()[0].support()[0],
        self_support[0].derivation(),
    );
    assert_eq!(
        admit_closure_candidates(&bundle, &receipt, from, &snapshot(to), &self_support),
        Err(ClosureAdmissionError::Lineage(LineageError::SelfSupport))
    );
}
