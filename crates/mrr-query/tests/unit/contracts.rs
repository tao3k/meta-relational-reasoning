use crate::{
    Atom, BinaryOperator, Binding, Direction, Expression, Filter, GraphPattern, MetaQueryIr,
    NodePattern, PathPattern, PathSegment, Projection, PropertyKey, QueryId, QueryIrError,
    QueryOperatorId, RelationId, RelationPattern, RelationalGoal, RelationalGoalError, Term, Value,
    Variable,
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
        Some(25),
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
}
