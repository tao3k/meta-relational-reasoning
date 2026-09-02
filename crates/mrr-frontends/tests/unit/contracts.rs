use crate::{FrontendError, QueryFrontend, QueryLanguage};
use mrr_query::{BinaryOperator, Direction, Expression, UnaryOperator, Value};

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

#[test]
fn numeric_unary_operators_lower_without_a_compatibility_operator() {
    let query = QueryFrontend::new(QueryLanguage::Gql)
        .compile("unary.gql", "MATCH (n) RETURN -1, +2")
        .expect("numeric unary slice");

    assert!(matches!(
        query.projections()[0].expression(),
        Expression::Unary {
            operator: UnaryOperator::Negate,
            operand,
        } if operand.as_ref() == &Expression::Literal(Value::Integer(1))
    ));
    assert_eq!(
        query.projections()[1].expression(),
        &Expression::Literal(Value::Integer(2))
    );
}

#[test]
fn operators_absent_from_meta_query_ir_fail_closed_by_exact_name() {
    for (source, expected) in [
        ("MATCH (n) RETURN TRUE XOR FALSE", "XOR expression"),
        ("MATCH (n) RETURN 'a' || 'b'", "concatenation expression"),
    ] {
        assert_eq!(
            QueryFrontend::new(QueryLanguage::Gql)
                .compile("unsupported-expression.gql", source)
                .expect_err("target query algebra must reject an absent operator"),
            FrontendError::Unsupported(expected.into())
        );
    }
}

#[test]
fn complete_query_pipeline_is_rejected_as_one_unit_without_partial_consumption() {
    let source = "MATCH (n) LET team = n.team RETURN team AS team, COUNT(n) AS total GROUP BY team ORDER BY total DESC OFFSET 1 LIMIT 10";

    assert_eq!(
        QueryFrontend::new(QueryLanguage::Gql)
            .compile("complete-pipeline.gql", source)
            .expect_err("MetaQueryIR cannot partially consume the GQL query pipeline"),
        FrontendError::Unsupported("LET".into())
    );
}

#[test]
fn general_literal_values_lower_to_backend_neutral_meta_query_ir() {
    let source = concat!(
        "MATCH (n) RETURN X'CA FE', DATE '2026-09-02', TIME '12:34:56.789Z', ",
        "TIMESTAMP '2026-09-02T12:34:56Z', DURATION 'P1DT2H', ",
        "RECORD {name: 'Ada', age: 42}"
    );
    let query = QueryFrontend::new(QueryLanguage::Gql)
        .compile("general-literals.gql", source)
        .expect("general literal values lower to MetaQueryIR");

    let values = query
        .projections()
        .iter()
        .map(|projection| projection.expression())
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        values,
        vec![
            Expression::Literal(Value::ByteString(vec![0xCA, 0xFE])),
            Expression::Literal(Value::Date("2026-09-02".into())),
            Expression::Literal(Value::Time("12:34:56.789Z".into())),
            Expression::Literal(Value::Timestamp("2026-09-02T12:34:56Z".into())),
            Expression::Literal(Value::Duration("P1DT2H".into())),
            Expression::Literal(Value::Record(vec![
                ("NAME".into(), Value::String("Ada".into())),
                ("AGE".into(), Value::Integer(42)),
            ])),
        ]
    );

    let replay = QueryFrontend::new(QueryLanguage::Gql)
        .compile("general-literals.gql", source)
        .expect("same source replays deterministically");
    assert_eq!(query.id(), replay.id());
    assert_eq!(
        query.projections()[5].operator(),
        replay.projections()[5].operator()
    );
    assert_eq!(
        query
            .encode_canonical()
            .expect("canonical general literals"),
        replay.encode_canonical().expect("canonical replay")
    );

    let changed_source = source.replace("age: 42", "age: 43");
    let changed = QueryFrontend::new(QueryLanguage::Gql)
        .compile("general-literals.gql", &changed_source)
        .expect("changed literal remains valid");
    assert_ne!(query.id(), changed.id());
    assert_ne!(
        query.projections()[5].operator(),
        changed.projections()[5].operator()
    );
}

#[test]
fn character_string_source_forms_share_only_semantically_equal_mrr_identity() {
    let frontend = QueryFrontend::new(QueryLanguage::Gql);
    let single = frontend
        .compile("single-quoted.gql", r"MATCH (n) RETURN 'A\nB'")
        .expect("single-quoted escaped character sequence");
    let double = frontend
        .compile("double-quoted.gql", r#"MATCH (n) RETURN "A\nB""#)
        .expect("double-quoted escaped character sequence");
    let no_escape = frontend
        .compile("no-escape.gql", r"MATCH (n) RETURN @'A\nB'")
        .expect("NO_ESCAPE character sequence");

    assert_eq!(single.id(), double.id());
    assert_eq!(
        single.encode_canonical().expect("single canonical bytes"),
        double.encode_canonical().expect("double canonical bytes")
    );
    assert_ne!(single.id(), no_escape.id());
    assert_eq!(
        single.projections()[0].expression(),
        &Expression::Literal(Value::String("A\nB".into()))
    );
    assert_eq!(
        no_escape.projections()[0].expression(),
        &Expression::Literal(Value::String(r"A\nB".into()))
    );
}
