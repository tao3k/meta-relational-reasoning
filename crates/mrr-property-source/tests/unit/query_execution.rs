use std::{
    cell::Cell,
    future::Future,
    num::NonZeroUsize,
    pin::pin,
    task::{Context, Poll, Waker},
};

use meta_relational_reasoning::{
    CandidateQueryResult, CatalogBoundQuery, EntityCatalog, EntityId, EntitySchema,
    ExternalRevisionIdentity, GenerationId, QueryResultBinding, QueryResultLimits, RelationCatalog,
    RelationField, RelationId, RelationSchema, RevisionBinding, SemanticSnapshot, ValueSchema,
};
use mrr_frontends::{ParserLanguage, QueryFrontend};

use crate::{PropertyQueryExecutor, PropertySourceExecutionError, compile_property_source_query};

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
    let generation = GenerationId::from_canonical_bytes(b"property-query-execution-test").unwrap();
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

fn ready<T>(future: impl Future<Output = T>) -> T {
    let mut future = pin!(future);
    match future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(result) => result,
        Poll::Pending => panic!("test executor must complete without suspension"),
    }
}

struct Executor {
    calls: Cell<usize>,
    fail: bool,
}

impl PropertyQueryExecutor for Executor {
    type Error = &'static str;

    fn execute<'a>(
        &'a self,
        query: &'a CatalogBoundQuery,
    ) -> impl Future<Output = Result<CandidateQueryResult, Self::Error>> + 'a {
        async move {
            self.calls.set(self.calls.get() + 1);
            if self.fail {
                return Err("backend failed");
            }
            Ok(CandidateQueryResult::new(
                QueryResultBinding::for_query(query),
                Vec::new(),
                Vec::new(),
            ))
        }
    }
}

#[test]
fn mrr_owns_bind_execute_and_admit_sequence() {
    let (relations, entities) = catalogs("KNOWS");
    let executor = Executor {
        calls: Cell::new(0),
        fail: false,
    };
    let result =
        ready(compiled().execute_with(&relations, &entities, &snapshot(), &executor, limits()))
            .unwrap();
    assert_eq!(executor.calls.get(), 1);
    assert_eq!(result.compilation.source_name, "source.gql");
    assert!(result.candidate.rows().is_empty());
}

#[test]
fn semantic_binding_rejects_before_physical_execution() {
    let (relations, entities) = catalogs("OTHER");
    let executor = Executor {
        calls: Cell::new(0),
        fail: false,
    };
    let result =
        ready(compiled().execute_with(&relations, &entities, &snapshot(), &executor, limits()));
    assert!(matches!(
        result,
        Err(PropertySourceExecutionError::Semantic(_))
    ));
    assert_eq!(executor.calls.get(), 0);
}

#[test]
fn physical_failure_never_yields_admitted_evidence() {
    let (relations, entities) = catalogs("KNOWS");
    let executor = Executor {
        calls: Cell::new(0),
        fail: true,
    };
    let result =
        ready(compiled().execute_with(&relations, &entities, &snapshot(), &executor, limits()));
    assert!(matches!(
        result,
        Err(PropertySourceExecutionError::Physical("backend failed"))
    ));
    assert_eq!(executor.calls.get(), 1);
}
