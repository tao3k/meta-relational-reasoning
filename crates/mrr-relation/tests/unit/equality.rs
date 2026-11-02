use crate::{EntityId, Value};

// Independent derived structural oracle for the former generic equality path.
#[derive(PartialEq)]
enum Reference<'a> {
    Entity(EntityId),
    Null,
    Boolean(bool),
    Integer(i64),
    Text(u8, &'a str),
    Bytes(&'a [u8]),
    List(Vec<Reference<'a>>),
    Record(Vec<(&'a str, Reference<'a>)>),
}

fn reference(value: &Value) -> Reference<'_> {
    match value {
        Value::Entity(id) => Reference::Entity(*id),
        Value::Null => Reference::Null,
        Value::Boolean(value) => Reference::Boolean(*value),
        Value::Integer(value) => Reference::Integer(*value),
        Value::Decimal(value) => Reference::Text(0, value),
        Value::Float(value) => Reference::Text(1, value),
        Value::String(value) => Reference::Text(2, value),
        Value::Date(value) => Reference::Text(3, value),
        Value::Time(value) => Reference::Text(4, value),
        Value::Timestamp(value) => Reference::Text(5, value),
        Value::Duration(value) => Reference::Text(6, value),
        Value::ByteString(value) => Reference::Bytes(value),
        Value::List(values) => Reference::List(values.iter().map(reference).collect()),
        Value::Record(values) => Reference::Record(
            values
                .iter()
                .map(|(key, value)| (key.as_str(), reference(value)))
                .collect(),
        ),
    }
}

#[test]
fn direct_recursive_equality_matches_derived_structural_equality() {
    let mut values = vec![
        Value::Entity(EntityId::from_canonical_bytes(b"entity").unwrap()),
        Value::Null,
        Value::Boolean(false),
        Value::Boolean(true),
        Value::Integer(-1),
        Value::Integer(0),
        Value::Decimal("1.0".into()),
        Value::Decimal("1.00".into()),
        Value::Float("NaN".into()),
        Value::Float("nan".into()),
        Value::String("1.0".into()),
        Value::Date("1.0".into()),
        Value::Time("1.0".into()),
        Value::Timestamp("1.0".into()),
        Value::Duration("1.0".into()),
        Value::ByteString(vec![]),
        Value::ByteString(vec![0, 1]),
        Value::ByteString(vec![1, 0]),
        Value::List(vec![]),
        Value::Record(vec![]),
    ];
    for value in values.clone() {
        values.push(Value::List(vec![Value::Null, value.clone()]));
        values.push(Value::Record(vec![(
            "key".into(),
            Value::List(vec![value]),
        )]));
    }
    values.extend([
        Value::Record(vec![
            ("a".into(), Value::Null),
            ("a".into(), Value::Integer(1)),
        ]),
        Value::Record(vec![
            ("a".into(), Value::Integer(1)),
            ("a".into(), Value::Null),
        ]),
        Value::Record(vec![
            ("b".into(), Value::Null),
            ("a".into(), Value::Integer(1)),
        ]),
        Value::List(vec![Value::Null]),
        Value::List(vec![Value::Null, Value::Null]),
    ]);
    for left in &values {
        for right in &values {
            assert_eq!(
                left == right,
                reference(left) == reference(right),
                "{left:?} / {right:?}"
            );
        }
    }
}
