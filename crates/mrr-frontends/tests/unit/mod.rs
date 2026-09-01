use crate::{FrontendError, QueryFrontend, QueryLanguage};
use mrr_query::{BinaryOperator, Direction, Expression, Value};

const PARITY_QUERY: &str =
    "MATCH (a:Module)-[:DEPENDS_ON]->(b:Module) WHERE a.name = 'runtime' RETURN b";

#[test]
fn gql_and_cypher_lower_to_the_same_meta_query_ir() {
    let gql = QueryFrontend::new(QueryLanguage::Gql)
        .compile("parity.gql", PARITY_QUERY)
        .expect("GQL parity slice");
    let cypher = QueryFrontend::new(QueryLanguage::Cypher)
        .compile("parity.cypher", PARITY_QUERY)
        .expect("Cypher parity slice");

    assert_eq!(gql, cypher);
    let path = &gql.graph().paths()[0];
    assert_eq!(path.start().binding().as_str(), "a");
    assert_eq!(
        path.segments()[0].relation().direction(),
        Direction::Outgoing
    );
    assert_eq!(path.segments()[0].node().binding().as_str(), "b");
    assert_eq!(gql.projections().len(), 1);
    assert_eq!(gql.filters().len(), 1);
    assert!(matches!(
        gql.filters()[0].predicate(),
        Expression::Binary {
            operator: BinaryOperator::Equal,
            right,
            ..
        } if right.as_ref() == &Expression::Literal(Value::String("runtime".into()))
    ));
}

#[test]
fn parity_result_has_identical_canonical_bytes() {
    let gql = QueryFrontend::new(QueryLanguage::Gql)
        .compile("query.gql", PARITY_QUERY)
        .expect("GQL query");
    let cypher = QueryFrontend::new(QueryLanguage::Cypher)
        .compile("query.cypher", PARITY_QUERY)
        .expect("Cypher query");
    assert_eq!(
        gql.encode_canonical().expect("GQL canonical bytes"),
        cypher.encode_canonical().expect("Cypher canonical bytes")
    );
}

#[test]
fn unsupported_surface_fails_closed_before_meta_query_admission() {
    let error = QueryFrontend::new(QueryLanguage::Cypher)
        .compile("unsupported.cypher", "INSERT (a)")
        .expect_err("data mutation is outside the parity slice");
    assert!(matches!(
        error,
        FrontendError::Unsupported(_) | FrontendError::Diagnostics(_)
    ));
}
