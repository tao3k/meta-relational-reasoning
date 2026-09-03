use crate::{FrontendError, QueryFrontend, QueryLanguage};
use mrr_query::{
    AggregationFunction, BinaryOperator, Direction, Expression, Parameter, SetQuantifier,
    UnaryOperator, Value,
};

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
fn primitive_result_semantics_absent_from_meta_query_ir_fail_closed_by_exact_name() {
    for (source, expected) in [
        ("MATCH (n) RETURN DISTINCT n", "RETURN DISTINCT"),
        ("MATCH (n) RETURN *", "RETURN *"),
        ("MATCH (n) FINISH", "FINISH result statement"),
    ] {
        assert_eq!(
            QueryFrontend::new(QueryLanguage::Gql)
                .compile("primitive-result.gql", source)
                .expect_err("MetaQueryIR must not erase primitive result semantics"),
            FrontendError::Unsupported(expected.into()),
            "source={source}"
        );
    }
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
        (
            "MATCH (n) RETURN n IS TYPED INT64",
            "value-type predicate expression",
        ),
        (
            "MATCH (a)-[e]->(b) RETURN e IS DIRECTED",
            "graph-element predicate expression",
        ),
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
fn graph_match_and_path_search_authority_fail_closed_by_exact_name() {
    for (source, expected) in [
        (
            "MATCH REPEATABLE ELEMENTS (a)-[e]->(b) RETURN a",
            "graph match mode",
        ),
        (
            "MATCH ALL SHORTEST TRAIL PATHS (a)-[e]->(b) RETURN a",
            "path search prefix",
        ),
        (
            "MATCH (a)-[e]->(b) KEEP ANY 2 WALK PATHS RETURN a",
            "KEEP path prefix",
        ),
    ] {
        assert_eq!(
            QueryFrontend::new(QueryLanguage::Gql)
                .compile("unsupported-path-authority.gql", source)
                .expect_err("MetaQueryIR has no path-search execution authority"),
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
fn filter_lowers_to_meta_query_filter_while_for_fails_closed_by_operator_name() {
    let filter = QueryFrontend::new(QueryLanguage::Gql)
        .compile("filter.gql", "MATCH (n) FILTER n.score > 1 RETURN n")
        .expect("FILTER is representable by MetaQueryIR");
    assert_eq!(filter.filters().len(), 1);
    assert!(matches!(
        filter.filters()[0].predicate(),
        Expression::Binary {
            operator: BinaryOperator::Greater,
            ..
        }
    ));

    assert_eq!(
        QueryFrontend::new(QueryLanguage::Gql)
            .compile("for.gql", "MATCH (n) FOR value IN [1, 2] RETURN value",)
            .expect_err("MetaQueryIR has no collection-expansion operator"),
        FrontendError::Unsupported("FOR collection expansion".into())
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
fn iso_aggregate_family_lowers_to_explicit_meta_query_aggregations() {
    let source = concat!(
        "MATCH (n) RETURN COUNT(*) AS rows, COUNT(DISTINCT n) AS nodes, ",
        "PERCENTILE_CONT(ALL n.score, 0.5) AS median"
    );
    let query = QueryFrontend::new(QueryLanguage::Gql)
        .compile("aggregate-family.gql", source)
        .expect("aggregate family lowers to MetaQueryIR");

    assert!(query.projections().is_empty());
    assert_eq!(query.aggregations().len(), 3);
    assert_eq!(
        query.aggregations()[0].function(),
        AggregationFunction::Count
    );
    assert!(query.aggregations()[0].is_count_star());
    assert!(query.aggregations()[0].expressions().is_empty());
    assert_eq!(
        query.aggregations()[1].quantifier(),
        Some(SetQuantifier::Distinct)
    );
    assert_eq!(query.aggregations()[1].expressions().len(), 1);
    assert_eq!(
        query.aggregations()[2].function(),
        AggregationFunction::PercentileContinuous
    );
    assert_eq!(
        query.aggregations()[2].quantifier(),
        Some(SetQuantifier::All)
    );
    assert_eq!(query.aggregations()[2].expressions().len(), 2);

    let replay = QueryFrontend::new(QueryLanguage::Gql)
        .compile("aggregate-family.gql", source)
        .expect("aggregate replay");
    assert_eq!(query.id(), replay.id());
    assert_eq!(
        query.encode_canonical().expect("aggregate canonical bytes"),
        replay.encode_canonical().expect("aggregate replay bytes")
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

#[test]
fn dynamic_parameter_identity_uses_decoded_name_not_source_delimiters() {
    let extended = QueryFrontend::new(QueryLanguage::Gql)
        .compile(
            "parameter-extended.gql",
            "MATCH (n {value: $limit}) RETURN $limit",
        )
        .expect("extended dynamic parameter");
    let delimited = QueryFrontend::new(QueryLanguage::Gql)
        .compile(
            "parameter-delimited.gql",
            "MATCH (n {value: $\"limit\"}) RETURN $\"limit\"",
        )
        .expect("delimited dynamic parameter");
    let changed = QueryFrontend::new(QueryLanguage::Gql)
        .compile(
            "parameter-changed.gql",
            "MATCH (n {value: $other}) RETURN $other",
        )
        .expect("changed dynamic parameter");

    assert_eq!(extended.id(), delimited.id());
    assert_eq!(
        extended
            .encode_canonical()
            .expect("extended canonical bytes"),
        delimited
            .encode_canonical()
            .expect("delimited canonical bytes")
    );
    assert_eq!(
        extended.projections()[0].expression(),
        &Expression::Parameter(Parameter::new("limit").expect("parameter identity"))
    );
    assert_ne!(extended.id(), changed.id());
}

#[test]
fn null_and_truth_predicates_lower_to_explicit_mrr_unary_operators() {
    let query = QueryFrontend::new(QueryLanguage::Gql)
        .compile(
            "truth-null-predicates.gql",
            "MATCH (n) WHERE n.deleted IS NULL RETURN n.deleted IS NOT NULL, TRUE IS TRUE, NULL IS UNKNOWN",
        )
        .expect("truth/null predicate slice");

    assert!(matches!(
        query.filters()[0].predicate(),
        Expression::Unary {
            operator: UnaryOperator::IsNull,
            ..
        }
    ));
    assert!(matches!(
        query.projections()[0].expression(),
        Expression::Unary {
            operator: UnaryOperator::IsNotNull,
            ..
        }
    ));
    assert!(matches!(
        query.projections()[1].expression(),
        Expression::Unary {
            operator: UnaryOperator::IsTrue,
            ..
        }
    ));
    assert!(matches!(
        query.projections()[2].expression(),
        Expression::Unary {
            operator: UnaryOperator::IsUnknown,
            ..
        }
    ));

    let negated = QueryFrontend::new(QueryLanguage::Gql)
        .compile(
            "truth-null-predicates-negated.gql",
            "MATCH (n) WHERE n.deleted IS NOT NULL RETURN TRUE IS NOT TRUE",
        )
        .expect("negated truth/null predicate slice");
    assert_ne!(query.id(), negated.id());
}

#[test]
fn zero_limit_is_valid_while_unowned_page_semantics_fail_closed_by_exact_name() {
    let frontend = QueryFrontend::new(QueryLanguage::Gql);
    let zero = frontend
        .compile("zero-limit.gql", "MATCH (n) RETURN n LIMIT 0")
        .expect("ISO zero LIMIT is a valid empty-result bound");
    assert_eq!(zero.limit(), Some(0));

    assert_eq!(
        frontend.compile("dynamic-limit.gql", "MATCH (n) RETURN n LIMIT $limit",),
        Err(FrontendError::Unsupported("dynamic LIMIT".into()))
    );
    assert_eq!(
        frontend.compile(
            "null-ordering.gql",
            "MATCH (n) RETURN n ORDER BY n NULLS LAST LIMIT 1",
        ),
        Err(FrontendError::Unsupported("NULLS ordering".into()))
    );
}
