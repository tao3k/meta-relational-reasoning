//! Complete source-bound native transport, publication, restart and revocation.
use std::{error::Error, num::NonZeroUsize};

use meta_relational_reasoning::{
    ExternalRevisionIdentity, FiniteProblem, FiniteTransformationCatalog, FiniteTransport,
    GenerationId, RevisionBinding, SemanticSnapshot, TransformationBinding, TransformationLimits,
    TransformationPlanCandidate, TransformationResultStore, TransformationStep, Value, ValueSchema,
    admit_transformation, transformation_value_digest,
};

fn main() -> Result<(), Box<dyn Error>> {
    let archive = std::env::args()
        .nth(1)
        .ok_or("usage: finite_transformation <archive.cbor>")?;
    let limits = TransformationLimits {
        max_bytes: NonZeroUsize::new(16384).ok_or("byte bound")?,
        max_schema_nodes: NonZeroUsize::new(32).ok_or("node bound")?,
        max_schema_depth: NonZeroUsize::new(8).ok_or("depth bound")?,
        max_dependencies: NonZeroUsize::new(16).ok_or("dependency bound")?,
        max_steps: NonZeroUsize::new(4).ok_or("step bound")?,
    };
    let generation = GenerationId::from_canonical_bytes(b"finite-example-generation")?;
    let revision = RevisionBinding::admit(
        ExternalRevisionIdentity::new("local-owner", "finite-specification", "v1")
            .map_err(|error| format!("{error:?}"))?,
        generation,
    )
    .map_err(|error| format!("{error:?}"))?;
    let snapshot = SemanticSnapshot::admit(generation, vec![revision])
        .map_err(|error| format!("{error:?}"))?;
    // This installation explicitly authorizes one pure local specification cut.
    let binding = TransformationBinding::new(&snapshot, [1; 32], [2; 32], [3; 32], 0, 0)?;
    let source = FiniteProblem {
        domain: [4; 32],
        answers: 3,
        correct: vec![vec![0], vec![1], vec![2]],
    };
    let target = FiniteProblem {
        domain: [5; 32],
        answers: 3,
        correct: vec![vec![1], vec![2], vec![0]],
    };
    let catalog = FiniteTransformationCatalog::new(
        vec![FiniteTransport {
            lineage: vec![],
            source,
            target,
            forward: vec![0, 1, 2],
            extract: vec![vec![2, 0, 1]; 3],
        }],
        binding.clone(),
        limits,
    )?;
    let definition = &catalog.definitions()[0];
    let admission = admit_transformation(
        definition,
        &catalog.evidence(0)?,
        &binding,
        limits,
        &catalog,
    )?;
    let input = transformation_value_digest(&ValueSchema::Integer, &Value::Integer(1), limits)?;
    let support = *admission.digest();
    let candidate = TransformationPlanCandidate {
        binding,
        source: definition.source.clone(),
        target: definition.target.clone(),
        input,
        solver: catalog.solver(&definition.target)?,
        steps: vec![TransformationStep {
            admission,
            input,
            target_input: input,
            parameters: catalog.parameters()?,
        }],
    };
    let mut store = TransformationResultStore::open(&archive, limits)?;
    let execution = store.execute_and_publish(&candidate, Value::Integer(1), &catalog)?;
    println!(
        "EXECUTION-OK answer={:?} freshness={:?} cells={}",
        execution.answer(),
        store.freshness(),
        catalog.extraction_cells()
    );
    drop(store);
    let mut store = TransformationResultStore::open(&archive, limits)?;
    println!("REOPEN freshness={:?}", store.freshness());
    store.replay(&candidate, Value::Integer(1), &catalog)?;
    println!("REPLAY-OK freshness={:?}", store.freshness());
    store.revoke_support(&catalog, &support)?;
    println!(
        "REVOKED freshness={:?} retained={:?}",
        store.freshness(),
        store.historical_answer()
    );
    Ok(())
}
