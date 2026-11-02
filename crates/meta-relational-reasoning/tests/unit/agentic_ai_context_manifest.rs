use super::agentic_ai_context::{
    admit, contract, limits, presentation_request, snapshot, source, source_with_text,
};
use crate::{
    AgenticAiContextAdmissionError, AgenticAiContextAdmissionRequest,
    AgenticAiContextManifestRecord, AgenticAiContextRestoreRequest, EvidenceCompleteness,
    admit_agentic_ai_context, bind_query_to_catalog, restore_agentic_ai_context,
};

#[test]
fn manifest_canonicalizes_declarations_but_distinguishes_the_dependency_graph() {
    let (bundle, snapshot, query_id, fact) = source();
    let query = bind_query_to_catalog(&bundle, query_id, &snapshot).unwrap();
    let build = |duplicate: bool, cyclic: bool| {
        let mut contract = contract();
        contract.required = if duplicate {
            vec![fact, fact]
        } else {
            vec![fact]
        };
        admit_agentic_ai_context(
            &bundle,
            &snapshot,
            &query,
            AgenticAiContextAdmissionRequest {
                contract,
                roots: if duplicate {
                    vec![fact, fact]
                } else {
                    vec![fact]
                },
                dependencies: if cyclic {
                    vec![(
                        fact,
                        if duplicate {
                            vec![fact, fact]
                        } else {
                            vec![fact]
                        },
                    )]
                } else {
                    vec![]
                },
                limits: limits(),
            },
        )
        .unwrap()
    };
    let canonical = build(false, true);
    let duplicates = build(true, true);
    let independent = build(false, false);
    assert_eq!(canonical.manifest(), duplicates.manifest());
    assert_eq!(canonical.closure(), independent.closure());
    assert_ne!(
        canonical.manifest().digest(),
        independent.manifest().digest()
    );
}

#[test]
fn manifest_binds_source_revision_contract_and_limits_but_not_renderer_output() {
    let (bundle, source_snapshot, query_id, fact) = source();
    let old = admit(&bundle, &source_snapshot, query_id, fact);
    let revised = snapshot(source_snapshot.generation(), "two");
    assert_ne!(
        old.manifest().digest(),
        admit(&bundle, &revised, query_id, fact).manifest().digest()
    );
    let (replacement, _, _, _) = source_with_text("changed payload with the same fact ID");
    assert_ne!(
        old.manifest().digest(),
        admit(&replacement, &source_snapshot, query_id, fact)
            .manifest()
            .digest()
    );
    let query = bind_query_to_catalog(&bundle, query_id, &source_snapshot).unwrap();
    let mut changed_contract = contract();
    changed_contract.policy_digest = [8; 32];
    let changed = admit_agentic_ai_context(
        &bundle,
        &source_snapshot,
        &query,
        AgenticAiContextAdmissionRequest {
            contract: changed_contract,
            roots: vec![fact],
            dependencies: vec![],
            limits: limits(),
        },
    )
    .unwrap();
    assert_ne!(old.manifest().digest(), changed.manifest().digest());
    let mut changed_limits = limits();
    changed_limits.max_rendered_bytes = std::num::NonZeroUsize::new(32).unwrap();
    let changed = admit_agentic_ai_context(
        &bundle,
        &source_snapshot,
        &query,
        AgenticAiContextAdmissionRequest {
            contract: contract(),
            roots: vec![fact],
            dependencies: vec![],
            limits: changed_limits,
        },
    )
    .unwrap();
    assert_ne!(old.manifest().digest(), changed.manifest().digest());
    let first = old
        .materialize(
            &bundle,
            &source_snapshot,
            presentation_request(fact, &[1, 2]),
        )
        .unwrap();
    let second = old
        .materialize(
            &bundle,
            &source_snapshot,
            presentation_request(fact, &[9, 8]),
        )
        .unwrap();
    assert_eq!(first.manifest(), old.manifest());
    assert_eq!(second.manifest(), old.manifest());
    assert_ne!(
        first.materialization().bytes(),
        second.materialization().bytes()
    );
}

#[test]
fn stored_manifest_round_trips_only_through_current_source_admission() {
    let (bundle, snapshot, query, fact) = source();
    let admitted = admit(&bundle, &snapshot, query, fact);
    let wire = serde_json::to_vec(admitted.manifest().record()).unwrap();
    let record: AgenticAiContextManifestRecord = serde_json::from_slice(&wire).unwrap();
    let restored = restore_agentic_ai_context(
        &bundle,
        &snapshot,
        AgenticAiContextRestoreRequest {
            record: &record,
            expected_digest: *admitted.manifest().digest(),
            limits: limits(),
        },
    )
    .unwrap();
    assert_eq!(restored, admitted);
    let mut cbor = Vec::new();
    ciborium::into_writer(&record, &mut cbor).unwrap();
    let record: AgenticAiContextManifestRecord = ciborium::from_reader(cbor.as_slice()).unwrap();
    assert_eq!(
        restore_agentic_ai_context(
            &bundle,
            &snapshot,
            AgenticAiContextRestoreRequest {
                record: &record,
                expected_digest: *admitted.manifest().digest(),
                limits: limits(),
            }
        )
        .unwrap(),
        admitted
    );
}

#[test]
fn restore_rejects_forged_closure_graph_contract_and_expected_identity() {
    let (bundle, snapshot, query, fact) = source();
    let admitted = admit(&bundle, &snapshot, query, fact);
    let original = admitted.manifest().record();
    let mut candidates = Vec::new();
    let mut record = original.clone();
    record.selected.clear();
    candidates.push(record);
    let mut record = original.clone();
    record.completeness = EvidenceCompleteness::Unknown;
    candidates.push(record);
    let mut record = original.clone();
    record.roots.push(fact);
    candidates.push(record);
    let mut record = original.clone();
    record.dependencies[0].1.push(fact);
    candidates.push(record);
    let mut record = original.clone();
    record.contract.policy_digest = [9; 32];
    candidates.push(record);
    for record in candidates {
        assert_eq!(
            restore_agentic_ai_context(
                &bundle,
                &snapshot,
                AgenticAiContextRestoreRequest {
                    record: &record,
                    expected_digest: *admitted.manifest().digest(),
                    limits: limits(),
                }
            ),
            Err(AgenticAiContextAdmissionError::ManifestMismatch)
        );
    }
    assert_eq!(
        restore_agentic_ai_context(
            &bundle,
            &snapshot,
            AgenticAiContextRestoreRequest {
                record: original,
                expected_digest: [0; 32],
                limits: limits(),
            }
        ),
        Err(AgenticAiContextAdmissionError::ManifestMismatch)
    );
}

#[test]
fn restore_rejects_schema_and_resource_inflation_before_reconstruction() {
    let (bundle, snapshot, query, fact) = source();
    let admitted = admit(&bundle, &snapshot, query, fact);
    let mut record = admitted.manifest().record().clone();
    record.schema = "future-schema".into();
    assert_eq!(
        restore_agentic_ai_context(
            &bundle,
            &snapshot,
            AgenticAiContextRestoreRequest {
                record: &record,
                expected_digest: *admitted.manifest().digest(),
                limits: limits(),
            }
        ),
        Err(AgenticAiContextAdmissionError::ManifestSchemaMismatch)
    );
    let mut record = admitted.manifest().record().clone();
    record.dependencies[0].1 = vec![fact; limits().max_dependency_edges.get() + 1];
    assert_eq!(
        restore_agentic_ai_context(
            &bundle,
            &snapshot,
            AgenticAiContextRestoreRequest {
                record: &record,
                expected_digest: *admitted.manifest().digest(),
                limits: limits(),
            }
        ),
        Err(AgenticAiContextAdmissionError::ManifestBudget)
    );
    let mut ceiling = limits();
    ceiling.max_elements = std::num::NonZeroUsize::new(1).unwrap();
    assert_eq!(
        restore_agentic_ai_context(
            &bundle,
            &snapshot,
            AgenticAiContextRestoreRequest {
                record: admitted.manifest().record(),
                expected_digest: *admitted.manifest().digest(),
                limits: ceiling,
            }
        ),
        Err(AgenticAiContextAdmissionError::ManifestBudget)
    );
}

#[test]
fn restore_rejects_current_source_replacement_and_same_generation_revision_drift() {
    let (bundle, source_snapshot, query, fact) = source();
    let admitted = admit(&bundle, &source_snapshot, query, fact);
    let revised = snapshot(source_snapshot.generation(), "two");
    let (replacement, _, _, _) = source_with_text("changed source evidence");
    assert_eq!(
        restore_agentic_ai_context(
            &replacement,
            &source_snapshot,
            AgenticAiContextRestoreRequest {
                record: admitted.manifest().record(),
                expected_digest: *admitted.manifest().digest(),
                limits: limits(),
            }
        ),
        Err(AgenticAiContextAdmissionError::SourceBundleMismatch)
    );
    assert_eq!(
        restore_agentic_ai_context(
            &bundle,
            &revised,
            AgenticAiContextRestoreRequest {
                record: admitted.manifest().record(),
                expected_digest: *admitted.manifest().digest(),
                limits: limits(),
            }
        ),
        Err(AgenticAiContextAdmissionError::QueryBindingMismatch)
    );
}
