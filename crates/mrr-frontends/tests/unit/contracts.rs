use crate::{FrontendError, PARSER_OWNED_COMPILATION_SCHEMA_V1, QueryFrontend};
use mrr_query::{
    AggregationFunction, BinaryOperator, Direction, Expression, PageValue, Parameter, ResultMode,
    SetQuantifier, UnaryOperator, Value,
};

const PARITY_QUERY: &str =
    "MATCH (a:Module)-[:DEPENDS_ON]->(b:Module) WHERE a.name = 'runtime' RETURN b";

#[test]
fn parser_owned_gql_projection_preserves_the_bounded_graph_shape() {
    let query = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql)
        .compile("parity.gql", PARITY_QUERY)
        .expect("parser-owned GQL projection");

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
fn parser_owned_compilation_receipt_binds_source_grammar_and_query() {
    let compilation = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql)
        .compile_with_receipt("parity.gql", PARITY_QUERY)
        .expect("source-bound parser-owned compilation");
    let receipt = &compilation.receipt;

    assert_eq!(receipt.schema, PARSER_OWNED_COMPILATION_SCHEMA_V1);
    assert_eq!(receipt.language, mrr_gerbil::ParserLanguage::Gql);
    assert_eq!(receipt.source_name, "parity.gql");
    assert_eq!(
        receipt.source_digest,
        "sha256:b4974dbe0e0751df01f7010fa245d91c5b2db689c951b2add81b6e3ba14d10a5"
    );
    assert!(receipt.grammar_digest.starts_with("sha256:"));
    assert_eq!(receipt.query_id, compilation.query.id());
}

#[test]
fn gql_and_cypher_frontends_admit_identical_normalized_ir() {
    let gql = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql)
        .compile_with_receipt("parity.gql", PARITY_QUERY)
        .expect("GQL parity compilation");
    let cypher = QueryFrontend::new(mrr_gerbil::ParserLanguage::Cypher)
        .compile_with_receipt("parity.cypher", PARITY_QUERY)
        .expect("Cypher parity compilation");

    assert_eq!(gql.receipt.language, mrr_gerbil::ParserLanguage::Gql);
    assert_eq!(cypher.receipt.language, mrr_gerbil::ParserLanguage::Cypher);
    assert_ne!(gql.receipt.grammar_digest, cypher.receipt.grammar_digest);
    assert_eq!(gql.query, cypher.query);
    assert_eq!(
        gql.query.encode_canonical().expect("GQL canonical IR"),
        cypher
            .query
            .encode_canonical()
            .expect("Cypher canonical IR")
    );
}

#[test]
fn cypher_unlowered_update_never_publishes_partial_query_ir() {
    let error = QueryFrontend::new(mrr_gerbil::ParserLanguage::Cypher)
        .compile("update.cypher", "MATCH (n:Person) DELETE n RETURN n")
        .expect_err("Cypher update semantics must not be erased");
    assert_eq!(
        error,
        FrontendError::Unsupported("data update statement".into())
    );
}

#[test]
fn parser_owned_properties_and_unicode_are_lossless() {
    let source = "MATCH (n:Person {name: '\u{827e}\u{8fbe}', age: 42}) RETURN n";
    let query = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql)
        .compile("unicode.gql", source)
        .expect("parser-owned Unicode property query");

    assert_eq!(query.graph().paths()[0].start().binding().as_str(), "n");
    assert_eq!(query.filters().len(), 2);
    assert_eq!(query.projections().len(), 1);
}

#[test]
fn parser_owned_expression_filter_order_and_limit_reach_meta_query_ir() {
    let frontend = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql);
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
    assert_eq!(query.limit(), Some(&PageValue::Literal(0)));
}

#[test]
fn parser_owned_parameters_and_truth_predicates_are_admitted() {
    let frontend = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql);
    for source in [
        "MATCH (n {value: $limit}) RETURN $limit",
        "MATCH (n) WHERE n.deleted IS NULL RETURN n.deleted IS NOT NULL, TRUE IS TRUE, NULL IS UNKNOWN",
        "MATCH (n) WHERE n.deleted IS NOT NULL RETURN FALSE IS NOT FALSE",
    ] {
        frontend
            .compile("parser-owned-expression.gql", source)
            .expect("parser-owned parameter or truth predicate");
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
fn parser_owned_rejection_never_falls_back_to_another_parser() {
    let error = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql)
        .compile("rejected.gql", "MATCH (\n")
        .expect_err("rejected ParseArtifact must fail closed");
    assert!(matches!(error, FrontendError::ParserOwned(_)));
}

#[test]
fn parser_owned_where_precedence_is_not_partially_consumed() {
    assert_eq!(
        QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql)
            .compile(
                "where-precedence.gql",
                "MATCH (n) WHERE n.score > 2 AND n.active = TRUE RETURN n",
            )
            .expect_err("a parser artifact may not silently omit WHERE semantics"),
        FrontendError::Unsupported(
            "parser-owned lowering does not admit mixed operator precedence not encoded by parser CST"
                .into(),
        )
    );
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
        (
            "graph-element-predicate.gql",
            "MATCH (a)-[e]->(b) RETURN e IS DIRECTED",
        ),
        (
            "complete-pipeline.gql",
            "MATCH (n) LET team = n.team RETURN team AS team, COUNT(n) AS total GROUP BY team ORDER BY total DESC OFFSET 1 LIMIT 10",
        ),
        ("for.gql", "MATCH (n) FOR value IN [1, 2] RETURN value"),
        (
            "group-by.gql",
            "MATCH (n) RETURN n.team AS team, COUNT(n) AS total GROUP BY n.team",
        ),
    ] {
        let error = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql)
            .compile(source_name, source)
            .expect_err("unsupported upstream parser syntax must not reach another parser");
        assert!(
            matches!(error, FrontendError::ParserOwned(_)),
            "{source_name}: {error:?}"
        );
    }
}

#[test]
fn parser_owned_aggregates_are_admitted() {
    let frontend = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql);
    for source in [
        "MATCH (n) RETURN COUNT(n)",
        "MATCH (n) RETURN AVG(DISTINCT n.score)",
        "MATCH (n) RETURN SUM(ALL n.score)",
        "MATCH (n) RETURN PERCENTILE_CONT(n.score, 0.5)",
    ] {
        frontend
            .compile("parser-owned-aggregate.gql", source)
            .expect("parser-owned aggregate");
    }
}

#[test]
fn parser_owned_numeric_and_structured_literals_are_admitted() {
    let frontend = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql);
    for source in [
        "MATCH (n) RETURN 0.5, 1e3",
        "MATCH (n) RETURN DATE '2024-01-02', TIME '12:34:56'",
        "MATCH (n) RETURN DATETIME '2024-01-02T12:34:56', DURATION 'P1D'",
        "MATCH (n) RETURN [1, 2], RECORD {a: 1, b: 'two'}",
    ] {
        frontend
            .compile("parser-owned-literal.gql", source)
            .expect("parser-owned literal");
    }
}

#[test]
fn parser_owned_arithmetic_and_boolean_precedence_is_admitted() {
    let frontend = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql);
    for source in [
        "MATCH (n) RETURN n.score + 1, 2 * 3, 10 / 2, 7 - 3",
        "MATCH (n) WHERE (n.score > 2) AND (n.active = TRUE) RETURN n",
        "MATCH (n) WHERE NOT ((n.active = FALSE) OR (n.score < 0)) RETURN n",
    ] {
        frontend
            .compile("parser-owned-operators.gql", source)
            .expect("parser-owned operators");
    }

    assert_eq!(
        frontend
            .compile(
                "ambiguous-precedence.gql",
                "MATCH (n) RETURN n.score + 1 * 2",
            )
            .expect_err("parser CST must encode precedence before admission"),
        FrontendError::Unsupported(
            "parser-owned lowering does not admit mixed operator precedence not encoded by parser CST"
                .into()
        )
    );
}

#[test]
fn parser_owned_multiple_match_paths_reach_one_graph_pattern() {
    let frontend = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql);
    let source = concat!(
        "MATCH (a:Person {name: 'Ada'})-[e:KNOWS]->(b), ",
        "(c)<-[f:LIKES {weight: 1}]-(d) RETURN a, b, c, d"
    );
    let parser_owned = frontend
        .compile("parser-owned-multiple-paths.gql", source)
        .expect("parser-owned multiple path pattern");
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
        ("MATCH REPEATABLE ELEMENTS (n) RETURN n", "graph match mode"),
        (
            "MATCH ALL SHORTEST PATHS (n) RETURN n",
            "path search prefix",
        ),
    ] {
        assert_eq!(
            QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql)
                .compile("unsupported.gql", source)
                .expect_err("unlowered semantics must never be erased"),
            FrontendError::Unsupported(expected.into())
        );
    }
}

#[test]
fn parser_owned_projection_is_canonically_deterministic() {
    let frontend = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql);
    let parser_owned = frontend
        .compile("query.gql", PARITY_QUERY)
        .expect("parser-owned GQL query");
    let repeated = frontend
        .compile("query.gql", PARITY_QUERY)
        .expect("repeat parser-owned GQL projection");
    assert_eq!(
        parser_owned
            .encode_canonical()
            .expect("parser-owned canonical bytes"),
        repeated
            .encode_canonical()
            .expect("repeated canonical bytes")
    );
}

#[test]
fn parser_owned_projection_rejects_mutation_before_meta_query_admission() {
    let error = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql)
        .compile("unsupported.gql", "INSERT (a)")
        .expect_err("data mutation is outside the admitted MetaQueryIr slice");
    assert!(matches!(error, FrontendError::Unsupported(_)));
}

#[test]
fn unimplemented_profile_surfaces_fail_closed_without_a_public_domain_model() {
    for (source_name, source) in [
        ("optional-match.gql", "OPTIONAL MATCH (n) RETURN n"),
        ("catalog.gql", "CREATE GRAPH analytics"),
        ("procedure.gql", "CALL analytics.refresh()"),
        ("session.gql", "SESSION SET SCHEMA analytics"),
    ] {
        let error = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql)
            .compile(source_name, source)
            .expect_err("an unimplemented profile surface must publish no MetaQueryIr");
        assert!(
            !matches!(&error, FrontendError::ParserOwned(message) if message.contains("RuntimeStatus")),
            "runtime setup failure is not semantic rejection: {error:?}"
        );
    }
}

#[test]
fn primitive_result_semantics_are_explicit_in_meta_query_ir() {
    let frontend = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql);
    let distinct = frontend
        .compile("distinct.gql", "MATCH (n) RETURN DISTINCT n")
        .expect("DISTINCT result");
    assert_eq!(
        distinct.result().mode(),
        ResultMode::Return(SetQuantifier::Distinct)
    );

    let all_bindings = frontend
        .compile("star.gql", "MATCH (n)-[r]->(m) RETURN *")
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
        .compile("finish.gql", "MATCH (n) FINISH")
        .expect("FINISH terminal");
    assert_eq!(finish.result().mode(), ResultMode::Finish);
    assert!(finish.projections().is_empty());
}

#[test]
fn parser_owned_result_grouping_and_pagination_are_admitted() {
    let frontend = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql);
    for source in [
        "MATCH (n) RETURN DISTINCT n",
        "MATCH (n)-[r]->(m) RETURN *",
        "MATCH (n) FINISH",
        "MATCH (n) RETURN n OFFSET 2 LIMIT $limit",
    ] {
        frontend
            .compile("result.gql", source)
            .unwrap_or_else(|error| panic!("parser-owned source={source}: {error:?}"));
    }
}

#[test]
fn numeric_unary_operators_lower_without_a_compatibility_operator() {
    let query = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql)
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
    ] {
        assert_eq!(
            QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql)
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
            QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql)
                .compile("unsupported-path-authority.gql", source)
                .expect_err("MetaQueryIR has no path-search execution authority"),
            FrontendError::Unsupported(expected.into())
        );
    }
}

#[test]
fn filter_lowers_to_meta_query_filter() {
    let filter = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql)
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
}

#[test]
fn general_literal_values_lower_to_backend_neutral_meta_query_ir() {
    let source = concat!(
        "MATCH (n) RETURN DATE '2026-09-02', TIME '12:34:56.789Z', ",
        "TIMESTAMP '2026-09-02T12:34:56Z', DURATION 'P1DT2H', ",
        "RECORD {name: 'Ada', age: 42}"
    );
    let query = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql)
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

    let repeated = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql)
        .compile("general-literals.gql", source)
        .expect("same source compiles deterministically");
    assert_eq!(query.id(), repeated.id());
    assert_eq!(
        query.projections()[4].operator(),
        repeated.projections()[4].operator()
    );
    assert_eq!(
        query
            .encode_canonical()
            .expect("canonical general literals"),
        repeated
            .encode_canonical()
            .expect("repeated canonical bytes")
    );

    let changed_source = source.replace("age: 42", "age: 43");
    let changed = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql)
        .compile("general-literals.gql", &changed_source)
        .expect("changed literal remains valid");
    assert_ne!(query.id(), changed.id());
    assert_ne!(
        query.projections()[4].operator(),
        changed.projections()[4].operator()
    );
}

#[test]
fn iso_aggregate_family_lowers_to_explicit_meta_query_aggregations() {
    let source = concat!(
        "MATCH (n) RETURN COUNT(n) AS rows, COUNT(DISTINCT n) AS nodes, ",
        "PERCENTILE_CONT(n.score, 0.5) AS median"
    );
    let query = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql)
        .compile("aggregate-family.gql", source)
        .expect("aggregate family lowers to MetaQueryIR");

    assert!(query.projections().is_empty());
    assert_eq!(query.aggregations().len(), 3);
    assert_eq!(
        query.aggregations()[0].function(),
        AggregationFunction::Count
    );
    assert!(!query.aggregations()[0].is_count_star());
    assert_eq!(query.aggregations()[0].expressions().len(), 1);
    assert_eq!(
        query.aggregations()[1].quantifier(),
        Some(SetQuantifier::Distinct)
    );
    assert_eq!(query.aggregations()[1].expressions().len(), 1);
    assert_eq!(
        query.aggregations()[2].function(),
        AggregationFunction::PercentileContinuous
    );
    assert_eq!(query.aggregations()[2].quantifier(), None);
    assert_eq!(query.aggregations()[2].expressions().len(), 2);

    let repeated = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql)
        .compile("aggregate-family.gql", source)
        .expect("repeat aggregate compilation");
    assert_eq!(query.id(), repeated.id());
    assert_eq!(
        query.encode_canonical().expect("aggregate canonical bytes"),
        repeated
            .encode_canonical()
            .expect("repeated aggregate bytes")
    );
}

#[test]
fn character_string_source_forms_share_only_semantically_equal_mrr_identity() {
    let frontend = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql);
    let single = frontend
        .compile("single-quoted.gql", r"MATCH (n) RETURN 'A\nB'")
        .expect("single-quoted escaped character sequence");
    let double = frontend
        .compile("double-quoted.gql", r#"MATCH (n) RETURN "A\nB""#)
        .expect("double-quoted escaped character sequence");
    assert_eq!(single.id(), double.id());
    assert_eq!(
        single.encode_canonical().expect("single canonical bytes"),
        double.encode_canonical().expect("double canonical bytes")
    );
    assert_eq!(
        single.projections()[0].expression(),
        &Expression::Literal(Value::String("A\nB".into()))
    );
}

#[test]
fn dynamic_parameter_identity_uses_decoded_name() {
    let extended = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql)
        .compile(
            "parameter-extended.gql",
            "MATCH (n {value: $limit}) RETURN $limit",
        )
        .expect("extended dynamic parameter");
    let changed = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql)
        .compile(
            "parameter-changed.gql",
            "MATCH (n {value: $other}) RETURN $other",
        )
        .expect("changed dynamic parameter");

    assert_eq!(
        extended.projections()[0].expression(),
        &Expression::Parameter(Parameter::new("limit").expect("parameter identity"))
    );
    assert_ne!(extended.id(), changed.id());
}

#[test]
fn null_and_truth_predicates_lower_to_explicit_mrr_unary_operators() {
    let query = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql)
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

    let negated = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql)
        .compile(
            "truth-null-predicates-negated.gql",
            "MATCH (n) WHERE n.deleted IS NOT NULL RETURN TRUE IS NOT TRUE",
        )
        .expect("negated truth/null predicate slice");
    assert_ne!(query.id(), negated.id());
}

#[test]
fn zero_limit_is_valid_while_unowned_page_semantics_fail_closed_by_exact_name() {
    let frontend = QueryFrontend::new(mrr_gerbil::ParserLanguage::Gql);
    let zero = frontend
        .compile("zero-limit.gql", "MATCH (n) RETURN n LIMIT 0")
        .expect("ISO zero LIMIT is a valid empty-result bound");
    assert_eq!(zero.limit(), Some(&PageValue::Literal(0)));

    let dynamic = frontend
        .compile("dynamic-limit.gql", "MATCH (n) RETURN n LIMIT $limit")
        .expect("dynamic LIMIT is represented by MetaQueryIR PageValue");
    assert_eq!(
        dynamic.limit(),
        Some(&PageValue::Parameter(
            Parameter::new("limit").expect("parameter identity")
        ))
    );
    assert_eq!(
        frontend.compile(
            "null-ordering.gql",
            "MATCH (n) RETURN n ORDER BY n NULLS LAST LIMIT 1",
        ),
        Err(FrontendError::Unsupported("NULLS ordering".into()))
    );
}
