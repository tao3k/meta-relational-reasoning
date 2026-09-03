use crate::Compiler;
use gql_catalog::{Catalog, CatalogName};
use gql_ir::Expression;

fn catalog() -> Catalog {
    Catalog::new(CatalogName("test-catalog".into()), Vec::new(), Vec::new())
}

#[test]
fn compiler_preserves_rowan_source_for_node_only_vertical_slice() {
    let source = "MATCH (n) RETURN n";
    let compiler = Compiler;
    let result = compiler.compile("node-only.gql", source, &catalog());

    assert!(
        result.parse.diagnostics.is_empty(),
        "parse: {:?}",
        result.parse.diagnostics
    );
    assert!(
        result.analysis.diagnostics.is_empty(),
        "sema: {:?}",
        result.analysis.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    let ir = result
        .analysis
        .ir
        .expect("node-only query should produce IR");
    assert_eq!(
        ir.matches
            .into_iter()
            .next()
            .expect("graph match")
            .paths
            .into_iter()
            .next()
            .expect("path pattern")
            .elements
            .len(),
        1
    );
    assert_eq!(ir.projection[0].expression, Expression::Binding("N".into()));
}

#[test]
fn generated_inputs_never_panic_in_the_complete_compiler_pipeline() {
    const FRAGMENTS: &[&str] = &[
        "MATCH",
        "RETURN",
        "WHERE",
        "UNION",
        "ORDER",
        "BY",
        "LIMIT",
        "OFFSET",
        "(",
        ")",
        "[",
        "]",
        "{",
        "}",
        "-",
        "->",
        "<-",
        ":",
        ",",
        ".",
        "'",
        "\"",
        "# comment\n",
        " ",
        "\t",
        "\r\n",
        "\0",
        "@",
        "/",
        "identifier_1",
        "Z9",
        "_",
        "~",
        "123",
        "1.2.3",
    ];

    for seed in 0_u64..1_024 {
        let mut state = seed.wrapping_add(1);
        let mut input = String::new();
        for _ in 0..48 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            input.push_str(FRAGMENTS[(state as usize) % FRAGMENTS.len()]);
        }

        let compile = || Compiler.compile("generated-input.gql", &input, &catalog());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(compile))
            .unwrap_or_else(|_| panic!("compiler panicked for generated seed {seed}"));
        let repeated = std::panic::catch_unwind(std::panic::AssertUnwindSafe(compile))
            .unwrap_or_else(|_| panic!("compiler repeat panicked for generated seed {seed}"));

        assert_eq!(result.parse.tree.rowan_root().text().to_string(), input);
        assert_eq!(
            result.parse.diagnostics, repeated.parse.diagnostics,
            "compiler parse diagnostics are non-deterministic for generated seed {seed}"
        );
        assert_eq!(
            result.analysis.diagnostics, repeated.analysis.diagnostics,
            "compiler semantic diagnostics are non-deterministic for generated seed {seed}"
        );
        for diagnostic in result
            .parse
            .diagnostics
            .iter()
            .chain(&result.analysis.diagnostics)
        {
            let start = diagnostic.span.start as usize;
            let end = diagnostic.span.end as usize;
            assert!(
                start <= end && end <= input.len(),
                "compiler diagnostic {} has out-of-bounds span {}..{} for generated seed {seed}",
                diagnostic.code,
                diagnostic.span.start,
                diagnostic.span.end
            );
            assert!(
                input.is_char_boundary(start) && input.is_char_boundary(end),
                "compiler diagnostic {} splits UTF-8 at {}..{} for generated seed {seed}",
                diagnostic.code,
                diagnostic.span.start,
                diagnostic.span.end
            );
        }
    }
}

#[test]
fn compiler_preserves_graph_filter_and_projection_vertical_slice() {
    let source = "MATCH (a)-[:CALLS]->(b) WHERE a = 1 RETURN b";
    let compiler = Compiler;
    let result = compiler.compile("graph-filter.gql", source, &catalog());

    assert!(
        result.parse.diagnostics.is_empty(),
        "parse: {:?}",
        result.parse.diagnostics
    );
    assert!(
        result.analysis.diagnostics.is_empty(),
        "sema: {:?}",
        result.analysis.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    let ir = result.analysis.ir.expect("graph query should produce IR");
    assert_eq!(
        ir.matches
            .into_iter()
            .next()
            .expect("graph match")
            .paths
            .into_iter()
            .next()
            .expect("path pattern")
            .elements
            .len(),
        3
    );
    assert_eq!(ir.filters.len(), 1);
    assert_eq!(ir.projection[0].expression, Expression::Binding("B".into()));
}

#[test]
fn compiler_reports_invalid_return_while_preserving_rowan_source() {
    let source = "MATCH (a) RETURN";
    let compiler = Compiler;
    let result = compiler.compile("invalid-return.gql", source, &catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result
            .analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-PARSE-RETURN-SYNTAX")
    );
}

#[test]
fn compiler_rejects_reserved_but_unsupported_statements_without_ir() {
    let grammar = gql_syntax::grammar_projection_receipt();
    assert_eq!(grammar.schema, "mrr.gerbil-grammar-projection.v1");
    for source in ["CREATE INDEX social", "DROP INDEX social"] {
        let result = Compiler.compile("unsupported-statement.gql", source, &catalog());
        assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
        assert!(
            result
                .analysis
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.code == "GQL-PARSE-UNSUPPORTED-STATEMENT" }),
            "expected unsupported-statement diagnostic for {source:?}: {:?}",
            result.analysis.diagnostics
        );
        assert!(
            result.analysis.ir.is_none(),
            "unsupported statement must not produce IR: {source:?}"
        );
    }
}

#[test]
fn compiler_rejects_reserved_keywords_in_expression_positions_without_ir() {
    for source in [
        "MATCH (n) RETURN CALL",
        "MATCH (n) WHERE CREATE RETURN n",
        "MATCH (n) LET value = DROP RETURN value",
        "MATCH (n) RETURN [INSERT, DELETE, SET, REMOVE]",
    ] {
        let result = Compiler.compile("unsupported-keyword-expression.gql", source, &catalog());
        assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
        assert!(
            result.analysis.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "GQL-PARSE-UNSUPPORTED-KEYWORD-EXPRESSION"
            }),
            "expected unsupported-keyword-expression diagnostic for {source:?}: {:?}",
            result.analysis.diagnostics
        );
        assert!(
            result.analysis.ir.is_none(),
            "unsupported keyword must not produce IR: {source:?}"
        );
    }
}

#[test]
fn compiler_lowers_return_projection_alias_to_canonical_ir() {
    let source = "MATCH (n) RETURN n AS person";
    let result = Compiler.compile("return-alias.gql", source, &catalog());
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse: {:?}",
        result.parse.diagnostics
    );
    assert!(
        result.analysis.diagnostics.is_empty(),
        "sema: {:?}",
        result.analysis.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(
        result.analysis.ir.expect("alias IR").projection[0]
            .alias
            .as_deref(),
        Some("PERSON")
    );
}

#[test]
fn compiler_lowers_union_to_independent_canonical_ir_branches() {
    let source = "MATCH (a) RETURN a UNION MATCH (b) RETURN b";
    let result = Compiler.compile("union.gql", source, &catalog());
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse: {:?}",
        result.parse.diagnostics
    );
    assert!(
        result.analysis.diagnostics.is_empty(),
        "sema: {:?}",
        result.analysis.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    let ir = result.analysis.ir.expect("UNION IR");
    assert_eq!(ir.set_operations.len(), 1);
    assert_eq!(
        ir.set_operations[0].right.projection[0].expression,
        Expression::Binding("B".into())
    );
}

#[test]
fn compiler_lowers_limit_to_canonical_ir() {
    let source = "MATCH (n) RETURN n LIMIT 10";
    let result = Compiler.compile("limit.gql", source, &catalog());
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse: {:?}",
        result.parse.diagnostics
    );
    assert!(
        result.analysis.diagnostics.is_empty(),
        "sema: {:?}",
        result.analysis.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(
        result.analysis.ir.expect("LIMIT IR").limit,
        Some(gql_ir::NonNegativeIntegerSpecification::Literal(10))
    );
}

#[test]
fn compiler_lowers_order_by_to_canonical_ir() {
    let source = "MATCH (n) RETURN n ORDER BY n DESC LIMIT 10";
    let result = Compiler.compile("order-by.gql", source, &catalog());
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse: {:?}",
        result.parse.diagnostics
    );
    assert!(
        result.analysis.diagnostics.is_empty(),
        "sema: {:?}",
        result.analysis.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(
        result.analysis.ir.expect("ORDER BY IR").order_by[0].direction,
        gql_ir::SortDirection::Descending
    );
}

#[test]
fn compiler_lowers_offset_to_canonical_ir() {
    let source = "MATCH (n) RETURN n ORDER BY n LIMIT 10 OFFSET 2";
    let result = Compiler.compile("offset.gql", source, &catalog());
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse: {:?}",
        result.parse.diagnostics
    );
    assert!(
        result.analysis.diagnostics.is_empty(),
        "sema: {:?}",
        result.analysis.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(
        result.analysis.ir.expect("OFFSET IR").offset,
        Some(gql_ir::NonNegativeIntegerSpecification::Literal(2))
    );
}
