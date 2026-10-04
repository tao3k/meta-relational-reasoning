//! Receipts from actual Rust admission/worklist/revision and identity encoders.
pub mod support;

use std::{env, fs, path::PathBuf};

use meta_relational_reasoning::{
    AgenticAiContextExactSelectionRestoreRequest, AgenticAiContextRevisionRequest, FactId,
    compare_agentic_ai_context_revision, restore_agentic_ai_context_query_selection_exact,
    select_agentic_ai_context_from_query, select_agentic_ai_context_from_query_exact,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn labels(ids: &[FactId], count: usize) -> Vec<usize> {
    let mut values: Vec<_> = ids
        .iter()
        .map(|id| {
            (1..=count)
                .find(|index| support::fact_id(*index) == *id)
                .unwrap()
        })
        .collect();
    values.sort_unstable();
    values
}

fn closure_case(
    name: &str,
    count: usize,
    roots: &[usize],
    deps: &[(usize, Vec<usize>)],
    mandatory: &[usize],
    temporal: &[usize],
) -> Value {
    let source = support::source(&(1..=count).map(|id| (id, 7)).collect::<Vec<_>>());
    let rows = support::candidate(
        &source,
        &roots.iter().map(|id| (*id, 7)).collect::<Vec<_>>(),
    );
    let mut request = support::request(count, roots.len().max(1), deps);
    request.limits.max_dependency_edges = support::nz(count * count + count);
    request.contract.required = mandatory.iter().map(|id| support::fact_id(*id)).collect();
    request.contract.temporal_receipts = temporal.iter().map(|id| support::fact_id(*id)).collect();
    let selection = select_agentic_ai_context_from_query(
        &source.bundle,
        &source.snapshot,
        &source.query,
        &rows,
        request,
    )
    .unwrap();
    println!("REFINEMENT-RUST-OK: closure {name}");
    json!({"name":name, "source": (1..=count).collect::<Vec<_>>(), "roots":roots,
        "mandatory":mandatory,"temporal":temporal,"dependencies":deps,
        "selected":labels(selection.context().closure().elements(),count)})
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = PathBuf::from(
        env::args()
            .nth(1)
            .unwrap_or_else(|| "/tmp/mrr-context-refinement.json".into()),
    );
    let mut closures = vec![
        closure_case(
            "chain",
            4,
            &[4],
            &[(2, vec![1]), (3, vec![2]), (4, vec![3])],
            &[],
            &[],
        ),
        closure_case(
            "cycle",
            4,
            &[4],
            &[(1, vec![4]), (2, vec![1]), (3, vec![2]), (4, vec![3])],
            &[],
            &[],
        ),
        closure_case(
            "diamond",
            4,
            &[4, 4],
            &[(2, vec![1]), (3, vec![1]), (4, vec![2, 3])],
            &[],
            &[],
        ),
        closure_case("empty-contract", 4, &[], &[(2, vec![1])], &[2], &[3]),
    ];
    for seed in 0..64_u64 {
        let mut state = seed + 1;
        let mut deps = Vec::new();
        for id in 1..=6 {
            let mut edges = Vec::new();
            for dependency in 1..=6 {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                if state >> 62 == 0 {
                    edges.push(dependency);
                }
            }
            deps.push((id, edges));
        }
        closures.push(closure_case(
            &format!("finite-{seed}"),
            6,
            &[seed as usize % 6 + 1],
            &deps,
            &[],
            &[],
        ));
    }
    let source = support::source(&[(1, 7), (2, 7), (3, 7), (4, 7)]);
    let rows = support::candidate(&source, &[(3, 7), (3, 7)]);
    let old_deps = vec![(2, vec![1]), (3, vec![2])];
    let new_deps = vec![(2, vec![]), (3, vec![2]), (4, vec![2])];
    let request = support::request(8, 8, &old_deps);
    let old = select_agentic_ai_context_from_query_exact(
        &source.bundle,
        &source.snapshot,
        &source.query,
        &rows,
        request.clone(),
    )?;
    let record = old.export_record(request.result_limits, support::nz(65536))?;
    let restored = restore_agentic_ai_context_query_selection_exact(
        &source.bundle,
        &source.snapshot,
        AgenticAiContextExactSelectionRestoreRequest {
            record: &record,
            expected: &old,
            limits: request.limits,
            result_limits: request.result_limits,
            max_result_bytes: support::nz(65536),
        },
    )?;
    assert_eq!(old, restored);
    println!("REFINEMENT-RUST-OK: exact trusted-reference restore");
    let new = select_agentic_ai_context_from_query(
        &source.bundle,
        &source.snapshot,
        &source.query,
        &rows,
        support::request(8, 8, &new_deps),
    )?;
    let revision = compare_agentic_ai_context_revision(AgenticAiContextRevisionRequest {
        old: old.selection().context(),
        new: new.context(),
        old_bundle: &source.bundle,
        new_bundle: &source.bundle,
        old_snapshot: &source.snapshot,
        new_snapshot: &source.snapshot,
    })?;
    let revision = json!({"source":[1,2,3,4],"old_dependencies":old_deps,"new_dependencies":new_deps,
        "old_selected":labels(old.selection().context().closure().elements(),4),"new_selected":labels(new.context().closure().elements(),4),
        "changed":labels(revision.revision().changed(),4),"invalidated":labels(revision.revision().invalidated(),4),
        "reusable":labels(revision.revision().reusable().elements(),4)});
    println!("REFINEMENT-RUST-OK: old/new reverse dependency impact");
    let selection = old.selection();
    let manifest = selection.context().manifest();
    let manifest_bytes = manifest.canonical_bytes()?;
    let selection_bytes = selection.canonical_identity_bytes()?;
    assert_eq!(
        <[u8; 32]>::from(Sha256::digest(&manifest_bytes)),
        *manifest.digest()
    );
    assert_eq!(
        <[u8; 32]>::from(Sha256::digest(&selection_bytes)),
        *selection.digest()
    );
    let identities = vec![
        json!({"name":"manifest","bytes":manifest_bytes,"value":manifest.record(),"digest":manifest.digest()}),
        json!({"name":"selection","bytes":selection_bytes,"value":["mrr.agentic-ai-context.query-selection.v1",manifest.digest(),selection.result_receipt().digest(),selection.column()],"digest":selection.digest()}),
    ];
    let hash_vectors: Vec<_> = [
        vec![],
        b"abc".to_vec(),
        vec![97; 55],
        vec![97; 56],
        vec![97; 63],
        vec![97; 64],
        vec![97; 65],
        vec![97; 1024],
    ]
    .into_iter()
    .map(|bytes| {
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        json!({"bytes":bytes,"digest":digest})
    })
    .collect();
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &output,
        serde_json::to_vec_pretty(&json!({"schema":"mrr.context-refinement-receipts.v1",
        "closures":closures,"revision":revision,"identities":identities,"hash_vectors":hash_vectors}))?,
    )?;
    println!(
        "REFINEMENT-RUST-OK: actual CBOR preimages and SHA-256 receipts {}",
        output.display()
    );
    Ok(())
}
