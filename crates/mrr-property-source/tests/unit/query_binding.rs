use std::num::NonZeroUsize;

use meta_relational_reasoning::{
    CandidateQueryResult, EntityCatalog, EntityId, EntitySchema, ExternalRevisionIdentity,
    GenerationId, QueryResultBinding, QueryResultLimits, RelationCatalog, RelationField,
    RelationId, RelationSchema, RevisionBinding, SemanticSnapshot, ValueSchema,
};
use mrr_frontends::{ParserLanguage, QueryFrontend};

use crate::{PropertySourceQueryError, compile_property_source_query};

const SOURCE: &str = "MATCH (a:Person)-[:KNOWS]->(b:Person) FINISH";

fn entity_id() -> EntityId {
    EntityId::from_canonical_bytes(b"mrr.frontend.entity-type.v1\0Person").unwrap()
}

fn relation_id(name: &str) -> RelationId {
    RelationId::from_canonical_bytes(format!("mrr.frontend.relation-type.v1\0{name}")).unwrap()
}

fn catalogs(relation_name: &str) -> (RelationCatalog, EntityCatalog) {
    let relation = RelationSchema::new(
        relation_id(relation_name),
        relation_name,
        vec![
            RelationField::new("subject", ValueSchema::Entity, false).unwrap(),
            RelationField::new("object", ValueSchema::Entity, false).unwrap(),
        ],
        Vec::new(),
    )
    .unwrap();
    (
        RelationCatalog::admit(vec![relation]).unwrap(),
        EntityCatalog::admit(vec![
            EntitySchema::new(entity_id(), "Person", Vec::new()).unwrap(),
        ])
        .unwrap(),
    )
}

fn snapshot() -> SemanticSnapshot {
    let generation = GenerationId::from_canonical_bytes(b"property-query-binding-test").unwrap();
    let revision = RevisionBinding::admit(
        ExternalRevisionIdentity::new("git", "repository", "revision").unwrap(),
        generation,
    )
    .unwrap();
    SemanticSnapshot::admit(generation, vec![revision]).unwrap()
}

fn compiled() -> crate::CompiledPropertySourceQuery {
    let digest = QueryFrontend::new(ParserLanguage::Gql)
        .compile_with_receipt("source.gql", SOURCE)
        .unwrap()
        .receipt
        .source_digest;
    compile_property_source_query("source.gql", SOURCE, &digest).unwrap()
}

fn limits() -> QueryResultLimits {
    QueryResultLimits::new(NonZeroUsize::new(1).unwrap(), NonZeroUsize::new(1).unwrap())
}

#[test]
fn caller_can_bind_source_then_admit_a_result() {
    let (relations, entities) = catalogs("KNOWS");
    let bound = compiled().bind(&relations, &entities, &snapshot()).unwrap();
    let candidate = CandidateQueryResult::new(
        QueryResultBinding::for_query(bound.query()),
        Vec::new(),
        Vec::new(),
    );
    let admission = bound.admit(&candidate, limits()).unwrap();
    assert_eq!(bound.compilation().source_name, "source.gql");
    assert_eq!(admission.row_count(), 0);
}

#[test]
fn source_digest_and_catalog_binding_reject_drift() {
    assert!(matches!(
        compile_property_source_query("source.gql", SOURCE, "sha256:wrong"),
        Err(PropertySourceQueryError::SourceDigestMismatch)
    ));
    let (relations, entities) = catalogs("OTHER");
    assert!(compiled().bind(&relations, &entities, &snapshot()).is_err());
}

#[test]
fn candidate_for_a_different_binding_cannot_be_admitted() {
    let (relations, entities) = catalogs("KNOWS");
    let bound = compiled().bind(&relations, &entities, &snapshot()).unwrap();
    let binding = QueryResultBinding::for_query(bound.query());
    let wrong = QueryResultBinding::new(
        [0; 32],
        binding.generation(),
        binding.relation_catalog_digest(),
        binding.entity_catalog_digest(),
        *binding.snapshot_digest(),
    );
    let candidate = CandidateQueryResult::new(wrong, Vec::new(), Vec::new());
    assert!(matches!(
        bound.admit(&candidate, limits()),
        Err(PropertySourceQueryError::Admission(_))
    ));
}

struct Physical {
    substitute_binding: bool,
}
impl meta_relational_reasoning::PropertyQueryBackend for Physical {
    type PhysicalEvidence = meta_relational_reasoning::CatalogBoundQuery;
    type Error = std::io::Error;

    async fn execute<'a>(
        &'a self,
        query: &'a meta_relational_reasoning::CatalogBoundQuery,
    ) -> Result<
        meta_relational_reasoning::PropertyExecutionCandidate<Self::PhysicalEvidence>,
        Self::Error,
    > {
        let binding = QueryResultBinding::for_query(query);
        let binding = if self.substitute_binding {
            QueryResultBinding::new(
                [0; 32],
                binding.generation(),
                binding.relation_catalog_digest(),
                binding.entity_catalog_digest(),
                *binding.snapshot_digest(),
            )
        } else {
            binding
        };
        Ok(meta_relational_reasoning::PropertyExecutionCandidate {
            candidate: CandidateQueryResult::new(binding, Vec::new(), Vec::new()),
            physical_evidence: query.clone(),
        })
    }
}

// This adapter completes synchronously; pending means the fixture is broken.
fn ready<T>(future: impl std::future::Future<Output = T>) -> T {
    let mut future = std::pin::pin!(future);
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    match future.as_mut().poll(&mut context) {
        std::task::Poll::Ready(value) => value,
        std::task::Poll::Pending => panic!("fixture must complete without I/O"),
    }
}

#[test]
fn source_execution_retains_original_receipts_after_request_drop() {
    let (relations, entities) = catalogs("KNOWS");
    let bound = compiled().bind(&relations, &entities, &snapshot()).unwrap();
    let original = bound.compilation().clone();
    let query = bound.query().clone();
    let output = ready(bound.execute_with(
        &Physical {
            substitute_binding: false,
        },
        limits(),
    ))
    .unwrap();
    drop(bound);
    assert_eq!(output.compilation(), &original);
    assert_eq!(output.query(), &query);
    assert_eq!(output.physical_evidence(), &query);
    let bytes = output
        .export_result_transport(NonZeroUsize::new(16384).unwrap())
        .unwrap();
    let verified = meta_relational_reasoning::verify_query_result_transport(
        &query,
        &bytes,
        limits(),
        NonZeroUsize::new(16384).unwrap(),
    )
    .unwrap();
    assert_eq!(verified.receipt(), output.receipt());
    assert_eq!(verified.candidate(), output.candidate());
    assert!(
        output
            .export_result_transport(NonZeroUsize::new(1).unwrap())
            .is_err()
    );
    println!("CASE original parser/query/physical evidence retained and Scheme v1 re-admitted");
}

#[test]
fn source_execution_rejects_substituted_result_before_retention() {
    let (relations, entities) = catalogs("KNOWS");
    let bound = compiled().bind(&relations, &entities, &snapshot()).unwrap();
    assert!(matches!(
        ready(bound.execute_with(
            &Physical {
                substitute_binding: true
            },
            limits()
        )),
        Err(meta_relational_reasoning::PropertyExecutionError::Admission(_))
    ));
}
