use crate::{FrontendError, PARSER_OWNED_COMPILATION_SCHEMA_V1, QueryFrontend, QueryLanguage};
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
fn parser_owned_gql_lowers_to_the_same_meta_query_ir() {
    let frontend = QueryFrontend::new(QueryLanguage::Gql);
    let legacy = frontend
        .compile("parity.gql", PARITY_QUERY)
        .expect("legacy parity slice");
    let parser_owned = frontend
        .compile_parser_owned("parity.gql", PARITY_QUERY)
        .expect("parser-owned parity slice");
    assert_eq!(parser_owned, legacy);
}

#[test]
fn parser_owned_compilation_receipt_binds_source_grammar_and_query() {
    let compilation = QueryFrontend::new(QueryLanguage::Gql)
        .compile_parser_owned_with_receipt("parity.gql", PARITY_QUERY)
        .expect("source-bound parser-owned compilation");
    let receipt = &compilation.receipt;

    assert_eq!(receipt.schema, PARSER_OWNED_COMPILATION_SCHEMA_V1);
    assert_eq!(receipt.source_name, "parity.gql");
    assert_eq!(
        receipt.source_digest,
        "sha256:b4974dbe0e0751df01f7010fa245d91c5b2db689c951b2add81b6e3ba14d10a5"
    );
    assert!(receipt.grammar_digest.starts_with("sha256:"));
    assert_eq!(receipt.query_id, compilation.query.id());
}

#[test]
fn parser_owned_properties_and_unicode_are_lossless() {
    let source = "MATCH (n:Person {name: '\u{827e}\u{8fbe}', age: 42}) RETURN n";
    let query = QueryFrontend::new(QueryLanguage::Gql)
        .compile_parser_owned("unicode.gql", source)
        .expect("parser-owned Unicode property query");

    assert_eq!(query.graph().paths()[0].start().binding().as_str(), "n");
    assert_eq!(query.filters().len(), 2);
    assert_eq!(query.projections().len(), 1);
}

#[test]
fn parser_owned_expression_filter_order_and_limit_reach_meta_query_ir() {
    let frontend = QueryFrontend::new(QueryLanguage::Gql);
    let unary = frontend
        .compile_parser_owned("unary.gql", "MATCH (n) RETURN -1, +2")
        .expect("parser-owned unary expressions");
    assert!(matches!(
        unary.projections()[0].expression(),
        Expression::Unary {
            operator: UnaryOperator::Negate,
            ..
        }
    ));
    assert_eq!(
        unary.projections()[1].expression(),
        &Expression::Literal(Value::Integer(2))
    );

    let source = "MATCH (n) FILTER n.score > 1 RETURN n ORDER BY n DESC LIMIT 0";
    let query = frontend
        .compile_parser_owned("order-limit.gql", source)
        .expect("parser-owned filter/order/limit");
    assert!(matches!(
        query.filters()[0].predicate(),
        Expression::Binary {
            operator: BinaryOperator::Greater,
            ..
        }
    ));
    assert_eq!(
        query,
        frontend
            .compile("order-limit.gql", source)
            .expect("legacy differential oracle")
    );
    assert_eq!(query.limit(), Some(0));
}

#[test]
fn parser_owned_parameters_and_truth_predicates_match_the_legacy_oracle() {
    let frontend = QueryFrontend::new(QueryLanguage::Gql);
    for source in [
        "MATCH (n {value: $limit}) RETURN $limit",
        "MATCH (n) WHERE n.deleted IS NULL RETURN n.deleted IS NOT NULL, TRUE IS TRUE, NULL IS UNKNOWN",
        "MATCH (n) WHERE n.deleted IS NOT NULL RETURN FALSE IS NOT FALSE",
    ] {
        assert_eq!(
            frontend
                .compile_parser_owned("parser-owned-expression.gql", source)
                .expect("parser-owned parameter or truth predicate"),
            frontend
                .compile("legacy-expression.gql", source)
                .expect("legacy differential oracle")
        );
    }

    let parameter = frontend
        .compile_parser_owned(
            "parser-owned-parameter.gql",
            "MATCH (n {value: $limit}) RETURN $limit",
        )
        .expect("parser-owned dynamic parameter");
    assert_eq!(
        parameter.projections()[0].expression(),
        &Expression::Parameter(Parameter::new("limit").expect("parameter identity"))
    );

    let predicates = frontend
        .compile_parser_owned(
            "parser-owned-predicates.gql",
            "MATCH (n) WHERE n.deleted IS NULL RETURN n.deleted IS NOT NULL, TRUE IS TRUE, NULL IS UNKNOWN",
        )
        .expect("parser-owned NULL and truth predicates");
    assert!(matches!(
        predicates.filters()[0].predicate(),
        Expression::Unary {
            operator: UnaryOperator::IsNull,
            ..
        }
    ));
    assert!(matches!(
        predicates.projections()[2].expression(),
        Expression::Unary {
            operator: UnaryOperator::IsUnknown,
            ..
        }
    ));
}

#[test]
fn parser_owned_rejection_never_falls_back_to_legacy_parser() {
    let error = QueryFrontend::new(QueryLanguage::Gql)
        .compile_parser_owned("rejected.gql", "MATCH (\n")
        .expect_err("rejected ParseArtifact must fail closed");
    assert!(matches!(error, FrontendError::ParserOwned(_)));
}

#[test]
fn parser_owned_upstream_grammar_gaps_remain_typed_and_fail_closed() {
    for (source_name, source) in [
        (
            "parameter-delimited.gql",
            "MATCH (n {value: $\"limit\"}) RETURN $\"limit\"",
        ),
        ("binary-literal.gql", "MATCH (n) RETURN X'CAFE'"),
        ("count-star.gql", "MATCH (n) RETURN COUNT(*)"),
    ] {
        let error = QueryFrontend::new(QueryLanguage::Gql)
            .compile_parser_owned(source_name, source)
            .expect_err("unsupported upstream parser syntax must not reach legacy fallback");
        assert!(
            matches!(error, FrontendError::ParserOwned(_)),
            "{source_name}: {error:?}"
        );
    }
}

#[test]
fn parser_owned_aggregates_match_the_legacy_oracle() {
    let frontend = QueryFrontend::new(QueryLanguage::Gql);
    for source in [
        "MATCH (n) RETURN COUNT(n)",
        "MATCH (n) RETURN AVG(DISTINCT n.score)",
        "MATCH (n) RETURN SUM(ALL n.score)",
        "MATCH (n) RETURN PERCENTILE_CONT(n.score, 0.5)",
    ] {
        assert_eq!(
            frontend
                .compile_parser_owned("parser-owned-aggregate.gql", source)
                .expect("parser-owned aggregate"),
            frontend
                .compile("legacy-aggregate.gql", source)
                .expect("legacy aggregate differential oracle")
        );
    }
}

#[test]
fn parser_owned_numeric_and_structured_literals_match_the_legacy_oracle() {
    let frontend = QueryFrontend::new(QueryLanguage::Gql);
    for source in [
        "MATCH (n) RETURN 0.5, 1e3",
        "MATCH (n) RETURN DATE '2024-01-02', TIME '12:34:56'",
        "MATCH (n) RETURN DATETIME '2024-01-02T12:34:56', DURATION 'P1D'",
        "MATCH (n) RETURN [1, 2], RECORD {a: 1, b: 'two'}",
    ] {
        assert_eq!(
            frontend
                .compile_parser_owned("parser-owned-literal.gql", source)
                .expect("parser-owned literal"),
            frontend
                .compile("legacy-literal.gql", source)
                .expect("legacy literal differential oracle")
        );
    }
}

#[test]
fn parser_owned_arithmetic_and_boolean_precedence_matches_the_legacy_oracle() {
    let frontend = QueryFrontend::new(QueryLanguage::Gql);
    for source in [
        "MATCH (n) RETURN n.score + 1, 2 * 3, 10 / 2, 7 - 3",
        "MATCH (n) WHERE (n.score > 2) AND (n.active = TRUE) RETURN n",
        "MATCH (n) WHERE NOT ((n.active = FALSE) OR (n.score < 0)) RETURN n",
    ] {
        assert_eq!(
            frontend
                .compile_parser_owned("parser-owned-operators.gql", source)
                .expect("parser-owned operators"),
            frontend
                .compile("legacy-operators.gql", source)
                .expect("legacy operator differential oracle")
        );
    }

    for source in [
        "MATCH (n) RETURN n.score + 1 * 2",
        "MATCH (n) WHERE n.score > 2 AND n.active = TRUE RETURN n",
    ] {
        assert_eq!(
            frontend
                .compile_parser_owned("ambiguous-precedence.gql", source)
                .expect_err("parser CST must encode precedence before admission"),
            FrontendError::Unsupported(
                "parser-owned lowering does not admit mixed operator precedence not encoded by parser CST"
                    .into()
            )
        );
    }
}

#[test]
fn parser_owned_multiple_match_paths_reach_one_graph_pattern() {
    let frontend = QueryFrontend::new(QueryLanguage::Gql);
    let source = concat!(
        "MATCH (a:Person {name: 'Ada'})-[e:KNOWS]->(b), ",
        "(c)<-[f:LIKES {weight: 1}]-(d) RETURN a, b, c, d"
    );
    let parser_owned = frontend
        .compile_parser_owned("parser-owned-multiple-paths.gql", source)
        .expect("parser-owned multiple path pattern");
    assert_eq!(
        parser_owned,
        frontend
            .compile("legacy-multiple-paths.gql", source)
            .expect("legacy multiple path differential oracle")
    );
    assert_eq!(parser_owned.graph().paths().len(), 2);
    assert_eq!(
        parser_owned.graph().paths()[0].segments()[0]
            .relation()
            .direction(),
        Direction::Outgoing
    );
    assert_eq!(
        parser_owned.graph().paths()[1].segments()[0]
            .relation()
            .direction(),
        Direction::Incoming
    );
    assert_eq!(parser_owned.filters().len(), 2);

    let anonymous = frontend
        .compile_parser_owned("anonymous-multiple-paths.gql", "MATCH (), () RETURN 1")
        .expect("anonymous bindings remain path-scoped");
    assert_eq!(
        anonymous.graph().paths()[0].start().binding().as_str(),
        "_node_0_0"
    );
    assert_eq!(
        anonymous.graph().paths()[1].start().binding().as_str(),
        "_node_1_0"
    );

    let correlated_source = "MATCH (a)-[e]->(b), (a)-[f]->(c) RETURN a";
    let correlated = frontend
        .compile_parser_owned("correlated-multiple-paths.gql", correlated_source)
        .expect("an explicit binding correlates graph paths");
    assert_eq!(
        correlated,
        frontend
            .compile("legacy-correlated-multiple-paths.gql", correlated_source)
            .expect("legacy correlated path differential oracle")
    );
    assert_eq!(
        correlated.graph().paths()[0].start().binding().as_str(),
        "a"
    );
    assert_eq!(
        correlated.graph().paths()[1].start().binding().as_str(),
        "a"
    );
}

#[test]
fn parser_owned_entrypoint_rejects_non_gql_authority() {
    assert_eq!(
        QueryFrontend::new(QueryLanguage::Cypher)
            .compile_parser_owned("query.cypher", PARITY_QUERY)
            .expect_err("Cypher is not owned by the ISO parser"),
        FrontendError::Unsupported(
            "parser-owned lowering is only authoritative for ISO GQL".into()
        )
    );
}

#[test]
fn parser_owned_entrypoint_rejects_unlowered_semantics() {
    for (source, expected) in [
        ("MATCH (n) RETURN *", "RETURN *"),
        ("MATCH REPEATABLE ELEMENTS (n) RETURN n", "MatchMode"),
        ("MATCH ALL SHORTEST PATHS (n) RETURN n", "PathPatternPrefix"),
        ("MATCH (n) RETURN n OFFSET 1", "OffsetClause"),
    ] {
        assert_eq!(
            QueryFrontend::new(QueryLanguage::Gql)
                .compile_parser_owned("unsupported.gql", source)
                .expect_err("unlowered semantics must never be erased"),
            FrontendError::Unsupported(format!("parser-owned lowering does not admit {expected}"))
        );
    }
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
