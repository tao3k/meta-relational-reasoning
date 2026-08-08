use crate::{parse, SyntaxKind, TokenKind};
use gql_source::Diagnostic;

#[test]
fn preserves_source_and_unicode_identifiers() {
    let input = "MATCH (node)-[:CALLS]->(target) RETURN node";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert!(
        parsed.diagnostics.is_empty(),
        "diagnostics: {:?}",
        parsed.diagnostics
    );
    assert_eq!(parsed.tree.root().kind(), SyntaxKind::SourceFile);
    assert_eq!(parsed.tree.rowan_root().text().to_string(), input);
}

#[test]
fn preserves_whitespace_and_unknown_tokens() {
    let input = "MATCH  a ## bad\n/x @";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert!(
        parsed
            .tree
            .tokens()
            .iter()
            .any(|token| token.kind == TokenKind::Comment)
    );
    assert!(
        parsed
            .tree
            .tokens()
            .iter()
            .any(|token| token.kind == TokenKind::Whitespace)
    );
    assert!(
        parsed
            .tree
            .tokens()
            .iter()
            .any(|token| token.kind == TokenKind::Unknown)
    );
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
fn parses_where_clause_with_identifier_expression() {
    let input = "MATCH (a)-[:CALLS]->(b) WHERE a RETURN b";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert!(parsed.diagnostics.is_empty(), "diagnostics: {:?}", parsed.diagnostics);
    assert_eq!(parsed.tree.root().kind(), SyntaxKind::SourceFile);
}

#[test]
fn parses_where_clause_with_boolean_and_comparison_expression() {
    let input = "MATCH (a)-[:CALLS]->(b) WHERE a = 1 AND NOT (b != 2 OR a >= 3) RETURN b";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert!(parsed.diagnostics.is_empty(), "diagnostics: {:?}", parsed.diagnostics);
    assert_eq!(parsed.tree.root().kind(), SyntaxKind::SourceFile);
}

#[test]
fn parses_where_clause_with_nested_parentheses() {
    let input = "MATCH (a)-[:CALLS]->(b) WHERE (a = 1) AND (b = 2 OR a < 3) RETURN b";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert!(parsed.diagnostics.is_empty(), "diagnostics: {:?}", parsed.diagnostics);
    assert_eq!(parsed.tree.root().kind(), SyntaxKind::SourceFile);
}

#[test]
fn parses_let_clause_with_simple_binding() {
    let input = "LET a = 1";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert!(parsed.diagnostics.is_empty(), "diagnostics: {:?}", parsed.diagnostics);
    assert_eq!(parsed.tree.root().kind(), SyntaxKind::SourceFile);
}

#[test]
fn parses_incoming_edge_pattern() {
    let input = "MATCH (a)<-[:CALLS]-(b) RETURN a";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert!(parsed.diagnostics.is_empty(), "diagnostics: {:?}", parsed.diagnostics);
    assert_eq!(parsed.tree.root().kind(), SyntaxKind::SourceFile);
}

#[test]
fn parses_labeled_outgoing_edge_pattern() {
    let input = "MATCH (a)-[:CALLS:Person]->(b) RETURN a";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert!(parsed.diagnostics.is_empty(), "diagnostics: {:?}", parsed.diagnostics);
    assert_eq!(parsed.tree.root().kind(), SyntaxKind::SourceFile);
}

#[test]
fn parses_labeled_incoming_edge_pattern() {
    let input = "MATCH (a)<-[:CALLS:Person]-(b) RETURN a";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert!(parsed.diagnostics.is_empty(), "diagnostics: {:?}", parsed.diagnostics);
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
        "diagnostics: {:?}",
        parsed.diagnostics
    );
    assert_eq!(parsed.tree.root().kind(), SyntaxKind::SourceFile);
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
fn does_not_treat_backend_keyword_as_gql_keyword() {
    let input = "MATCH (a) RETURN ascent";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert!(parsed.diagnostics.is_empty(), "diagnostics: {:?}", parsed.diagnostics);
    assert!(
        parsed
            .tree
            .tokens()
            .iter()
            .any(|token| token.kind == TokenKind::Identifier),
        "expected ascent to remain an identifier"
    );
    assert_eq!(parsed.tree.root().kind(), SyntaxKind::SourceFile);
}

#[test]
fn parse_keeps_invalid_let_clause_syntax_recoverable() {
    let input = "LET = 1";
    let parsed = parse("test.gql", input);
    assert_eq!(parsed.tree.source().text(), input);
    assert!(parsed.diagnostics.is_empty(), "diagnostics: {:?}", parsed.diagnostics);
}
