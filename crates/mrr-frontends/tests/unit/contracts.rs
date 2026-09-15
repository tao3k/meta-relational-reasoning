use crate::{FrontendError, PARSER_OWNED_COMPILATION_SCHEMA_V1, QueryFrontend};
use mrr_query::{
    AggregationFunction, BinaryOperator, Direction, Expression, PageValue, Parameter, ResultMode,
    SetQuantifier, UnaryOperator, Value,
};

const PARITY_QUERY: &str =
    "MATCH (a:Module)-[:DEPENDS_ON]->(b:Module) WHERE a.name = 'runtime' RETURN b";

#[test]
fn legacy_gql_oracle_preserves_the_bounded_graph_shape() {
    let query = QueryFrontend::new()
        .compile_legacy_oracle("parity.gql", PARITY_QUERY)
        .expect("legacy GQL differential oracle");

    let path = &query.graph().paths()[0];
    assert_eq!(path.start().binding().as_str(), "a");
    assert_eq!(
        path.segments()[0].relation().direction(),
        Direction::Outgoing
    );
    assert_eq!(path.segments()[0].node().binding().as_str(), "b");
    assert_eq!(query.projections().len(), 1);
    assert_eq!(query.filters().len(), 1);
    assert!(matches!(
        query.filters()[0].predicate(),
        Expression::Binary {
            operator: BinaryOperator::Equal,
            right,
            ..
        } if right.as_ref() == &Expression::Literal(Value::String("runtime".into()))
    ));
}

#[test]
fn parser_owned_gql_lowers_to_the_same_meta_query_ir() {
    let frontend = QueryFrontend::new();
    let legacy = frontend
        .compile_legacy_oracle("parity.gql", PARITY_QUERY)
        .expect("legacy parity slice");
    let parser_owned = frontend
        .compile("parity.gql", PARITY_QUERY)
        .expect("parser-owned parity slice");
    assert_eq!(parser_owned, legacy);
}

#[test]
fn parser_owned_compilation_receipt_binds_source_grammar_and_query() {
    let compilation = QueryFrontend::new()
        .compile_with_receipt("parity.gql", PARITY_QUERY)
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
    let query = QueryFrontend::new()
        .compile("unicode.gql", source)
        .expect("parser-owned Unicode property query");

    assert_eq!(query.graph().paths()[0].start().binding().as_str(), "n");
    assert_eq!(query.filters().len(), 2);
    assert_eq!(query.projections().len(), 1);
}

#[test]
fn parser_owned_expression_filter_order_and_limit_reach_meta_query_ir() {
    let frontend = QueryFrontend::new();
    let unary = frontend
        .compile("unary.gql", "MATCH (n) RETURN -1, +2")
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
        .compile("order-limit.gql", source)
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
            .compile_legacy_oracle("order-limit.gql", source)
            .expect("legacy differential oracle")
    );
    assert_eq!(query.limit(), Some(&PageValue::Literal(0)));
}

#[test]
fn parser_owned_parameters_and_truth_predicates_match_the_legacy_oracle() {
    let frontend = QueryFrontend::new();
    for source in [
        "MATCH (n {value: $limit}) RETURN $limit",
        "MATCH (n) WHERE n.deleted IS NULL RETURN n.deleted IS NOT NULL, TRUE IS TRUE, NULL IS UNKNOWN",
        "MATCH (n) WHERE n.deleted IS NOT NULL RETURN FALSE IS NOT FALSE",
    ] {
        assert_eq!(
            frontend
                .compile("parser-owned-expression.gql", source)
                .expect("parser-owned parameter or truth predicate"),
            frontend
                .compile_legacy_oracle("legacy-expression.gql", source)
                .expect("legacy differential oracle")
        );
    }

    let parameter = frontend
        .compile(
            "parser-owned-parameter.gql",
            "MATCH (n {value: $limit}) RETURN $limit",
        )
        .expect("parser-owned dynamic parameter");
    assert_eq!(
        parameter.projections()[0].expression(),
        &Expression::Parameter(Parameter::new("limit").expect("parameter identity"))
    );

    let predicates = frontend
        .compile(
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
    let error = QueryFrontend::new()
        .compile("rejected.gql", "MATCH (\n")
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
        let error = QueryFrontend::new()
            .compile(source_name, source)
            .expect_err("unsupported upstream parser syntax must not reach legacy fallback");
        assert!(
            matches!(error, FrontendError::ParserOwned(_)),
            "{source_name}: {error:?}"
        );
    }
}

#[test]
fn parser_owned_aggregates_match_the_legacy_oracle() {
    let frontend = QueryFrontend::new();
    for source in [
        "MATCH (n) RETURN COUNT(n)",
        "MATCH (n) RETURN AVG(DISTINCT n.score)",
        "MATCH (n) RETURN SUM(ALL n.score)",
        "MATCH (n) RETURN PERCENTILE_CONT(n.score, 0.5)",
    ] {
        assert_eq!(
            frontend
                .compile("parser-owned-aggregate.gql", source)
                .expect("parser-owned aggregate"),
            frontend
                .compile_legacy_oracle("legacy-aggregate.gql", source)
                .expect("legacy aggregate differential oracle")
        );
    }
}

#[test]
fn parser_owned_numeric_and_structured_literals_match_the_legacy_oracle() {
    let frontend = QueryFrontend::new();
    for source in [
        "MATCH (n) RETURN 0.5, 1e3",
        "MATCH (n) RETURN DATE '2024-01-02', TIME '12:34:56'",
        "MATCH (n) RETURN DATETIME '2024-01-02T12:34:56', DURATION 'P1D'",
        "MATCH (n) RETURN [1, 2], RECORD {a: 1, b: 'two'}",
    ] {
        assert_eq!(
            frontend
                .compile("parser-owned-literal.gql", source)
                .expect("parser-owned literal"),
            frontend
                .compile_legacy_oracle("legacy-literal.gql", source)
                .expect("legacy literal differential oracle")
        );
    }
}

#[test]
fn parser_owned_arithmetic_and_boolean_precedence_matches_the_legacy_oracle() {
    let frontend = QueryFrontend::new();
    for source in [
        "MATCH (n) RETURN n.score + 1, 2 * 3, 10 / 2, 7 - 3",
        "MATCH (n) WHERE (n.score > 2) AND (n.active = TRUE) RETURN n",
        "MATCH (n) WHERE NOT ((n.active = FALSE) OR (n.score < 0)) RETURN n",
    ] {
        assert_eq!(
            frontend
                .compile("parser-owned-operators.gql", source)
                .expect("parser-owned operators"),
            frontend
                .compile_legacy_oracle("legacy-operators.gql", source)
                .expect("legacy operator differential oracle")
        );
    }

    for source in [
        "MATCH (n) RETURN n.score + 1 * 2",
        "MATCH (n) WHERE n.score > 2 AND n.active = TRUE RETURN n",
    ] {
        assert_eq!(
            frontend
                .compile("ambiguous-precedence.gql", source)
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
    let frontend = QueryFrontend::new();
    let source = concat!(
        "MATCH (a:Person {name: 'Ada'})-[e:KNOWS]->(b), ",
        "(c)<-[f:LIKES {weight: 1}]-(d) RETURN a, b, c, d"
    );
    let parser_owned = frontend
        .compile("parser-owned-multiple-paths.gql", source)
        .expect("parser-owned multiple path pattern");
    assert_eq!(
        parser_owned,
        frontend
            .compile_legacy_oracle("legacy-multiple-paths.gql", source)
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
        .compile("anonymous-multiple-paths.gql", "MATCH (), () RETURN 1")
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
        .compile("correlated-multiple-paths.gql", correlated_source)
        .expect("an explicit binding correlates graph paths");
    assert_eq!(
        correlated,
        frontend
            .compile_legacy_oracle("legacy-correlated-multiple-paths.gql", correlated_source)
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
fn parser_owned_entrypoint_rejects_unlowered_semantics() {
    for (source, expected) in [
        ("MATCH REPEATABLE ELEMENTS (n) RETURN n", "MatchMode"),
        ("MATCH ALL SHORTEST PATHS (n) RETURN n", "PathPatternPrefix"),
    ] {
        assert_eq!(
            QueryFrontend::new()
                .compile("unsupported.gql", source)
                .expect_err("unlowered semantics must never be erased"),
            FrontendError::Unsupported(format!("parser-owned lowering does not admit {expected}"))
        );
    }
}

#[test]
fn parser_owned_and_legacy_oracle_have_identical_canonical_bytes() {
    let frontend = QueryFrontend::new();
    let parser_owned = frontend
        .compile("query.gql", PARITY_QUERY)
        .expect("parser-owned GQL query");
    let legacy = frontend
        .compile_legacy_oracle("query.gql", PARITY_QUERY)
        .expect("legacy GQL differential oracle");
    assert_eq!(
        parser_owned
            .encode_canonical()
            .expect("parser-owned canonical bytes"),
        legacy.encode_canonical().expect("legacy canonical bytes")
    );
}

#[test]
fn legacy_oracle_rejects_mutation_before_meta_query_admission() {
    let error = QueryFrontend::new()
        .compile_legacy_oracle("unsupported.gql", "INSERT (a)")
        .expect_err("data mutation is outside the differential slice");
    assert!(matches!(
        error,
        FrontendError::Unsupported(_) | FrontendError::Diagnostics(_)
    ));
}

#[test]
fn primitive_result_semantics_are_explicit_in_meta_query_ir() {
    let frontend = QueryFrontend::new();
    let distinct = frontend
        .compile_legacy_oracle("distinct.gql", "MATCH (n) RETURN DISTINCT n")
        .expect("DISTINCT result");
    assert_eq!(
        distinct.result().mode(),
        ResultMode::Return(SetQuantifier::Distinct)
    );

    let all_bindings = frontend
        .compile_legacy_oracle("star.gql", "MATCH (n)-[r]->(m) RETURN *")
        .expect("wildcard result");
    assert_eq!(
        all_bindings
            .projections()
            .iter()
            .map(|projection| projection.alias().as_str())
            .collect::<Vec<_>>(),
        vec!["n", "r", "m"]
    );

    let finish = frontend
        .compile_legacy_oracle("finish.gql", "MATCH (n) FINISH")
        .expect("FINISH terminal");
    assert_eq!(finish.result().mode(), ResultMode::Finish);
    assert!(finish.projections().is_empty());
}

#[test]
fn parser_owned_result_grouping_and_pagination_match_legacy_lowering() {
    let frontend = QueryFrontend::new();
    for source in [
        "MATCH (n) RETURN DISTINCT n",
        "MATCH (n)-[r]->(m) RETURN *",
        "MATCH (n) FINISH",
        "MATCH (n) RETURN n OFFSET 2 LIMIT $limit",
        "MATCH (n) RETURN n.team AS team, COUNT(*) AS total GROUP BY n.team",
    ] {
        assert_eq!(
            frontend
                .compile("result.gql", source)
                .unwrap_or_else(|error| panic!("parser-owned source={source}: {error:?}")),
            frontend
                .compile_legacy_oracle("result.gql", source)
                .unwrap_or_else(|error| panic!("legacy source={source}: {error:?}")),
            "source={source}"
        );
    }
}

#[test]
fn numeric_unary_operators_lower_without_a_compatibility_operator() {
    let query = QueryFrontend::new()
        .compile_legacy_oracle("unary.gql", "MATCH (n) RETURN -1, +2")
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
            QueryFrontend::new()
                .compile_legacy_oracle("unsupported-expression.gql", source)
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
            QueryFrontend::new()
                .compile_legacy_oracle("unsupported-path-authority.gql", source)
                .expect_err("MetaQueryIR has no path-search execution authority"),
            FrontendError::Unsupported(expected.into())
        );
    }
}

#[test]
fn complete_query_pipeline_is_rejected_as_one_unit_without_partial_consumption() {
    let source = "MATCH (n) LET team = n.team RETURN team AS team, COUNT(n) AS total GROUP BY team ORDER BY total DESC OFFSET 1 LIMIT 10";

    assert_eq!(
        QueryFrontend::new()
            .compile_legacy_oracle("complete-pipeline.gql", source)
            .expect_err("MetaQueryIR cannot partially consume the GQL query pipeline"),
        FrontendError::Unsupported("LET".into())
    );
}

#[test]
fn filter_lowers_to_meta_query_filter_while_for_fails_closed_by_operator_name() {
    let filter = QueryFrontend::new()
        .compile_legacy_oracle("filter.gql", "MATCH (n) FILTER n.score > 1 RETURN n")
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
        QueryFrontend::new()
            .compile_legacy_oracle("for.gql", "MATCH (n) FOR value IN [1, 2] RETURN value",)
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
    let query = QueryFrontend::new()
        .compile_legacy_oracle("general-literals.gql", source)
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

    let replay = QueryFrontend::new()
        .compile_legacy_oracle("general-literals.gql", source)
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
    let changed = QueryFrontend::new()
        .compile_legacy_oracle("general-literals.gql", &changed_source)
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
    let query = QueryFrontend::new()
        .compile_legacy_oracle("aggregate-family.gql", source)
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

    let replay = QueryFrontend::new()
        .compile_legacy_oracle("aggregate-family.gql", source)
        .expect("aggregate replay");
    assert_eq!(query.id(), replay.id());
    assert_eq!(
        query.encode_canonical().expect("aggregate canonical bytes"),
        replay.encode_canonical().expect("aggregate replay bytes")
    );
}

#[test]
fn character_string_source_forms_share_only_semantically_equal_mrr_identity() {
    let frontend = QueryFrontend::new();
    let single = frontend
        .compile_legacy_oracle("single-quoted.gql", r"MATCH (n) RETURN 'A\nB'")
        .expect("single-quoted escaped character sequence");
    let double = frontend
        .compile_legacy_oracle("double-quoted.gql", r#"MATCH (n) RETURN "A\nB""#)
        .expect("double-quoted escaped character sequence");
    let no_escape = frontend
        .compile_legacy_oracle("no-escape.gql", r"MATCH (n) RETURN @'A\nB'")
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
    let extended = QueryFrontend::new()
        .compile_legacy_oracle(
            "parameter-extended.gql",
            "MATCH (n {value: $limit}) RETURN $limit",
        )
        .expect("extended dynamic parameter");
    let delimited = QueryFrontend::new()
        .compile_legacy_oracle(
            "parameter-delimited.gql",
            "MATCH (n {value: $\"limit\"}) RETURN $\"limit\"",
        )
        .expect("delimited dynamic parameter");
    let changed = QueryFrontend::new()
        .compile_legacy_oracle(
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
    let query = QueryFrontend::new()
        .compile_legacy_oracle(
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

    let negated = QueryFrontend::new()
        .compile_legacy_oracle(
            "truth-null-predicates-negated.gql",
            "MATCH (n) WHERE n.deleted IS NOT NULL RETURN TRUE IS NOT TRUE",
        )
        .expect("negated truth/null predicate slice");
    assert_ne!(query.id(), negated.id());
}

#[test]
fn zero_limit_is_valid_while_unowned_page_semantics_fail_closed_by_exact_name() {
    let frontend = QueryFrontend::new();
    let zero = frontend
        .compile_legacy_oracle("zero-limit.gql", "MATCH (n) RETURN n LIMIT 0")
        .expect("ISO zero LIMIT is a valid empty-result bound");
    assert_eq!(zero.limit(), Some(&PageValue::Literal(0)));

    assert_eq!(
        frontend.compile_legacy_oracle("dynamic-limit.gql", "MATCH (n) RETURN n LIMIT $limit",),
        Err(FrontendError::Unsupported("dynamic LIMIT".into()))
    );
    assert_eq!(
        frontend.compile_legacy_oracle(
            "null-ordering.gql",
            "MATCH (n) RETURN n ORDER BY n NULLS LAST LIMIT 1",
        ),
        Err(FrontendError::Unsupported("NULLS ordering".into()))
    );
}
