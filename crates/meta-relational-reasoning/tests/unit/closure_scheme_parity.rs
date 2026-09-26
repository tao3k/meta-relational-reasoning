use std::{
    collections::BTreeMap,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use super::{
    BundleBoundClosure, ClosureCandidateRow, ClosurePairComparisonError, FactId, GenerationId,
    MrrEngine, ReasoningBundle, RelationId, RuleId, fixture, limits, snapshot, source_fact_at,
};

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
