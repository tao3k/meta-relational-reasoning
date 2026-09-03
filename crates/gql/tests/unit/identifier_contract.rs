use crate::Compiler;
use crate::ast::{
    Expression as AstExpression, ParameterNameForm, PatternElement, QueryClause, Statement,
};
use crate::catalog::{Catalog, CatalogName};
use crate::ir::{Expression as IrExpression, GraphPatternElement};
use crate::syntax::SyntaxKind;

fn empty_catalog() -> Catalog {
    Catalog::new(
        CatalogName("identifier-contract".into()),
        Vec::new(),
        Vec::new(),
    )
}

#[test]
fn delimited_identifiers_are_lossless_and_normalized_through_canonical_ir() {
    for (source, source_name, logical_name) in [
        ("MATCH (n {\"MATCH\": 1}) RETURN n", "\"MATCH\"", "MATCH"),
        (
            "MATCH (n {\"say\"\"hi\": 2}) RETURN n",
            "\"say\"\"hi\"",
            "say\"hi",
        ),
    ] {
        let result = Compiler.compile("delimited-identifier.gql", source, &empty_catalog());
        assert!(
            result.parse.diagnostics.is_empty(),
            "parse diagnostics for {source:?}: {:?}",
            result.parse.diagnostics
        );
        assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);

        let Some(Statement::Query(query)) = &result.statement else {
            panic!("delimited identifier source must remain a query");
        };
        let Some(QueryClause::Match(match_clause)) = query.clauses.first() else {
            panic!("MATCH clause exists");
        };
        let Some(PatternElement::Node(node)) = match_clause.patterns[0].elements.first() else {
            panic!("node pattern exists");
        };
        let property = &node.properties[0];
        assert_eq!(property.key.text, logical_name);
        assert_eq!(
            &source[property.key.span.start as usize..property.key.span.end as usize],
            source_name
        );

        assert!(
            result.analysis.diagnostics.is_empty(),
            "semantic diagnostics for {source:?}: {:?}",
            result.analysis.diagnostics
        );
        let ir = result.analysis.ir.expect("canonical identifier IR");
        let Some(GraphPatternElement::Node(node)) = ir
            .matches
            .into_iter()
            .next()
            .expect("graph match")
            .paths
            .into_iter()
            .next()
            .expect("path pattern")
            .elements
            .into_iter()
            .next()
        else {
            panic!("canonical node pattern exists");
        };
        assert_eq!(node.properties[0].key, logical_name);
    }
}

#[test]
fn grave_accent_identifiers_preserve_escape_and_span_through_ir() {
    let source = "MATCH (n {`MATCH`: 1, `say``hi`: 2}) RETURN n";
    let result = Compiler.compile("grave-accent-identifier.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse diagnostics: {:?}",
        result.parse.diagnostics
    );
    let Some(Statement::Query(query)) = &result.statement else {
        panic!("delimited property source must remain a query");
    };
    let Some(QueryClause::Match(match_clause)) = query.clauses.first() else {
        panic!("MATCH clause exists");
    };
    let Some(PatternElement::Node(node)) = match_clause.patterns[0].elements.first() else {
        panic!("node pattern exists");
    };
    assert_eq!(
        node.properties
            .iter()
            .map(|property| property.key.text.as_str())
            .collect::<Vec<_>>(),
        ["MATCH", "say`hi"]
    );
    assert_eq!(
        &source
            [node.properties[1].key.span.start as usize..node.properties[1].key.span.end as usize],
        "`say``hi`"
    );

    let ir = result.analysis.ir.expect("canonical identifier IR");
    let Some(GraphPatternElement::Node(node)) = ir.matches[0].paths[0].elements.first() else {
        panic!("canonical node pattern exists");
    };
    assert_eq!(
        node.properties
            .iter()
            .map(|property| property.key.as_str())
            .collect::<Vec<_>>(),
        ["MATCH", "say`hi"]
    );
}

#[test]
fn identifier_escapes_and_no_escape_prefix_have_distinct_values() {
    let source = r#"MATCH (n {`\u0041`: 1, @`raw\n`: 2}) RETURN n"#;
    let result = Compiler.compile("identifier-escape.gql", source, &empty_catalog());

    assert!(
        result.parse.diagnostics.is_empty(),
        "parse diagnostics: {:?}",
        result.parse.diagnostics
    );
    let Some(Statement::Query(query)) = &result.statement else {
        panic!("identifier escape source must remain a query");
    };
    let Some(QueryClause::Match(match_clause)) = query.clauses.first() else {
        panic!("MATCH clause exists");
    };
    let Some(PatternElement::Node(node)) = match_clause.patterns[0].elements.first() else {
        panic!("node pattern exists");
    };
    assert_eq!(node.properties[0].key.text, "A");
    assert_eq!(node.properties[1].key.text, r"raw\n");
}

#[test]
fn malformed_identifier_escape_is_typed_and_emits_no_ir() {
    let source = r#"MATCH (n {`\q`: 1}) RETURN n"#;
    let result = Compiler.compile("invalid-identifier-escape.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(
        result
            .parse
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-SYNTAX-INVALID-IDENTIFIER-ESCAPE"]
    );
    assert!(result.analysis.ir.is_none());
}

#[test]
fn unicode_identifier_uses_iso_id_start_pc_and_id_continue() {
    let source = "MATCH (\u{203f}e\u{0301}) RETURN \u{203f}e\u{0301}";
    let result = Compiler.compile(
        "unicode-identifier-categories.gql",
        source,
        &empty_catalog(),
    );

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse diagnostics: {:?}",
        result.parse.diagnostics
    );
    assert!(result.analysis.diagnostics.is_empty());
    let ir = result.analysis.ir.expect("canonical Unicode identifier IR");
    assert_eq!(
        ir.projection[0].expression,
        IrExpression::Binding("\u{203f}E\u{0301}".into())
    );
}

#[test]
fn every_iso_non_reserved_word_is_a_regular_binding_identifier() {
    const NON_RESERVED: [&str; 47] = [
        "ACYCLIC",
        "BINDING",
        "BINDINGS",
        "CONNECTING",
        "DESTINATION",
        "DIFFERENT",
        "DIRECTED",
        "EDGE",
        "EDGES",
        "ELEMENT",
        "ELEMENTS",
        "FIRST",
        "GRAPH",
        "GROUPS",
        "KEEP",
        "LABEL",
        "LABELED",
        "LABELS",
        "LAST",
        "NFC",
        "NFD",
        "NFKC",
        "NFKD",
        "NO",
        "NODE",
        "NORMALIZED",
        "ONLY",
        "ORDINALITY",
        "PROPERTY",
        "READ",
        "RELATIONSHIP",
        "RELATIONSHIPS",
        "REPEATABLE",
        "SHORTEST",
        "SIMPLE",
        "SOURCE",
        "TABLE",
        "TO",
        "TRAIL",
        "TRANSACTION",
        "TYPE",
        "UNDIRECTED",
        "VERTEX",
        "WALK",
        "WITHOUT",
        "WRITE",
        "ZONE",
    ];

    for word in NON_RESERVED {
        let source = format!("MATCH ({word}) RETURN {word}");
        let result = Compiler.compile("non-reserved-binding.gql", &source, &empty_catalog());
        assert!(
            result.parse.diagnostics.is_empty(),
            "{word} must remain a regular identifier: {:?}",
            result.parse.diagnostics
        );
        assert!(
            result.analysis.diagnostics.is_empty(),
            "{word} semantic diagnostics: {:?}",
            result.analysis.diagnostics
        );
        assert!(result.analysis.ir.is_some(), "{word} must emit IR");
    }
}

#[test]
fn delimited_identifier_is_rejected_as_a_binding_variable() {
    let source = "MATCH (`node`) RETURN 1";
    let result = Compiler.compile("delimited-binding-variable.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(
        result
            .parse
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-PARSE-BINDING-VARIABLE-SYNTAX"]
    );
    assert!(result.analysis.ir.is_none());
}

#[test]
fn delimited_identifier_is_rejected_as_an_edge_binding_variable() {
    let source = "MATCH ()-[`edge`]->() RETURN 1";
    let result = Compiler.compile("delimited-edge-binding.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(
        result
            .parse
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        ["GQL-PARSE-BINDING-VARIABLE-SYNTAX"]
    );
    assert!(result.analysis.ir.is_none());
}

#[test]
fn alphabetic_codepoint_outside_iso_id_start_is_rejected() {
    let source = "MATCH (\u{24e7}) RETURN 1";
    let result = Compiler.compile("invalid-id-start.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(result.parse.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "GQL-SYNTAX-UNKNOWN-CHARACTER"
            && &source[diagnostic.span.start as usize..diagnostic.span.end as usize] == "\u{24e7}"
    }));
    assert!(result.analysis.ir.is_none());
}

#[test]
fn unterminated_delimited_identifier_is_typed_and_emits_no_ir() {
    let source = "MATCH (\"node) RETURN node";
    let result = Compiler.compile(
        "unterminated-delimited-identifier.gql",
        source,
        &empty_catalog(),
    );

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(result.analysis.ir.is_none());
    assert_eq!(
        result
            .parse
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == "GQL-SYNTAX-UNTERMINATED-DELIMITED-IDENTIFIER"
            })
            .count(),
        1,
        "the lexer must emit exactly one typed unterminated-identifier diagnostic"
    );
}

#[test]
fn undelimited_identifiers_use_one_canonical_name_through_ir() {
    let source = "MATCH (Node {Name: 1}) RETURN nOdE";
    let result = Compiler.compile(
        "undelimited-identifier-folding.gql",
        source,
        &empty_catalog(),
    );

    assert!(
        result.parse.diagnostics.is_empty(),
        "parse diagnostics: {:?}",
        result.parse.diagnostics
    );
    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);

    let Some(Statement::Query(query)) = &result.statement else {
        panic!("undelimited identifier source must remain a query");
    };
    let Some(QueryClause::Match(match_clause)) = query.clauses.first() else {
        panic!("MATCH clause exists");
    };
    let Some(PatternElement::Node(node)) = match_clause.patterns[0].elements.first() else {
        panic!("node pattern exists");
    };
    let binding = node.binding.as_ref().expect("node binding");
    assert_eq!(binding.text, "Node");
    assert_eq!(
        &source[binding.span.start as usize..binding.span.end as usize],
        "Node"
    );
    assert_eq!(node.properties[0].key.text, "Name");

    assert!(
        result.analysis.diagnostics.is_empty(),
        "semantic diagnostics: {:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("canonical identifier IR");
    let Some(GraphPatternElement::Node(node)) = ir
        .matches
        .into_iter()
        .next()
        .expect("graph match")
        .paths
        .into_iter()
        .next()
        .expect("path pattern")
        .elements
        .into_iter()
        .next()
    else {
        panic!("canonical node pattern exists");
    };
    assert_eq!(node.binding.as_deref(), Some("NODE"));
    assert_eq!(node.properties[0].key, "NAME");
    assert_eq!(
        ir.projection[0].expression,
        IrExpression::Binding("NODE".to_owned())
    );
}

#[test]
fn delimited_and_undelimited_identifiers_do_not_collapse_case() {
    let source = "MATCH (n {\"Node\": 1, node: 2}) RETURN n";
    let result = Compiler.compile("identifier-form-equality.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(result.analysis.diagnostics.is_empty());
    let ir = result.analysis.ir.expect("distinct property identities");
    let Some(GraphPatternElement::Node(node)) = ir.matches[0].paths[0].elements.first() else {
        panic!("canonical node pattern exists");
    };
    assert_eq!(node.properties[0].key, "Node");
    assert_eq!(node.properties[1].key, "NODE");
}

#[test]
fn case_equivalent_undelimited_property_keys_are_duplicates() {
    let source = "MATCH (n {Name: 1, nAmE: 2}) RETURN n";
    let result = Compiler.compile("identifier-property-equality.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(result.analysis.ir.is_none());
    assert_eq!(
        result
            .analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| { diagnostic.code == "GQL-SEMA-DUPLICATE-PATTERN-PROPERTY" })
            .count(),
        1
    );
}

#[test]
fn unicode_undelimited_identifier_folding_reaches_one_ir_identity() {
    let source = "MATCH (stra\u{00df}e) RETURN STRASSE";
    let result = Compiler.compile("unicode-identifier-folding.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.analysis.diagnostics.is_empty(),
        "semantic diagnostics: {:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("Unicode canonical identifier IR");
    let Some(GraphPatternElement::Node(node)) = ir
        .matches
        .into_iter()
        .next()
        .expect("graph match")
        .paths
        .into_iter()
        .next()
        .expect("path pattern")
        .elements
        .into_iter()
        .next()
    else {
        panic!("canonical node pattern exists");
    };
    assert_eq!(node.binding.as_deref(), Some("STRASSE"));
    assert_eq!(
        ir.projection[0].expression,
        IrExpression::Binding("STRASSE".to_owned())
    );
}

#[test]
fn dynamic_parameter_specifications_cross_lossless_cst_ast_and_canonical_ir() {
    let source = r#"MATCH (n {named: $limit, ordinal: $42, quoted: $"MATCH", accent: $`say``hi`}) RETURN $limit"#;
    let result = Compiler.compile(
        "dynamic-parameter-specification.gql",
        source,
        &empty_catalog(),
    );

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse diagnostics: {:?}",
        result.parse.diagnostics
    );
    assert!(
        result.analysis.diagnostics.is_empty(),
        "semantic diagnostics: {:?}",
        result.analysis.diagnostics
    );
    assert_eq!(
        result
            .parse
            .tree
            .rowan_root()
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::DynamicParameterExpression)
            .count(),
        5
    );
    let Some(Statement::Query(query)) = &result.statement else {
        panic!("dynamic parameter source must remain a query");
    };
    let Some(QueryClause::Match(match_clause)) = query.clauses.first() else {
        panic!("MATCH clause exists");
    };
    let Some(PatternElement::Node(node)) = match_clause.patterns[0].elements.first() else {
        panic!("node pattern exists");
    };
    let parameters = node
        .properties
        .iter()
        .map(|property| match &property.value {
            AstExpression::Parameter(parameter) => parameter,
            other => panic!("property value must be a parameter, got {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        parameters
            .iter()
            .map(|parameter| (parameter.name.as_str(), parameter.form))
            .collect::<Vec<_>>(),
        [
            ("limit", ParameterNameForm::Extended),
            ("42", ParameterNameForm::Extended),
            ("MATCH", ParameterNameForm::Delimited),
            ("say`hi", ParameterNameForm::Delimited),
        ]
    );
    assert_eq!(
        parameters
            .iter()
            .map(|parameter| {
                &source[parameter.span.start as usize..parameter.span.end as usize]
            })
            .collect::<Vec<_>>(),
        ["$limit", "$42", "$\"MATCH\"", "$`say``hi`"]
    );

    let ir = result
        .analysis
        .ir
        .expect("dynamic parameters remain first-class backend-neutral IR");
    let Some(GraphPatternElement::Node(node)) = ir.matches[0].paths[0].elements.first() else {
        panic!("canonical node pattern exists");
    };
    assert_eq!(
        node.properties
            .iter()
            .map(|property| &property.value)
            .collect::<Vec<_>>(),
        [
            &IrExpression::Parameter("limit".into()),
            &IrExpression::Parameter("42".into()),
            &IrExpression::Parameter("MATCH".into()),
            &IrExpression::Parameter("say`hi".into()),
        ]
    );
    assert_eq!(
        ir.projection[0].expression,
        IrExpression::Parameter("limit".into())
    );
}

#[test]
fn malformed_or_context_invalid_parameter_references_fail_closed_once() {
    for (source, expected) in [
        ("RETURN $", "GQL-SYNTAX-INVALID-DYNAMIC-PARAMETER"),
        (
            "RETURN $$catalog_graph",
            "GQL-PARSE-SUBSTITUTED-PARAMETER-CONTEXT",
        ),
        (
            r#"RETURN $"unterminated"#,
            "GQL-SYNTAX-INVALID-DYNAMIC-PARAMETER",
        ),
    ] {
        let result = Compiler.compile("invalid-parameter-reference.gql", source, &empty_catalog());

        assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
        assert_eq!(
            result
                .parse
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect::<Vec<_>>(),
            [expected],
            "{source:?} must emit exactly one typed terminal"
        );
        assert!(result.analysis.ir.is_none());
    }
}
