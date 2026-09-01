#[test]
fn preserves_empty_input_source() {
    for input in [
        "",
        "   \n\t  ",
        "# just a comment\n",
        "MATCH (a) RETURN a\n",
    ] {
        let parsed = parse("test.gql", input);
        assert_eq!(parsed.tree.source().text(), input);
        assert_eq!(parsed.tree.rowan_root().text().to_string(), input);
    }
}

#[test]
fn parses_scalar_literals_and_property_access_as_structural_nodes() {
    let input = "MATCH (n) WHERE n.name = TRUE RETURN n.name";
    let parsed = parse("test.gql", input);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert!(contains_node_kind(
        &parsed.tree.root(),
        SyntaxKind::PropertyAccessExpression
    ));
    assert!(contains_node_kind(
        &parsed.tree.root(),
        SyntaxKind::LiteralExpression
    ));
    assert_eq!(parsed.tree.rowan_root().text().to_string(), input);
}

#[test]
fn recovers_property_access_without_an_identifier() {
    let input = "MATCH (n) WHERE n. RETURN n";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.rowan_root().text().to_string(), input);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-PARSE-EXPRESSION-SYNTAX")
    );
}

#[test]
fn preserves_decimal_literal_as_one_number_token() {
    let input = "MATCH (n) WHERE n.score = 1.25 RETURN n";
    let parsed = parse("test.gql", input);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert!(
        parsed
            .tree
            .tokens()
            .iter()
            .any(|token| token.kind == TokenKind::Number && token.text() == "1.25")
    );
}

#[test]
fn parses_named_path_pattern_with_lossless_rowan_structure() {
    let input = "MATCH p = (a)-[:CALLS]->(b) RETURN p";
    let parsed = parse("test.gql", input);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert!(contains_node_kind(
        &parsed.tree.root(),
        SyntaxKind::PathPattern
    ));
    assert_eq!(parsed.tree.rowan_root().text().to_string(), input);
}

#[test]
fn recovers_named_path_pattern_without_graph_pattern() {
    let parsed = parse("test.gql", "MATCH p = RETURN p");
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-PARSE-PATH-SYNTAX")
    );
}

#[test]
fn parses_bounded_path_quantifier_as_a_structural_node() {
    let input = "MATCH (a)-[:CALLS]->{1,3}(b) RETURN b";
    let parsed = parse("test.gql", input);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert!(contains_node_kind(
        &parsed.tree.root(),
        SyntaxKind::PathQuantifier
    ));
}

#[test]
fn rejects_zero_minimum_path_quantifier() {
    let parsed = parse("test.gql", "MATCH (a)-[:CALLS]->{0}(b) RETURN b");
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-PARSE-PATH-QUANTIFIER")
    );
}

#[test]
fn parses_optional_match_as_a_structural_clause() {
    let input = "MATCH (a) OPTIONAL MATCH (a)-[:CALLS]->(b) RETURN a, b";
    let parsed = parse("test.gql", input);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert!(contains_node_kind(
        &parsed.tree.root(),
        SyntaxKind::OptionalMatchClause
    ));
}

#[test]
fn parses_arithmetic_expression_with_precedence_nodes() {
    let input = "MATCH (n) RETURN 1 + 2 * 3";
    let parsed = parse("test.gql", input);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert!(contains_node_kind(
        &parsed.tree.root(),
        SyntaxKind::BinaryExpression
    ));
    assert_eq!(parsed.tree.rowan_root().text().to_string(), input);
}

#[test]
fn parses_division_and_modulo_in_multiplicative_layer() {
    let input = "MATCH (n) RETURN 8 / 2 % 3";
    let parsed = parse("test.gql", input);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.tree.rowan_root().text().to_string(), input);
}

#[test]
fn parses_list_value_as_structural_node() {
    let input = "MATCH (n) RETURN [1, 2, [3]]";
    let parsed = parse("list-value.gql", input);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert!(contains_node_kind(
        &parsed.tree.root(),
        SyntaxKind::ListExpression
    ));
}

#[test]
fn parses_collection_subscript_as_structural_node() {
    let input = "MATCH (n) LET values = [1, 2] RETURN values[0]";
    let parsed = parse("list-subscript.gql", input);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert!(contains_node_kind(
        &parsed.tree.root(),
        SyntaxKind::SubscriptExpression
    ));
}

#[test]
fn parses_in_collection_predicate_as_binary_expression() {
    let input = "MATCH (n) RETURN 1 IN [1, 2]";
    let parsed = parse("list-membership.gql", input);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert!(contains_node_kind(
        &parsed.tree.root(),
        SyntaxKind::BinaryExpression
    ));
}
