use crate::native::NativeGrammar;
use crate::{load_reasoning_bundle, stamp_projection, validate_projection};
use gerbil_scheme_sys::GerbilStatus;
use mrr_bundle::{LineagePolicy, ProjectionPolicy, ValidationProfile};
use std::sync::{Arc, Barrier};
use std::{collections::BTreeMap, collections::BTreeSet, path::Path};

const INPUT: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const TEST_GRAMMAR_DIGEST: &str =
    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn native_runtime_status_preserves_known_meaning_and_unknown_code() {
    let unavailable = crate::NativeRuntimeStatus::from_code(4);
    assert_eq!(unavailable.known(), Some(GerbilStatus::RuntimeUnavailable));
    assert_eq!(unavailable.code(), 4);
    assert_eq!(unavailable.to_string(), "RuntimeUnavailable (4)");

    let future = crate::NativeRuntimeStatus::from_code(91);
    assert_eq!(future.known(), None);
    assert_eq!(future.code(), 91);
    assert_eq!(future.to_string(), "UnknownGerbilStatus (91)");
}

#[test]
fn stamped_projection_round_trips_through_admission() {
    let stamped = stamp_projection("generated body\n", INPUT);
    validate_projection(&stamped, INPUT).expect("fresh projection must be admitted");
}

#[test]
fn modified_projection_body_fails_closed() {
    let stamped = stamp_projection("generated body\n", INPUT);
    let tampered = stamped.replace("generated body", "modified body");
    let error = validate_projection(&tampered, INPUT).expect_err("drift must be rejected");
    assert!(error.to_string().contains("body fingerprint"));
}

#[test]
fn stale_scheme_input_fails_closed() {
    let stamped = stamp_projection("generated body\n", INPUT);
    let current = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
    let error = validate_projection(&stamped, current).expect_err("stale input must be rejected");
    assert!(error.to_string().contains("input fingerprint"));
}

#[test]
fn gerbil_package_owns_only_the_linked_parser_edge() {
    let package = include_str!("../../../../gerbil.pkg");
    assert!(
        package.contains("github.com/tao3k/gerbil-parser@956c18b1f0e833ba775f8077eafabbfecd543599")
    );
    assert_eq!(package.matches("github.com/tao3k/").count(), 1);
    assert!(!package.contains("github.com/tao3k/poo-flow@"));
    assert!(!package.contains("github.com/tao3k/asp-gerbil-scheme@"));
    assert!(!package.contains("github.com/mighty-gerbils/gerbil-poo@"));
    assert!(!package.contains("gerbil-scheme-language-project-harness"));
}

#[test]
fn rust_commands_inherit_the_canonical_gxpkg_environment() {
    let devenv = include_str!("../../../../devenv.nix");
    let justfile = include_str!("../../../../justfile");
    let readme = include_str!("../../../../README.md");
    let ci = include_str!("../../../../.github/workflows/ci.yml");

    assert!(devenv.contains("scripts.mrr-cargo.exec"));
    assert!(devenv.contains("exec gerbil env cargo \"$@\""));
    assert!(justfile.contains("mrr-gerbil build"));
    assert!(justfile.contains("mrr-cargo test --workspace --locked"));
    assert!(!justfile.contains(" gxpkg "));
    assert!(!justfile.contains(" cargo test"));
    assert!(readme.contains("devenv-profile-exec mrr-cargo test --workspace"));
    assert!(!readme.contains("devenv-profile-exec cargo test --workspace"));
    assert!(ci.contains("gerbil env cargo test --workspace --locked"));
    assert!(ci.contains("macos-latest"));
    assert!(ci.contains("GERBIL_BUILD_VERBOSE: \"1\""));
    assert!(ci.contains("Install declared Gerbil dependencies"));
    assert!(ci.contains("Build declared Gerbil package"));
    assert!(ci.contains("brew --prefix openssl@3"));
    assert!(ci.contains("LIBRARY_PATH=$openssl_prefix/lib"));
    assert!(!ci.contains("gparse"));
    assert!(!ci.contains("audit-spec"));
}

#[test]
fn native_aot_reuses_the_upstream_program_builder_and_runtime() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("mrr-gerbil is a workspace crate");
    let adapter =
        include_str!("../../../../build-support/mrr-gerbil-native-build/src/native_archive.rs");
    let ffi = include_str!("../../src/native/ffi.rs");

    assert!(adapter.contains("build_program_archive_with_contract"));
    assert!(adapter.contains("ProgramArchiveObserver"));
    for authoring_module in [
        "gerbil-parser/language-support",
        "gerbil-parser/src/language-support/antlr4-source",
        "gerbil-parser/src/language-support/antlr4-source-lexer",
        "gerbil-parser/src/language-support/fixture",
        "gerbil-parser/src/language-support/grammar-source",
        "gerbil-parser/src/language-support/iso-bnf",
        "gerbil-parser/src/language-support/javacc-source",
    ] {
        assert!(
            adapter.contains(authoring_module),
            "AOT contract must reject authoring-only module {authoring_module}"
        );
    }
    assert!(!adapter.contains("Command::new(\"gxc\")"));
    assert!(!adapter.contains("Command::new(\"gcc\")"));
    assert!(ffi.contains("gerbil_scheme_rust_runtime_init_program(Some(mrr_grammar_linker))"));
    assert!(
        !workspace
            .join("crates/mrr-gerbil/native/runtime.c")
            .exists()
    );
}

#[test]
fn native_projection_identifies_its_parser_owned_grammar() {
    let authority = crate::load_parser_authority()
        .expect("native projection must expose canonical parser authority");
    assert_eq!(authority.schema, "gerbil-parser.language-grammar.v1");
    assert_eq!(authority.language, "gql");
    assert_eq!(authority.version, "edition-1-2024-04");
    assert_eq!(
        authority.contract,
        "iso-iec-39075-2024.opengql-1.9.0-syntax.v1"
    );
    assert_eq!(authority.grammar_schema, "gerbil-parser.grammar-ir.v1");
    assert_eq!(authority.grammar_id, "gql-iso-grammar");
}

#[test]
fn parser_owned_parse_artifact_crosses_the_native_boundary_losslessly() {
    let source = "MATCH (n {name: '\u{827e}\u{8fbe}'}) RETURN n\n";
    let artifact = crate::parse_gql_artifact(source).expect("parser-owned ParseArtifact v1");
    assert_eq!(artifact.schema, crate::PARSE_ARTIFACT_SCHEMA_V1);
    assert_eq!(artifact.language, crate::ParserLanguage::Gql);
    assert_eq!(artifact.kind_catalog.language(), crate::ParserLanguage::Gql);
    assert_eq!(artifact.kind_catalog.kinds().len(), 581);
    assert_eq!(artifact.kind_catalog.kind_id("GqlProgram"), Some(0));
    assert_eq!(
        artifact.kind_catalog.terminal_kind_id("identifier"),
        artifact.kind_catalog.kind_id("LexicalIdentifier")
    );
    assert_eq!(
        artifact.kind_catalog.kinds().last().map(|kind| kind.name()),
        Some("UnknownToken")
    );
    assert_eq!(artifact.status, crate::ParseArtifactStatus::Accepted);
    assert!(artifact.grammar_digest.starts_with("sha256:"));
    assert_eq!(
        artifact.source_digest,
        "sha256:ccb1b889245b5e30f8c82eb07c54e54f4fa50e165cdced14692f413f439b79a0"
    );
    assert!(matches!(
        artifact.events.first(),
        Some(crate::ParseEvent::StartNode { kind, start: 0, .. }) if kind == "GqlProgram"
    ));
    let replayed = artifact
        .events
        .iter()
        .filter_map(|event| match event {
            crate::ParseEvent::Token { lexeme, .. } => Some(lexeme.as_str()),
            _ => None,
        })
        .collect::<String>();
    assert_eq!(replayed, source);
    let unicode = artifact
        .events
        .iter()
        .find_map(|event| match event {
            crate::ParseEvent::Token {
                lexeme, start, end, ..
            } if lexeme.contains('\u{827e}') => Some((lexeme, start, end)),
            _ => None,
        })
        .expect("Unicode token");
    assert_eq!(*unicode.2 - *unicode.1, unicode.0.len() as u32);
    let cst = artifact
        .to_rowan_cst()
        .expect("accepted ParseArtifact must sink into Rowan");
    assert_eq!(cst.root().text().to_string(), source);
    assert_eq!(cst.root().kind().raw(), 0);
    assert_eq!(
        cst.catalog().kinds()[usize::from(cst.root().kind().raw())].name(),
        "GqlProgram"
    );
}

#[test]
fn parser_owned_artifact_is_bound_to_the_exact_requested_source() {
    let source = "MATCH (n) RETURN n";
    let mut artifact = crate::parse_gql_artifact(source).expect("source-bound artifact");
    artifact.source_digest = "sha256:stale".into();
    assert_eq!(
        crate::native::parse_artifact::validate_source_digest(&artifact, source),
        Err(crate::ParseArtifactLoadError::InvalidSourceDigest(
            "sha256:stale".into()
        ))
    );
}

#[test]
fn parser_owned_failure_does_not_publish_partial_cst_events() {
    let artifact = crate::parse_gql_artifact("MATCH (\n").expect("typed rejected artifact");
    assert_eq!(artifact.schema, crate::PARSE_ARTIFACT_SCHEMA_V1);
    assert_eq!(artifact.status, crate::ParseArtifactStatus::Rejected);
    assert!(
        artifact
            .events
            .iter()
            .all(|event| matches!(event, crate::ParseEvent::Token { .. }))
    );
    assert_eq!(
        artifact.to_rowan_cst(),
        Err(crate::ParserCstError::RejectedArtifact)
    );
}

fn parse_artifact_payload(source: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};

    let mut payload = vec![0_u8; 80];
    payload[..4].copy_from_slice(b"GPA1");
    payload[4..8].copy_from_slice(&1_u32.to_le_bytes());
    payload[8..12].copy_from_slice(&0_u32.to_le_bytes());
    payload[12..16].copy_from_slice(&0_u32.to_le_bytes());
    payload[16..48].fill(0xaa);
    payload[48..80].copy_from_slice(&Sha256::digest(source.as_bytes()));
    payload
}

fn parser_kind_descriptor() -> serde_json::Value {
    serde_json::json!({
        "schema": crate::PARSER_NATIVE_DESCRIPTOR_SCHEMA_V1,
        "language": "gql",
        "grammarDigest": TEST_GRAMMAR_DIGEST,
        "fields": ["text", "operator", "sign"],
        "syntaxKinds": [
            ["GqlProgram", "node", []],
            ["UnknownToken", "token", ["text"]]
        ],
        "terminals": [["unknown", "UnknownToken"]]
    })
}

fn test_parser_kind_catalog() -> std::sync::Arc<crate::ParserKindCatalog> {
    std::sync::Arc::new(
        crate::native::parse_artifact::load_kind_catalog(
            &parser_kind_descriptor(),
            crate::ParserLanguage::Gql,
        )
        .expect("test parser kind catalog"),
    )
}

fn assert_invalid_kind_catalog(descriptor: serde_json::Value) {
    assert_eq!(
        crate::native::parse_artifact::load_kind_catalog(&descriptor, crate::ParserLanguage::Gql,),
        Err(crate::ParseArtifactLoadError::InvalidHostDescriptor)
    );
}

#[test]
fn parser_kind_catalog_rejects_stale_descriptor_authority() {
    let mut wrong_schema = parser_kind_descriptor();
    wrong_schema["schema"] = serde_json::json!("gerbil-parser.native-descriptor.unknown");
    assert_invalid_kind_catalog(wrong_schema);

    let mut malformed_grammar_digest = parser_kind_descriptor();
    malformed_grammar_digest["grammarDigest"] = serde_json::json!("sha256:not-a-digest");
    assert_invalid_kind_catalog(malformed_grammar_digest);

    let mut wrong_language = parser_kind_descriptor();
    wrong_language["language"] = serde_json::json!("cypher");
    assert_invalid_kind_catalog(wrong_language);
}

#[test]
fn parser_kind_catalog_rejects_duplicate_and_unknown_kinds() {
    let mut duplicate = parser_kind_descriptor();
    duplicate["syntaxKinds"] = serde_json::json!([
        ["GqlProgram", "node", []],
        ["GqlProgram", "token", ["text"]]
    ]);
    assert_invalid_kind_catalog(duplicate);

    let mut unknown_category = parser_kind_descriptor();
    unknown_category["syntaxKinds"][0][1] = serde_json::json!("opaque");
    assert_invalid_kind_catalog(unknown_category);

    let mut duplicate_fields = parser_kind_descriptor();
    duplicate_fields["fields"] = serde_json::json!(["text", "text"]);
    assert_invalid_kind_catalog(duplicate_fields);

    let mut undeclared_field = parser_kind_descriptor();
    undeclared_field["fields"] = serde_json::json!(["operator", "sign"]);
    assert_invalid_kind_catalog(undeclared_field);
}

#[test]
fn parser_kind_catalog_rejects_invalid_terminal_ownership() {
    let mut duplicate = parser_kind_descriptor();
    duplicate["terminals"] =
        serde_json::json!([["unknown", "UnknownToken"], ["unknown", "UnknownToken"]]);
    assert_invalid_kind_catalog(duplicate);

    let mut missing_kind = parser_kind_descriptor();
    missing_kind["terminals"][0][1] = serde_json::json!("MissingToken");
    assert_invalid_kind_catalog(missing_kind);

    let mut node_kind = parser_kind_descriptor();
    node_kind["terminals"][0][1] = serde_json::json!("GqlProgram");
    assert_invalid_kind_catalog(node_kind);
}

#[test]
fn unknown_parse_artifact_wire_format_fails_closed() {
    let source = "";
    let mut payload = parse_artifact_payload(source);
    payload[..4].copy_from_slice(b"GPA0");
    let error = crate::native::parse_artifact::decode_parse_artifact(
        &payload,
        source,
        test_parser_kind_catalog(),
    )
    .expect_err("only the frozen ParseArtifact V1 wire projection is admitted");
    assert_eq!(error, crate::ParseArtifactLoadError::InvalidPayload);
}

#[test]
fn parse_artifact_grammar_must_match_the_admitted_native_descriptor() {
    let source = "";
    let mut payload = parse_artifact_payload(source);
    payload[16] = 0xbb;
    assert!(matches!(
        crate::native::parse_artifact::decode_parse_artifact(
            &payload,
            source,
            test_parser_kind_catalog()
        ),
        Err(crate::ParseArtifactLoadError::InvalidGrammarDigest(_))
    ));
}

#[test]
fn parallel_parser_callers_share_one_ordered_native_owner() {
    const SOURCES: [&str; 8] = [
        "MATCH (a) RETURN a\n",
        "MATCH (b) RETURN b\n",
        "MATCH (c) RETURN c\n",
        "MATCH (d) RETURN d\n",
        "MATCH (e) RETURN e\n",
        "MATCH (f) RETURN f\n",
        "MATCH (g) RETURN g\n",
        "MATCH (h) RETURN h\n",
    ];
    let barrier = Arc::new(Barrier::new(SOURCES.len()));

    std::thread::scope(|scope| {
        for source in SOURCES {
            let barrier = Arc::clone(&barrier);
            scope.spawn(move || {
                barrier.wait();
                let artifact = crate::parse_gql_artifact(source)
                    .expect("resident native owner must answer every concurrent caller");
                assert_eq!(artifact.status, crate::ParseArtifactStatus::Accepted);
                let replayed = artifact
                    .events
                    .iter()
                    .filter_map(|event| match event {
                        crate::ParseEvent::Token { lexeme, .. } => Some(lexeme.as_str()),
                        _ => None,
                    })
                    .collect::<String>();
                assert_eq!(
                    replayed, source,
                    "native parser response crossed request order"
                );
            });
        }
    });
}

#[test]
fn native_aot_binding_exposes_the_declaration_without_text_protocols() {
    let grammar = NativeGrammar::load().expect("native Gerbil grammar ABI must load");

    let shape_names: BTreeSet<_> = grammar
        .syntax_shapes
        .iter()
        .map(|shape| shape.name.as_str())
        .collect();
    assert_eq!(shape_names.len(), grammar.syntax_shapes.len());
    let node_pattern = grammar
        .syntax_shapes
        .iter()
        .find(|shape| shape.name == "NodePattern")
        .expect("NodePattern syntax shape");
    assert_eq!(
        node_pattern.fields,
        ["binding", "labels", "properties", "predicate"]
    );
    let property_entry = grammar
        .syntax_shapes
        .iter()
        .find(|shape| shape.name == "PropertyEntry")
        .expect("PropertyEntry syntax shape");
    assert_eq!(property_entry.fields, ["key", "value"]);
    let graph_pattern_list = grammar
        .syntax_shapes
        .iter()
        .find(|shape| shape.name == "GraphPatternList")
        .expect("GraphPatternList syntax shape");
    assert_eq!(graph_pattern_list.fields, ["pattern"]);
    assert_label_predicate_shapes(&grammar);
    let path_mode = grammar
        .syntax_shapes
        .iter()
        .find(|shape| shape.name == "PathMode")
        .expect("PathMode syntax shape");
    assert_eq!(path_mode.fields, ["kind"]);
    for (name, fields) in [
        ("PathPattern", &["binding", "prefix", "pattern"][..]),
        ("GraphMatchMode", &["kind", "target", "bindings"][..]),
        ("PathPrefix", &["search", "mode", "target"][..]),
        ("PathSearch", &["kind", "count", "grouping"][..]),
        ("KeepClause", &["prefix"][..]),
    ] {
        let shape = grammar
            .syntax_shapes
            .iter()
            .find(|shape| shape.name == name)
            .unwrap_or_else(|| panic!("missing {name} syntax shape"));
        assert_eq!(shape.fields, fields, "{name}");
    }
    for keyword in ["WALK", "TRAIL", "ACYCLIC", "SIMPLE"] {
        assert!(
            grammar.keywords.iter().any(|entry| entry.text == keyword),
            "missing path-mode keyword {keyword}"
        );
    }

    assert_eq!(
        grammar.keywords.first().expect("MATCH keyword").text,
        "MATCH"
    );
    assert_eq!(grammar.non_reserved_words.len(), 47);
    assert_eq!(
        grammar.non_reserved_words.first().map(String::as_str),
        Some("ACYCLIC")
    );
    assert_eq!(
        grammar.non_reserved_words.last().map(String::as_str),
        Some("ZONE")
    );
    assert!(
        grammar
            .non_reserved_words
            .iter()
            .any(|word| word == "GRAPH")
    );
    assert_eq!(grammar.numeric_literals.len(), 9);
    assert!(grammar.numeric_literals.iter().any(|literal| {
        literal.form == "exact-scientific"
            && literal.notation == "scientific"
            && literal.suffix == "M"
            && literal.class == "exact"
    }));
    assert_eq!(grammar.character_string_literals.len(), 14);
    assert!(grammar.character_string_literals.iter().any(|literal| {
        literal.form == "no-escape"
            && literal.lexeme == "commercial-at"
            && literal.action == "preserve-representations"
            && literal.class == "raw"
    }));
    assert!(grammar.character_string_literals.iter().any(|literal| {
        literal.form == "escaped-unicode6"
            && literal.lexeme == "U"
            && literal.action == "decode"
            && literal.class == "six-hex-digits"
    }));
    assert_eq!(grammar.parameter_references.len(), 2);
    assert!(grammar.parameter_references.iter().any(|parameter| {
        parameter.form == "general"
            && parameter.prefix == "dollar"
            && parameter.name == "separated-identifier"
            && parameter.context == "dynamic-value"
    }));
    assert_eq!(
        grammar
            .predicate_tests
            .iter()
            .map(|predicate| (
                predicate.kind.as_str(),
                predicate.negation.as_str(),
                predicate.value.as_str(),
                predicate.operand.as_str(),
            ))
            .collect::<Vec<_>>(),
        [
            ("null", "optional-not", "Null", "any-value"),
            ("truth", "optional-not", "True", "boolean-or-null"),
            ("truth", "optional-not", "False", "boolean-or-null"),
            ("truth", "optional-not", "UnknownTruth", "boolean-or-null",),
            (
                "value-type",
                "optional-not",
                "declared-value-type",
                "value-primary",
            ),
            ("directed", "optional-not", "Directed", "edge-element"),
            ("source", "optional-not", "Source", "node-edge"),
            ("destination", "optional-not", "Destination", "node-edge",),
            (
                "all-different",
                "forbidden",
                "AllDifferent",
                "element-list-min-two",
            ),
            ("same", "forbidden", "Same", "element-list-min-two"),
            (
                "property-exists",
                "forbidden",
                "PropertyExists",
                "element-property",
            ),
        ]
    );
    assert_eq!(grammar.aggregate_functions.len(), 11);
    assert!(grammar.aggregate_functions.iter().any(|aggregate| {
        aggregate.name == "count-star"
            && aggregate.keyword == "Count"
            && aggregate.kind == "star"
            && aggregate.quantifier == "forbidden"
            && aggregate.arity == 0
    }));
    assert!(grammar.aggregate_functions.iter().any(|aggregate| {
        aggregate.name == "percentile-discrete"
            && aggregate.keyword == "PercentileDisc"
            && aggregate.kind == "binary"
            && aggregate.quantifier == "dependent"
            && aggregate.arity == 2
    }));
    assert!(grammar.parameter_references.iter().any(|parameter| {
        parameter.form == "substituted"
            && parameter.prefix == "double-dollar"
            && parameter.context == "catalog-reference"
    }));
    assert!(grammar.numeric_literals.iter().any(|literal| {
        literal.form == "approximate-scientific-unsuffixed"
            && literal.notation == "scientific"
            && literal.suffix == "none"
            && literal.class == "approximate"
    }));
    assert!(grammar.keywords.iter().any(|keyword| keyword.text == "END"));
    assert!(
        grammar
            .keywords
            .iter()
            .any(|keyword| keyword.text == "SCHEMA")
    );
    assert_eq!(grammar.prefix_operators[0].precedence, 25);
    assert_eq!(grammar.prefix_operators[0].associativity, "right");
    assert_eq!(grammar.binary_operators[0].lexeme, "Or");
    assert_eq!(grammar.binary_operators[0].precedence, 10);
    assert_eq!(grammar.binary_operators[0].associativity, "left");
    assert_eq!(grammar.parser_entrypoints[0].keyword, "Match");
    assert_eq!(grammar.parser_entrypoints[0].action, "MatchClause");
    assert_eq!(grammar.parser_entrypoints[0].effect, "marks-match");
    let unsupported = grammar
        .recoveries
        .iter()
        .find(|recovery| recovery.site == "unsupported-statement")
        .expect("unsupported statement recovery");
    assert_eq!(unsupported.code, "GQL-PARSE-UNSUPPORTED-STATEMENT");
    let delimited = grammar
        .recoveries
        .iter()
        .find(|recovery| recovery.site == "delimited-identifier")
        .expect("delimited identifier recovery");
    assert_eq!(
        delimited.code,
        "GQL-SYNTAX-UNTERMINATED-DELIMITED-IDENTIFIER"
    );
    let invalid_escape = grammar
        .recoveries
        .iter()
        .find(|recovery| recovery.site == "identifier-escape")
        .expect("identifier escape recovery");
    assert_eq!(invalid_escape.code, "GQL-SYNTAX-INVALID-IDENTIFIER-ESCAPE");
    let string = grammar
        .recoveries
        .iter()
        .find(|recovery| recovery.site == "string-literal")
        .expect("string literal recovery");
    assert_eq!(string.code, "GQL-SYNTAX-UNTERMINATED-STRING");
    let character_string = grammar
        .recoveries
        .iter()
        .find(|recovery| recovery.site == "character-string-literal")
        .expect("character-string literal recovery");
    assert_eq!(
        character_string.code,
        "GQL-SYNTAX-INVALID-CHARACTER-STRING-LITERAL"
    );
    let block_comment = grammar
        .recoveries
        .iter()
        .find(|recovery| recovery.site == "block-comment")
        .expect("block comment recovery");
    assert_eq!(block_comment.code, "GQL-SYNTAX-UNTERMINATED-BLOCK-COMMENT");
    let numeric = grammar
        .recoveries
        .iter()
        .find(|recovery| recovery.site == "numeric-literal")
        .expect("numeric literal recovery");
    assert_eq!(numeric.code, "GQL-SYNTAX-INVALID-NUMERIC-LITERAL");
    let integer_range = grammar
        .recoveries
        .iter()
        .find(|recovery| recovery.site == "integer-literal-range")
        .expect("integer literal range recovery");
    assert_eq!(
        integer_range.code,
        "GQL-SYNTAX-NUMERIC-LITERAL-OUT-OF-RANGE"
    );
    let edge_separator = grammar
        .recoveries
        .iter()
        .find(|recovery| recovery.site == "edge-label-separator")
        .expect("edge label separator recovery");
    assert_eq!(edge_separator.code, "GQL-PARSE-EDGE-LABEL-SEPARATOR");
    let create_schema = grammar
        .recoveries
        .iter()
        .find(|recovery| recovery.site == "create-schema")
        .expect("CREATE SCHEMA recovery");
    assert_eq!(create_schema.code, "GQL-PARSE-CREATE-SCHEMA-SYNTAX");
    let inline_where = grammar
        .recoveries
        .iter()
        .find(|recovery| recovery.site == "inline-node-where")
        .expect("inline node WHERE recovery");
    assert_eq!(inline_where.code, "GQL-PARSE-INLINE-WHERE-SYNTAX");
    for (site, code) in [
        ("graph-match-mode", "GQL-PARSE-GRAPH-MATCH-MODE-SYNTAX"),
        ("path-search-prefix", "GQL-PARSE-PATH-SEARCH-PREFIX-SYNTAX"),
        ("keep-clause", "GQL-PARSE-KEEP-CLAUSE-SYNTAX"),
    ] {
        assert_eq!(
            grammar
                .recoveries
                .iter()
                .find(|recovery| recovery.site == site)
                .unwrap_or_else(|| panic!("missing {site} recovery"))
                .code,
            code
        );
    }
    assert!(grammar.parser_entrypoints.iter().any(|entrypoint| {
        entrypoint.keyword == "Create" && entrypoint.action == "CreateSchemaStatement"
    }));
}

#[test]
fn native_aot_binding_exposes_the_scheme_owned_enhanced_query_table() {
    let table = crate::load_enhanced_tree_sitter_query_operator_table()
        .expect("Scheme-owned enhanced Tree-sitter Query table must load");

    assert_eq!(table.profile_id, "mrr.enhanced-tree-sitter-query.v1");
    assert_eq!(table.owner, "mrr-gerbil-aot");
    assert_eq!(
        table.declaration_digest,
        "blake3-256:e9f88db6f6cab915cad26739fd9c9da58e1dbd8e73b7e314adb08ff6fca45d2a"
    );
    assert_eq!(table.operators.len(), 12);
    assert!(
        table
            .operators
            .iter()
            .all(|operator| operator.spelling.starts_with("#asp-"))
    );

    let related = table
        .operators
        .iter()
        .find(|operator| operator.spelling == "#asp-related?")
        .expect("relation predicate");
    assert_eq!(related.minimum_arity, 3);
    assert_eq!(related.maximum_arity, Some(4));
    assert_eq!(related.lowering, "related");
    assert_eq!(related.operands[3].domain, "endpoint-selector");
    assert_eq!(related.operands[3].cardinality, "optional");

    let select = table
        .operators
        .iter()
        .find(|operator| operator.spelling == "#asp-select!")
        .expect("result directive");
    assert_eq!(select.kind, "directive");
    assert_eq!(select.minimum_arity, 2);
    assert_eq!(select.maximum_arity, None);
    assert_eq!(select.operands[1].domain, "result-field");
    assert_eq!(select.operands[1].cardinality, "one-or-more");

    assert!(
        table
            .recoveries
            .iter()
            .all(|recovery| recovery.strategy == "reject")
    );
    assert!(table.recoveries.iter().any(|recovery| {
        recovery.site == "fact" && recovery.code == "enhanced-query-fact-not-resident"
    }));
}

#[test]
fn native_aot_profile_is_complete_unique_and_evidence_owned() {
    let profile = crate::load_iso_profile().expect("public ISO profile AOT API must load");
    assert_eq!(profile.schema, "mrr.iso-gql-profile.v1");
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("mrr-gerbil is a workspace crate");

    assert_eq!(profile.releases.len(), 2);
    let modules = profile
        .modules
        .iter()
        .map(|module| module.id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(modules.len(), 6);
    assert!(
        profile
            .modules
            .iter()
            .all(|module| module.kind == "iso-standard-module")
    );

    let releases = profile
        .releases
        .iter()
        .map(|release| release.id.as_str())
        .collect::<BTreeSet<_>>();
    let profiles = profile
        .profiles
        .iter()
        .map(|profile| (profile.id.as_str(), profile))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(profiles.len(), 3);
    assert!(
        profile
            .profiles
            .iter()
            .all(|profile| releases.contains(profile.release_id.as_str()))
    );
    assert_eq!(profile.profile_supplements.len(), 1);
    assert_eq!(
        profile.profile_supplements[0].profile_id,
        "gql-iso-language-frontend-v1"
    );
    assert_eq!(
        profile.profile_supplements[0].release_id,
        "iso-39075-2024-cor-1"
    );

    let valid_statuses = [
        "partial",
        "not-implemented",
        "not-applicable",
        "implemented",
    ];
    let mut feature_priorities = BTreeMap::new();
    let mut priorities = BTreeSet::new();
    for feature in &profile.features {
        assert!(
            feature_priorities
                .insert(feature.id.as_str(), feature.priority)
                .is_none(),
            "duplicate feature {}",
            feature.id
        );
        assert!(priorities.insert(feature.priority));
        assert!(modules.contains(feature.module_id.as_str()));
        assert_eq!(feature.clause_status, "pending-licensed-clause");
        assert!(
            feature
                .layer_statuses
                .iter()
                .all(|status| valid_statuses.contains(&status.as_str()))
        );
        assert!(
            !feature
                .layer_statuses
                .iter()
                .any(|status| status == "implemented"),
            "{} cannot claim an implemented ISO layer without licensed clause evidence",
            feature.id
        );
        assert!(
            workspace.join(&feature.evidence_owner).is_file(),
            "{} evidence owner does not exist: {}",
            feature.id,
            feature.evidence_owner
        );
    }
    assert_eq!(profile.features.len(), 23);

    let mut dependency_edges = BTreeSet::new();
    for dependency in &profile.feature_dependencies {
        assert!(
            dependency_edges.insert((
                dependency.feature_id.as_str(),
                dependency.dependency_id.as_str()
            )),
            "duplicate dependency edge"
        );
        let feature_priority = feature_priorities[dependency.feature_id.as_str()];
        let dependency_priority = feature_priorities[dependency.dependency_id.as_str()];
        assert!(dependency_priority < feature_priority);
    }

    for membership in &profile.profile_modules {
        assert!(profiles.contains_key(membership.profile_id.as_str()));
        assert!(modules.contains(membership.module_id.as_str()));
        assert!(matches!(
            membership.disposition.as_str(),
            "included" | "deferred"
        ));
    }
    let target_memberships = profile
        .profile_modules
        .iter()
        .filter(|membership| membership.profile_id == "gql-iso-language-frontend-v1")
        .collect::<Vec<_>>();
    assert_eq!(target_memberships.len(), modules.len());
    assert!(
        target_memberships
            .iter()
            .all(|membership| membership.disposition == "included")
    );
    assert_eq!(
        profiles["gql-iso-language-frontend-v1"].claim,
        "independent-full-iso-language-frontend-target"
    );
}

#[test]
fn same_aot_reasoning_module_produces_byte_identical_canonical_bundles() {
    let first = load_reasoning_bundle().expect("first native bundle projection");
    let second = load_reasoning_bundle().expect("second native bundle projection");

    assert_eq!(first.id(), second.id());
    assert_eq!(first.encode_canonical(), second.encode_canonical());
    assert!(
        first
            .encode_canonical()
            .starts_with(b"mrr.reasoning-bundle.v1\0")
    );
    assert_eq!(first.relations().len(), 2);
    assert_eq!(first.query_templates().len(), 1);
    assert_eq!(first.rule_packs().len(), 1);
    assert_eq!(first.rules().count(), 2);
    assert_eq!(first.inverse_goals().len(), 1);
    assert_eq!(first.transition_systems().len(), 1);
    assert_eq!(first.lineage_policy(), LineagePolicy::Complete);
    assert_eq!(first.projection_policy(), ProjectionPolicy::default());
    assert_eq!(first.validation_profile(), ValidationProfile::default());
}

#[test]
fn parallel_callers_share_one_serial_native_runtime_boundary() {
    const CALLERS: usize = 8;
    const LOADS_PER_CALLER: usize = 8;
    let barrier = Arc::new(Barrier::new(CALLERS));

    std::thread::scope(|scope| {
        for caller in 0..CALLERS {
            let barrier = Arc::clone(&barrier);
            scope.spawn(move || {
                barrier.wait();
                for _ in 0..LOADS_PER_CALLER {
                    if caller % 2 == 0 {
                        let grammar = NativeGrammar::load()
                            .expect("serialized native Gerbil grammar ABI must load");
                        assert_label_predicate_shapes(&grammar);
                    } else {
                        let bundle = load_reasoning_bundle()
                            .expect("serialized native Gerbil reasoning ABI must load");
                        assert_eq!(bundle.relations().len(), 2);
                    }
                }
            });
        }
    });
}

fn assert_label_predicate_shapes(grammar: &NativeGrammar) {
    for (name, fields) in [
        ("LabelPredicateExpression", &["operand", "label"][..]),
        ("LabelNameExpression", &["name"][..]),
        ("LabelWildcardExpression", &["wildcard"][..]),
        ("LabelNotExpression", &["operand"][..]),
        ("LabelAndExpression", &["left", "right"][..]),
        ("LabelOrExpression", &["left", "right"][..]),
    ] {
        let shape = grammar
            .syntax_shapes
            .iter()
            .find(|shape| shape.name == name)
            .unwrap_or_else(|| panic!("missing declaration-owned {name} syntax shape"));
        assert_eq!(shape.fields, fields, "field drift for {name}");
    }
}

#[test]
fn scheme_aot_driver_owns_resource_order_and_completion() {
    let proposal = crate::driver_request(crate::DriverPhase::AwaitProposal)
        .expect("Scheme driver request")
        .expect("proposal resource");
    assert_eq!(proposal, crate::DriverResource::ModelProposal);

    let closure_phase = crate::driver_transition(crate::DriverTransition {
        phase: crate::DriverPhase::AwaitProposal,
        resource: proposal,
        status: crate::DriverStatus::Candidate,
        cycle: 0,
        max_cycles: 2,
    })
    .expect("candidate transition");
    assert_eq!(closure_phase, crate::DriverPhase::AwaitClosure);
    assert_eq!(
        crate::driver_request(closure_phase).expect("Scheme driver request"),
        Some(crate::DriverResource::MrrClosure)
    );

    let complete = crate::driver_transition(crate::DriverTransition {
        phase: closure_phase,
        resource: crate::DriverResource::MrrClosure,
        status: crate::DriverStatus::Admitted,
        cycle: 0,
        max_cycles: 2,
    })
    .expect("admission transition");
    assert_eq!(complete, crate::DriverPhase::Complete);
    assert_eq!(
        crate::driver_request(complete).expect("complete state"),
        None
    );
}

#[test]
fn scheme_aot_driver_rejects_wrong_authority_and_budget_exhaustion() {
    assert_eq!(
        crate::driver_transition(crate::DriverTransition {
            phase: crate::DriverPhase::AwaitProposal,
            resource: crate::DriverResource::MrrClosure,
            status: crate::DriverStatus::Candidate,
            cycle: 0,
            max_cycles: 2,
        }),
        Err(crate::DriverError::InvalidTransition)
    );
    assert_eq!(
        crate::driver_transition(crate::DriverTransition {
            phase: crate::DriverPhase::AwaitClosure,
            resource: crate::DriverResource::MrrClosure,
            status: crate::DriverStatus::Rejected,
            cycle: 0,
            max_cycles: 1,
        }),
        Err(crate::DriverError::BudgetExhausted)
    );
}
