use crate::{
    Aggregation, AggregationFunction, Atom, BinaryOperator, Binding, Direction, Expression, Filter,
    GraphPattern, MetaQueryIr, NodePattern, PathPattern, PathSegment, Projection, PropertyKey,
    QueryId, QueryIrError, QueryOperatorId, RelationId, RelationPattern, RelationalGoal,
    RelationalGoalError, SetQuantifier, Term, Value, Variable,
};
use mrr_identity::EntityId;

fn query_id(label: &str) -> QueryId {
    QueryId::from_canonical_bytes(format!("query:{label}")).expect("query id")
}

fn operator_id(label: &str) -> QueryOperatorId {
    QueryOperatorId::from_canonical_bytes(format!("operator:{label}")).expect("query operator id")
}

fn binding(name: &str) -> Binding {
    Binding::new(name).expect("binding")
}

fn fixture_query() -> MetaQueryIr {
    fixture_query_with_limit(Some(25))
}

fn fixture_query_with_limit(limit: Option<u64>) -> MetaQueryIr {
    let module = EntityId::from_canonical_bytes(b"type:module").expect("module type");
    let depends_on =
        RelationId::from_canonical_bytes(b"relation:depends-on").expect("relation type");
    let a = NodePattern::new(binding("a"), vec![module, module]);
    let b = NodePattern::new(binding("b"), vec![module]);
    let edge = RelationPattern::new(
        Some(binding("dependency")),
        vec![depends_on, depends_on],
        Direction::Outgoing,
        1,
        Some(1),
    )
    .expect("edge pattern");
    let graph = GraphPattern::new(
        operator_id("graph"),
        vec![PathPattern::new(a, vec![PathSegment::new(edge, b)])],
    )
    .expect("graph pattern");
    let filter = Filter::new(
        operator_id("filter"),
        Expression::Binary {
            left: Box::new(Expression::Property {
                binding: binding("a"),
                key: PropertyKey::new("name").expect("property key"),
            }),
            operator: BinaryOperator::Equal,
            right: Box::new(Expression::Literal(Value::String("runtime".into()))),
        },
    );
    let projection = Projection::new(
        operator_id("projection"),
        Expression::Binding(binding("b")),
        binding("dependency_target"),
    );
    MetaQueryIr::new(
        query_id("depends-on"),
        graph,
        vec![filter],
        vec![projection],
        Vec::new(),
        Vec::new(),
        limit,
    )
    .expect("meta query")
}

#[test]
fn relational_goal_outputs_must_be_bound_by_the_body() {
    let output = Variable::new("x").expect("variable");
    let body = vec![Atom {
        relation: RelationId::from_canonical_bytes(b"relation:query-test").expect("relation id"),
        terms: vec![Term::Value(Value::Integer(1))],
    }];
    assert_eq!(
        RelationalGoal::new(vec![output.clone()], body),
        Err(RelationalGoalError::UnboundOutput(output))
    );
}

#[test]
fn meta_query_normalizes_encodes_and_decodes_canonically() {
    let query = fixture_query().normalized();
    assert_eq!(query.graph().paths()[0].start().types().len(), 1);
    assert_eq!(
        query.graph().paths()[0].segments()[0]
            .relation()
            .types()
            .len(),
        1
    );

    let encoded = query.encode_canonical().expect("canonical encoding");
    assert!(encoded.starts_with(b"mrr.meta-query.v1\0"));
    assert_eq!(
        MetaQueryIr::decode_canonical(&encoded).expect("canonical decoding"),
        query
    );
    assert_eq!(query.encode_canonical().expect("repeat encoding"), encoded);
}

#[test]
fn zero_limit_round_trips_as_a_canonical_empty_result_bound() {
    let query = fixture_query_with_limit(Some(0)).normalized();
    assert_eq!(query.limit(), Some(0));
    let encoded = query.encode_canonical().expect("zero-limit encoding");
    assert_eq!(
        MetaQueryIr::decode_canonical(&encoded).expect("zero-limit decoding"),
        query
    );
}

#[test]
fn malformed_query_contracts_fail_closed() {
    assert_eq!(
        Binding::new(" bad"),
        Err(QueryIrError::InvalidName(" bad".into()))
    );
    assert_eq!(
        RelationPattern::new(None, Vec::new(), Direction::Outgoing, 2, Some(1)),
        Err(QueryIrError::InvalidHopRange {
            min: 2,
            max: Some(1),
        })
    );

    let query = fixture_query().normalized();
    let mut encoded = query.encode_canonical().expect("canonical encoding");
    encoded.extend_from_slice(b"trailing");
    assert_eq!(
        MetaQueryIr::decode_canonical(&encoded),
        Err(QueryIrError::TrailingBytes)
    );
    assert_eq!(
        MetaQueryIr::decode_canonical(b"wrong-schema"),
        Err(QueryIrError::SchemaMismatch)
    );

    let fixture = fixture_query();
    assert_eq!(
        MetaQueryIr::new(
            query_id("invalid-count-star-quantifier"),
            fixture.graph().clone(),
            fixture.filters().to_vec(),
            fixture.projections().to_vec(),
            vec![Aggregation::new(
                operator_id("invalid-count-star-quantifier"),
                AggregationFunction::Count,
                Some(SetQuantifier::Distinct),
                Vec::new(),
                true,
                binding("rows"),
            )],
            Vec::new(),
            None,
        ),
        Err(QueryIrError::AggregationRequiresExpression)
    );
}
