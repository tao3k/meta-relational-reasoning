//! Source compilation and original Search admission share one isolated owner.
use meta_relational_reasoning::{
    FactId, GenerationId, QueryOperatorId, SearchFactor, SearchFactorRole, SearchFrameworkLimits,
    SearchFrameworkStatus, SearchObservation, TemporalHost, configure_native_worker,
    evaluate_search_factors, shutdown_native_worker,
};
use mrr_frontends::{ParserLanguage, QueryFrontend};
use std::num::NonZeroUsize;
#[test]
fn source_and_search_share_configured_worker_without_embedded_initialization() {
    // Cargo workspace qualification builds the dependency's actual binary first.
    let executable = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("mrr-native-worker");
    configure_native_worker(&executable).unwrap();
    let source = "MATCH (a:Person)-[:KNOWS]->(b:Person) FINISH";
    let receipt = QueryFrontend::new(ParserLanguage::Gql)
        .compile_with_receipt("source.gql", source)
        .unwrap()
        .receipt;
    let compiled = mrr_property_source::compile_property_source_query(
        "source.gql",
        source,
        &receipt.source_digest,
    )
    .unwrap();
    assert_eq!(compiled.compilation(), &receipt);
    assert!(
        mrr_property_source::compile_property_source_query("source.gql", source, "sha256:wrong")
            .is_err()
    );
    println!("CASE original source and grammar receipts preserved by isolated Parser");
    let factor = QueryOperatorId::from_canonical_bytes(b"factor:source").unwrap();
    let generation = GenerationId::from_canonical_bytes(b"generation:one").unwrap();
    let observation = SearchObservation::new(
        FactId::from_canonical_bytes(b"event:one").unwrap(),
        FactId::from_canonical_bytes(b"candidate:one").unwrap(),
        factor,
        generation,
        1,
        vec![],
    );
    let bound = NonZeroUsize::new(8).unwrap();
    let result = evaluate_search_factors(
        generation,
        &[SearchFactor::new(factor, SearchFactorRole::Acquisition)],
        &[],
        &[observation],
        SearchFrameworkLimits::new(bound, bound, bound, bound, bound),
    )
    .unwrap();
    assert_eq!(result.status(), SearchFrameworkStatus::Complete);
    assert_eq!(result.influences().len(), 1);
    println!("CASE original Search witness and lineage admitted through Scheme worker");
    TemporalHost
        .refresh_proof_state(include_bytes!(
            "../../../mrr-gerbil/tests/fixtures/temporal-state-v1.ss"
        ))
        .unwrap();
    println!("CASE Temporal state refresh shares the configured native owner");
    let mut child = std::process::Command::new("/bin/sh")
        .args(["-c", "exit 23"])
        .spawn()
        .unwrap();
    assert_eq!(child.wait().unwrap().code(), Some(23));
    shutdown_native_worker().unwrap();
    assert!(
        QueryFrontend::new(ParserLanguage::Gql)
            .compile_with_receipt("source.gql", source)
            .is_err()
    );
    println!("CASE shutdown refuses embedded fallback and preserves Host exit 23");
}
