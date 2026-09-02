use crate::native::NativeGrammar;
use crate::{load_reasoning_bundle, stamp_projection, validate_projection};
use mrr_bundle::{LineagePolicy, ProjectionPolicy, ValidationProfile};
use std::sync::{Arc, Barrier};
use std::{collections::BTreeMap, collections::BTreeSet, path::Path};

const INPUT: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

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
fn gerbil_package_declares_poo_flow_and_poo_dependencies() {
    let package = include_str!("../../../../gerbil.pkg");
    assert!(package.contains("github.com/tao3k/poo-flow@"));
    assert!(package.contains("github.com/tao3k/gerbil-scheme-language-project-harness@"));
    assert!(package.contains("github.com/mighty-gerbils/gerbil-poo@"));
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
    assert!(grammar.parser_entrypoints.iter().any(|entrypoint| {
        entrypoint.keyword == "Create" && entrypoint.action == "CreateSchemaStatement"
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
