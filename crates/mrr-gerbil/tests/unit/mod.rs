use crate::native::NativeGrammar;
use crate::{load_reasoning_bundle, stamp_projection, validate_projection};
use mrr_bundle::{LineagePolicy, ProjectionPolicy, ValidationProfile};
use std::sync::{Arc, Barrier};

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

    assert_eq!(grammar.syntax_shapes.len(), 40);
    assert_eq!(grammar.syntax_shapes[7].name, "NodePattern");
    assert_eq!(
        grammar.syntax_shapes[7].fields,
        ["binding", "labels", "properties"]
    );
    assert_eq!(grammar.syntax_shapes[9].name, "PropertyEntry");
    assert_eq!(grammar.syntax_shapes[9].fields, ["key", "value"]);

    assert_eq!(
        grammar.keywords.first().expect("MATCH keyword").text,
        "MATCH"
    );
    assert_eq!(grammar.keywords.last().expect("END keyword").text, "END");
    assert_eq!(grammar.prefix_operators[0].precedence, 25);
    assert_eq!(grammar.prefix_operators[0].associativity, "right");
    assert_eq!(grammar.binary_operators[0].lexeme, "Or");
    assert_eq!(grammar.binary_operators[0].precedence, 10);
    assert_eq!(grammar.binary_operators[0].associativity, "left");
    assert_eq!(grammar.parser_entrypoints[0].keyword, "Match");
    assert_eq!(grammar.parser_entrypoints[0].action, "MatchClause");
    assert_eq!(grammar.parser_entrypoints[0].effect, "marks-match");
    assert_eq!(grammar.recoveries[0].site, "unsupported-statement");
    assert_eq!(
        grammar.recoveries[0].code,
        "GQL-PARSE-UNSUPPORTED-STATEMENT"
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
                        assert_eq!(grammar.syntax_shapes.len(), 40);
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
