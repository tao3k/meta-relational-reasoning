#[test]
fn preserves_whitespace_and_unknown_tokens() {
    let input = "MATCH  a ## bad\n/x @";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    for kind in [
        TokenKind::Comment,
        TokenKind::Whitespace,
        TokenKind::Unknown,
    ] {
        assert!(parsed.tree.tokens().iter().any(|token| token.kind == kind));
    }
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|diagnostic: &Diagnostic| diagnostic.code == "GQL-SYNTAX-UNKNOWN-CHARACTER")
    );
}

#[test]
fn recovers_from_unterminated_string() {
    let input = "'unterminated";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-SYNTAX-UNTERMINATED-STRING")
    );
}

#[test]
fn recovers_invalid_return_expression() {
    let input = "MATCH (a)-[:CALLS]->(b) RETURN";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|diagnostic: &Diagnostic| diagnostic.code == "GQL-PARSE-RETURN-SYNTAX")
    );
}

#[test]
fn parses_return_projection_alias_losslessly() {
    let input = "MATCH (n) RETURN n AS person";
    let parsed = parse("test.gql", input);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.tree.rowan_root().text().to_string(), input);
    assert!(contains_node_kind(
        &parsed.tree.root(),
        SyntaxKind::ProjectionAlias
    ));
}

#[test]
fn recovers_return_alias_without_identifier_losslessly() {
    let input = "MATCH (n) RETURN n AS";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.rowan_root().text().to_string(), input);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-PARSE-RETURN-ALIAS-SYNTAX")
    );
}

#[test]
fn parses_union_query_boundary_losslessly() {
    let input = "MATCH (a) RETURN a UNION MATCH (b) RETURN b";
    let parsed = parse("test.gql", input);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.tree.rowan_root().text().to_string(), input);
    assert!(contains_node_kind(
        &parsed.tree.root(),
        SyntaxKind::UnionClause
    ));
}

#[test]
fn parses_limit_clause_losslessly() {
    let input = "MATCH (n) RETURN n LIMIT 10";
    let parsed = parse("test.gql", input);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.tree.rowan_root().text().to_string(), input);
    assert!(contains_node_kind(
        &parsed.tree.root(),
        SyntaxKind::LimitClause
    ));
}

#[test]
fn recovers_limit_without_integer_losslessly() {
    let input = "MATCH (n) RETURN n LIMIT";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.rowan_root().text().to_string(), input);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-PARSE-LIMIT-SYNTAX")
    );
}

#[test]
fn parses_order_by_clause_with_directions_losslessly() {
    let input = "MATCH (n) RETURN n ORDER BY n DESC, n.name ASC LIMIT 10";
    let parsed = parse("test.gql", input);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.tree.rowan_root().text().to_string(), input);
    assert!(contains_node_kind(
        &parsed.tree.root(),
        SyntaxKind::OrderByClause
    ));
}

#[test]
fn recovers_order_by_without_expression_losslessly() {
    let input = "MATCH (n) RETURN n ORDER BY LIMIT 10";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.rowan_root().text().to_string(), input);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-PARSE-ORDER-BY-SYNTAX")
    );
}

#[test]
fn parses_offset_clause_losslessly() {
    let input = "MATCH (n) RETURN n ORDER BY n LIMIT 10 OFFSET 0";
    let parsed = parse("test.gql", input);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.tree.rowan_root().text().to_string(), input);
    assert!(contains_node_kind(
        &parsed.tree.root(),
        SyntaxKind::OffsetClause
    ));
}

#[test]
fn recovers_offset_without_integer_losslessly() {
    let input = "MATCH (n) RETURN n LIMIT 10 OFFSET";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.rowan_root().text().to_string(), input);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "GQL-PARSE-OFFSET-SYNTAX")
    );
}
