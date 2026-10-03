use super::agentic_ai_context::{contract, limits, snapshot};
use crate::{
    AgenticAiContextAdmissionError, AgenticAiContextQuerySelectionRequest,
    AgenticAiContextRevisionRequest, Binding, CandidateQueryResult, CatalogBoundQuery, Direction,
    EntityId, EvidenceCompleteness, Expression, Fact, FactId, FactProvenance, FactValidity,
    GenerationId, GraphPattern, MetaQueryIr, NodePattern, PathPattern, PathSegment, Projection,
    QueryId, QueryOperatorId, QueryResult, QueryResultAdmissionError, QueryResultBinding,
    QueryResultLimits, QueryResultValue, QueryTemplate, ReasoningBundle,
    ReasoningBundleDeclaration, RelationAuthority, RelationContext, RelationField, RelationId,
    RelationPattern, RelationSchema, SemanticSnapshot, SetQuantifier, Value, ValueSchema,
    bind_query_to_catalog, compare_agentic_ai_context_revision,
    select_agentic_ai_context_from_query,
};
use std::num::NonZeroUsize;

fn fixture() -> (
    ReasoningBundle,
    SemanticSnapshot,
    CatalogBoundQuery,
    FactId,
    RelationId,
) {
    let generation = GenerationId::from_canonical_bytes("selection-generation").unwrap();
    let relation = RelationId::from_canonical_bytes("selection-relation").unwrap();
    let entity = EntityId::from_canonical_bytes("selection-entity").unwrap();
    let fact = FactId::from_canonical_bytes("selection-fact").unwrap();
    let query_id = QueryId::from_canonical_bytes("selection-query").unwrap();
    let binding = |s| Binding::new(s).unwrap();
    let query = MetaQueryIr::new(
        query_id,
        GraphPattern::new(
            QueryOperatorId::from_canonical_bytes("selection-graph").unwrap(),
            vec![PathPattern::new(
                NodePattern::new(binding("source"), vec![]),
                vec![PathSegment::new(
                    RelationPattern::new(
                        Some(binding("edge")),
                        vec![relation],
                        Direction::Outgoing,
                        1,
                        Some(1),
                    )
                    .unwrap(),
                    NodePattern::new(binding("target"), vec![]),
                )],
            )],
        )
        .unwrap(),
        vec![],
        QueryResult::returning(SetQuantifier::All).with_projections(vec![Projection::new(
            QueryOperatorId::from_canonical_bytes("selection-projection").unwrap(),
            Expression::Binding(binding("edge")),
            binding("evidence"),
        )]),
    )
    .unwrap();
    let bundle = ReasoningBundle::admit(ReasoningBundleDeclaration {
        relations: vec![
            RelationSchema::new(
                relation,
                "depends",
                vec![
                    RelationField::new("subject", ValueSchema::Entity, false).unwrap(),
                    RelationField::new("object", ValueSchema::Entity, false).unwrap(),
                ],
                vec![],
            )
            .unwrap(),
        ],
        facts: vec![Fact::new(
            fact,
            relation,
            vec![Value::Entity(entity), Value::Entity(entity)],
            RelationContext::new(
                generation,
                RelationAuthority::Entity(entity),
                FactProvenance::Source(entity),
                EvidenceCompleteness::Complete,
                FactValidity::Valid,
            )
            .unwrap(),
        )],
        query_templates: vec![QueryTemplate::new(query, vec![])],
        ..ReasoningBundleDeclaration::default()
    })
    .unwrap();
    let snapshot = snapshot(generation, "selection-one");
    let bound = bind_query_to_catalog(&bundle, query_id, &snapshot).unwrap();
    (bundle, snapshot, bound, fact, relation)
}
fn request() -> AgenticAiContextQuerySelectionRequest {
    AgenticAiContextQuerySelectionRequest {
        column: Binding::new("evidence").unwrap(),
        contract: contract(),
        dependencies: vec![],
        limits: limits(),
        result_limits: QueryResultLimits::new(
            NonZeroUsize::new(8).unwrap(),
            NonZeroUsize::new(8).unwrap(),
        ),
    }
}
fn candidate(
    query: &CatalogBoundQuery,
    ids: &[FactId],
    relation: RelationId,
) -> CandidateQueryResult {
    CandidateQueryResult::new(
        QueryResultBinding::for_query(query),
        vec![Binding::new("evidence").unwrap()],
        ids.iter()
            .map(|id| vec![QueryResultValue::relation(*id, relation)])
            .collect(),
    )
}
#[test]
fn query_selection_deduplicates_roots_but_binds_exact_result_rows() {
    let (bundle, snapshot, query, fact, relation) = fixture();
    let one = select_agentic_ai_context_from_query(
        &bundle,
        &snapshot,
        &query,
        &candidate(&query, &[fact], relation),
        request(),
    )
    .unwrap();
    let two = select_agentic_ai_context_from_query(
        &bundle,
        &snapshot,
        &query,
        &candidate(&query, &[fact, fact], relation),
        request(),
    )
    .unwrap();
    assert_eq!(two.context().state().query().roots(), &[fact]);
    assert_eq!(one.context().manifest(), two.context().manifest());
    assert_ne!(one.digest(), two.digest());
    assert_eq!(two.result_receipt().row_count(), 2);
    let revision = compare_agentic_ai_context_revision(AgenticAiContextRevisionRequest {
        old: one.context(),
        new: two.context(),
        old_bundle: &bundle,
        new_bundle: &bundle,
        old_snapshot: &snapshot,
        new_snapshot: &snapshot,
    })
    .unwrap();
    assert_eq!(revision.revision().reusable().elements(), &[fact]);
}
#[test]
fn query_selection_refuses_unknown_facts_wrong_relation_and_budgets() {
    let (bundle, snapshot, query, fact, relation) = fixture();
    let unknown = FactId::from_canonical_bytes("absent-selection-fact").unwrap();
    assert_eq!(
        select_agentic_ai_context_from_query(
            &bundle,
            &snapshot,
            &query,
            &candidate(&query, &[unknown], relation),
            request()
        )
        .unwrap_err(),
        AgenticAiContextAdmissionError::SelectionFactMismatch(unknown)
    );
    let other = RelationId::from_canonical_bytes("other-selection-relation").unwrap();
    assert!(matches!(
        select_agentic_ai_context_from_query(
            &bundle,
            &snapshot,
            &query,
            &candidate(&query, &[fact], other),
            request()
        ),
        Err(AgenticAiContextAdmissionError::QueryResult(_))
    ));
    let mut req = request();
    req.result_limits =
        QueryResultLimits::new(NonZeroUsize::new(1).unwrap(), NonZeroUsize::new(1).unwrap());
    assert!(matches!(
        select_agentic_ai_context_from_query(
            &bundle,
            &snapshot,
            &query,
            &candidate(&query, &[fact, fact], relation),
            req
        ),
        Err(AgenticAiContextAdmissionError::QueryResult(
            QueryResultAdmissionError::RowLimitExceeded { .. }
        ))
    ));
}
#[test]
fn query_selection_rejects_source_snapshot_and_result_binding_drift() {
    let (bundle, old, query, fact, relation) = fixture();
    let rows = candidate(&query, &[fact], relation);
    let selected =
        select_agentic_ai_context_from_query(&bundle, &old, &query, &rows, request()).unwrap();
    let new = snapshot(old.generation(), "selection-two");
    assert_eq!(
        selected.check_source(&bundle, &new),
        Err(AgenticAiContextAdmissionError::QueryBindingMismatch)
    );
    assert_eq!(
        select_agentic_ai_context_from_query(&bundle, &new, &query, &rows, request()).unwrap_err(),
        AgenticAiContextAdmissionError::QueryBindingMismatch
    );
    let rebound = bind_query_to_catalog(&bundle, query.query().id(), &new).unwrap();
    assert!(matches!(
        select_agentic_ai_context_from_query(&bundle, &new, &rebound, &rows, request()),
        Err(AgenticAiContextAdmissionError::QueryResult(
            QueryResultAdmissionError::QueryBindingMismatch
        ))
    ));
    let fresh = select_agentic_ai_context_from_query(
        &bundle,
        &new,
        &rebound,
        &candidate(&rebound, &[fact], relation),
        request(),
    )
    .unwrap();
    let revision = compare_agentic_ai_context_revision(AgenticAiContextRevisionRequest {
        old: selected.context(),
        new: fresh.context(),
        old_bundle: &bundle,
        new_bundle: &bundle,
        old_snapshot: &old,
        new_snapshot: &new,
    })
    .unwrap();
    assert!(revision.query_binding_changed());
    assert!(revision.revision().reusable().elements().is_empty());
}
#[test]
fn query_selection_empty_result_and_missing_column_are_explicit() {
    let (bundle, snapshot, query, _, relation) = fixture();
    let rows = candidate(&query, &[], relation);
    let selected =
        select_agentic_ai_context_from_query(&bundle, &snapshot, &query, &rows, request()).unwrap();
    assert!(selected.context().closure().elements().is_empty());
    let mut req = request();
    req.column = Binding::new("absent").unwrap();
    assert!(matches!(
        select_agentic_ai_context_from_query(&bundle, &snapshot, &query, &rows, req),
        Err(AgenticAiContextAdmissionError::SelectionColumn(_))
    ));
}
