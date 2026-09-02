#[test]
fn parses_where_clause_with_identifier_expression() {
    let input = "MATCH (a)-[:CALLS]->(b) WHERE a RETURN b";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.tree.root().kind(), SyntaxKind::SourceFile);
}

#[test]
fn parses_where_clause_with_boolean_and_comparison_expression() {
    let input = "MATCH (a)-[:CALLS]->(b) WHERE a = 1 AND NOT (b <> 2 OR a >= 3) RETURN b";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.tree.root().kind(), SyntaxKind::SourceFile);
}

#[test]
fn parses_where_clause_with_nested_parentheses() {
    let input = "MATCH (a)-[:CALLS]->(b) WHERE (a = 1) AND (b = 2 OR a < 3) RETURN b";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.tree.root().kind(), SyntaxKind::SourceFile);
}

#[test]
fn parses_let_clause_with_simple_binding() {
    let input = "LET a = 1";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.tree.root().kind(), SyntaxKind::SourceFile);
}

#[test]
fn parses_incoming_edge_pattern() {
    let input = "MATCH (a)<-[:CALLS]-(b) RETURN a";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.tree.root().kind(), SyntaxKind::SourceFile);
}

#[test]
fn parses_labeled_outgoing_edge_pattern() {
    let input = "MATCH (a)-[:CALLS:Person]->(b) RETURN a";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.tree.root().kind(), SyntaxKind::SourceFile);
}

#[test]
fn parses_labeled_incoming_edge_pattern() {
    let input = "MATCH (a)<-[:CALLS:Person]-(b) RETURN a";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.tree.root().kind(), SyntaxKind::SourceFile);
}

#[test]
fn recovers_from_malformed_edge_label_list_with_colon() {
    let input = "MATCH (a)-[:CALLS:Person->(b) RETURN a";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert_eq!(parsed.tree.rowan_root().text().to_string(), input);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-PARSE-MATCH-SYNTAX"),
        "{:?}",
        parsed.diagnostics
    );
}

#[test]
fn recovers_where_clause_missing_expression() {
    let input = "MATCH (a)-[:CALLS]->(b) WHERE";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|diagnostic: &Diagnostic| diagnostic.code == "GQL-PARSE-WHERE-SYNTAX")
    );
}

#[test]
fn backend_names_remain_identifiers_and_never_activate_core_syntax() {
    for backend_name in [
        "ascent",
        "gql_ascent",
        "gql_reasoning",
        "agent_semantic_protocols",
        "duckdb",
        "graph_turbo",
        "turso",
        "wendao",
    ] {
        let input = format!("MATCH (a) RETURN {backend_name}");
        let parsed = parse("backend-name.gql", &input);
        assert_eq!(parsed.tree.source().text(), input);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert!(
            parsed
                .tree
                .tokens()
                .iter()
                .any(|token| token.text() == backend_name && token.kind == TokenKind::Identifier)
        );
    }
}

#[test]
fn reserved_but_unsupported_statements_report_profile_diagnostics_losslessly() {
    for source in ["CREATE INDEX social", "DROP INDEX social"] {
        let parsed = parse("unsupported-statement.gql", source);
        assert_eq!(parsed.tree.rowan_root().text().to_string(), source);
        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "GQL-PARSE-UNSUPPORTED-STATEMENT"),
            "{:?}",
            parsed.diagnostics
        );
    }
}

#[test]
fn reserved_keywords_are_rejected_in_expression_positions_losslessly() {
    for source in [
        "MATCH (n) RETURN CALL",
        "MATCH (n) WHERE CREATE RETURN n",
        "MATCH (n) LET value = DROP RETURN value",
        "MATCH (n) RETURN [INSERT, DELETE, SET, REMOVE]",
    ] {
        let parsed = parse("unsupported-keyword-expression.gql", source);
        assert_eq!(parsed.tree.rowan_root().text().to_string(), source);
        assert!(
            parsed.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "GQL-PARSE-UNSUPPORTED-KEYWORD-EXPRESSION"
            }),
            "{:?}",
            parsed.diagnostics
        );
    }
}

#[test]
fn parse_keeps_invalid_let_clause_syntax_recoverable() {
    let input = "LET = 1";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
}
