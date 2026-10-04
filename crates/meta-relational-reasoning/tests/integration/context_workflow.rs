#[path = "../../examples/support/mod.rs"]
pub mod support;

use meta_relational_reasoning::{
    AgenticAiContextAdmissionError, AgenticAiContextQuerySelectionRecord,
    AgenticAiContextQuerySelectionRestoreRequest, CandidateQueryResult, GenerationId,
    QueryResultBinding, QueryResultTransportError, RelationCatalog, bind_query_to_catalog,
    restore_agentic_ai_context_query_selection, select_agentic_ai_context_from_query,
};
use std::collections::BTreeSet;

#[test]
fn shared_selection_vectors_match_real_facade_admission() {
    let fixture: support::SelectionFixture = serde_json::from_str(support::FIXTURE).unwrap();
    assert_eq!(
        fixture.schema,
        "mrr.agentic-ai-context.selection-fixture.v1"
    );
    assert_eq!(fixture.current_binding, [1, 1, 1, 1]);
    let source = support::source(&fixture.facts);
    for case in fixture.cases {
        let normal = support::candidate(&source, &case.rows);
        let binding = QueryResultBinding::new(
            if case.binding[0] == 1 {
                *source.query.digest()
            } else {
                [0; 32]
            },
            if case.binding[1] == 1 {
                source.query.generation()
            } else {
                GenerationId::from_canonical_bytes("foreign-generation").unwrap()
            },
            if case.binding[2] == 1 {
                source.query.catalog_digest()
            } else {
                RelationCatalog::admit(vec![support::schema(8)])
                    .unwrap()
                    .digest()
            },
            source.query.entity_catalog_digest(),
            if case.binding[3] == 1 {
                *source.query.snapshot_digest()
            } else {
                [0; 32]
            },
        );
        let candidate =
            CandidateQueryResult::new(binding, normal.columns().to_vec(), normal.rows().to_vec());
        let result = select_agentic_ai_context_from_query(
            &source.bundle,
            &source.snapshot,
            &source.query,
            &candidate,
            support::request(case.max_facts, case.max_rows, &[]),
        );
        assert_eq!(result.is_ok(), case.accepted, "{}", case.name);
        if let Ok(selected) = result {
            let roots: BTreeSet<_> = selected
                .context()
                .state()
                .query()
                .roots()
                .iter()
                .copied()
                .collect();
            assert_eq!(
                roots,
                case.roots.iter().map(|id| support::fact_id(*id)).collect(),
                "{}",
                case.name
            );
        }
        println!("SHARED-SELECTION-OK: {}", case.name);
    }
}

#[test]
fn full_workflow_restores_and_compares_the_same_contract_chain() {
    let receipt = support::workflow().unwrap();
    assert_eq!((receipt.old_selected, receipt.new_selected), (2, 3));
    assert!(receipt.restored);
    assert_eq!(receipt.semantic_reusable, 0);
    assert_eq!(receipt.eligible_full_block_tokens, 2);
}

#[test]
fn selection_restore_rejects_tampering_rebinding_and_resource_inflation() {
    let fixture: support::SelectionFixture = serde_json::from_str(support::FIXTURE).unwrap();
    let source = support::source(&fixture.facts);
    let rows = support::candidate(&source, &fixture.workflow.old_rows);
    let limits = support::request(8, 8, &fixture.workflow.dependencies);
    let selected = select_agentic_ai_context_from_query(
        &source.bundle,
        &source.snapshot,
        &source.query,
        &rows,
        limits.clone(),
    )
    .unwrap();
    let record = selected
        .export_record(
            &source.query,
            &rows,
            limits.result_limits,
            support::nz(65536),
        )
        .unwrap();
    let decoded: AgenticAiContextQuerySelectionRecord =
        serde_json::from_slice(&serde_json::to_vec(&record).unwrap()).unwrap();
    let restore = |record: &AgenticAiContextQuerySelectionRecord, expected| {
        restore_agentic_ai_context_query_selection(
            &source.bundle,
            &source.snapshot,
            AgenticAiContextQuerySelectionRestoreRequest {
                record,
                expected_digest: expected,
                limits: limits.limits,
                result_limits: limits.result_limits,
                max_result_bytes: support::nz(65536),
            },
        )
    };
    assert_eq!(restore(&decoded, *selected.digest()).unwrap(), selected);
    assert_eq!(
        restore(&record, [0; 32]).unwrap_err(),
        AgenticAiContextAdmissionError::SelectionRecordMismatch
    );
    let mut changed = record.clone();
    changed.schema.push_str("-foreign");
    assert_eq!(
        restore(&changed, *selected.digest()).unwrap_err(),
        AgenticAiContextAdmissionError::SelectionRecordSchema
    );
    let mut changed = record.clone();
    changed.context.roots.clear();
    assert!(restore(&changed, *selected.digest()).is_err());
    let mut changed = record.clone();
    changed.context.limits.max_elements = support::nz(9);
    assert_eq!(
        restore(&changed, *selected.digest()).unwrap_err(),
        AgenticAiContextAdmissionError::ManifestBudget
    );
    let mut changed = record.clone();
    changed.result_transport.push(b'!');
    assert!(matches!(
        restore(&changed, *selected.digest()),
        Err(AgenticAiContextAdmissionError::SelectionTransport(
            QueryResultTransportError::Encoding(_)
        ))
    ));
    let mut changed = record.clone();
    changed.result_transport.resize(65537, 0);
    assert!(matches!(
        restore(&changed, *selected.digest()),
        Err(AgenticAiContextAdmissionError::SelectionTransport(
            QueryResultTransportError::TooLarge { .. }
        ))
    ));
    // Replace rows with a separately valid transport: hashes inside storage are not trust.
    let other_rows = support::candidate(&source, &fixture.workflow.new_rows);
    let other = select_agentic_ai_context_from_query(
        &source.bundle,
        &source.snapshot,
        &source.query,
        &other_rows,
        limits.clone(),
    )
    .unwrap();
    let other_record = other
        .export_record(
            &source.query,
            &other_rows,
            limits.result_limits,
            support::nz(65536),
        )
        .unwrap();
    assert!(restore(&other_record, *selected.digest()).is_err());
    assert!(
        selected
            .export_record(
                &source.query,
                &other_rows,
                limits.result_limits,
                support::nz(65536)
            )
            .is_err()
    );
    let mut duplicate_rows = fixture.workflow.old_rows.clone();
    duplicate_rows.extend_from_slice(&fixture.workflow.old_rows);
    let duplicates = support::candidate(&source, &duplicate_rows);
    let duplicates_selected = select_agentic_ai_context_from_query(
        &source.bundle,
        &source.snapshot,
        &source.query,
        &duplicates,
        limits.clone(),
    )
    .unwrap();
    let duplicates_record = duplicates_selected
        .export_record(
            &source.query,
            &duplicates,
            limits.result_limits,
            support::nz(65536),
        )
        .unwrap();
    assert_eq!(
        duplicates_selected.context().manifest(),
        selected.context().manifest()
    );
    assert_eq!(
        restore(&duplicates_record, *selected.digest()).unwrap_err(),
        AgenticAiContextAdmissionError::SelectionRecordMismatch
    );
    assert!(matches!(
        restore_agentic_ai_context_query_selection(
            &source.bundle,
            &source.snapshot,
            AgenticAiContextQuerySelectionRestoreRequest {
                record: &duplicates_record,
                expected_digest: *duplicates_selected.digest(),
                limits: limits.limits,
                result_limits: meta_relational_reasoning::QueryResultLimits::new(
                    support::nz(1),
                    support::nz(1)
                ),
                max_result_bytes: support::nz(65536)
            }
        ),
        Err(AgenticAiContextAdmissionError::SelectionTransport(
            QueryResultTransportError::Admission(_)
        ))
    ));
    let mut changed = record.clone();
    changed.context_digest = [0; 32];
    assert_eq!(
        restore(&changed, *selected.digest()).unwrap_err(),
        AgenticAiContextAdmissionError::ManifestMismatch
    );
    let mut changed = record.clone();
    changed.column = meta_relational_reasoning::Binding::new("absent").unwrap();
    assert!(matches!(
        restore(&changed, *selected.digest()),
        Err(AgenticAiContextAdmissionError::SelectionColumn(_))
    ));
    let revised = support::snapshot(source.snapshot.generation(), "two");
    assert!(
        restore_agentic_ai_context_query_selection(
            &source.bundle,
            &revised,
            AgenticAiContextQuerySelectionRestoreRequest {
                record: &record,
                expected_digest: *selected.digest(),
                limits: limits.limits,
                result_limits: limits.result_limits,
                max_result_bytes: support::nz(65536),
            }
        )
        .is_err()
    );
    let foreign = support::source(&[(1, 7), (2, 7)]);
    assert!(
        restore_agentic_ai_context_query_selection(
            &foreign.bundle,
            &foreign.snapshot,
            AgenticAiContextQuerySelectionRestoreRequest {
                record: &record,
                expected_digest: *selected.digest(),
                limits: limits.limits,
                result_limits: limits.result_limits,
                max_result_bytes: support::nz(65536),
            }
        )
        .is_err()
    );
    let rebound =
        bind_query_to_catalog(&source.bundle, source.query.query().id(), &revised).unwrap();
    assert!(
        selected
            .export_record(&rebound, &rows, limits.result_limits, support::nz(65536))
            .is_err()
    );
}
