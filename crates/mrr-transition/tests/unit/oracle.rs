use core::num::NonZeroUsize;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    Action, ActionId, Effect, FactId, InitialState, Invariant, Precondition, SafetyLimits,
    SafetyStatus, StatePredicate, StateSchema, StateSnapshot, TransitionSystem, check_safety,
};

struct OracleFixture {
    name: String,
    chain_length: usize,
    unsafe_model: bool,
}

#[test]
fn ten_transition_fixtures_match_tlc_status_and_counterexample_length() {
    if std::env::var("MRR_TLC_ORACLE_VERIFY").as_deref() != Ok("1") {
        eprintln!("registered external gate: set MRR_TLC_ORACLE_VERIFY=1 to execute TLC parity");
        return;
    }
    let fixtures = (1..=5)
        .flat_map(|chain_length| {
            [
                OracleFixture {
                    name: format!("safe-{chain_length}"),
                    chain_length,
                    unsafe_model: false,
                },
                OracleFixture {
                    name: format!("unsafe-{chain_length}"),
                    chain_length,
                    unsafe_model: true,
                },
            ]
        })
        .collect::<Vec<_>>();

    for fixture in fixtures {
        compare_with_tlc(&fixture);
    }
}

fn compare_with_tlc(fixture: &OracleFixture) {
    let system = rust_model(fixture);
    let receipt = check_safety(
        &system,
        SafetyLimits::new(
            NonZeroUsize::new(128).expect("nonzero state budget"),
            NonZeroUsize::new(1024).expect("nonzero transition budget"),
        ),
    )
    .expect("admitted model evaluates");
    let expected_status = if fixture.unsafe_model {
        SafetyStatus::Unsafe
    } else {
        SafetyStatus::Safe
    };
    assert_eq!(receipt.status(), expected_status, "{}", fixture.name);

    let temp_root = std::env::temp_dir().join(format!(
        "gql-rust-mrr-tlc-{}-{}-{}",
        std::process::id(),
        fixture.name,
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    fs::create_dir_all(&temp_root).expect("create TLC oracle directory");
    write_tla_fixture(&temp_root, fixture);
    let output = Command::new("tlc")
        .args(["-cleanup", "-config", "MrrOracle.cfg", "MrrOracle.tla"])
        .current_dir(&temp_root)
        .output()
        .expect("tlc must be provisioned by devenv");
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let tlc_unsafe = combined.contains("Invariant Inv is violated");
    assert_eq!(
        tlc_unsafe, fixture.unsafe_model,
        "{}\n{combined}",
        fixture.name
    );
    assert_eq!(
        output.status.success(),
        !fixture.unsafe_model,
        "{}\n{combined}",
        fixture.name
    );

    if fixture.unsafe_model {
        let rust_states = receipt
            .counterexample()
            .expect("unsafe receipt has counterexample")
            .states()
            .len();
        let tlc_states = combined
            .lines()
            .filter(|line| {
                let line = line.trim_start();
                line.starts_with("State ") && line.contains(':')
            })
            .count();
        assert_eq!(rust_states, fixture.chain_length + 1, "{}", fixture.name);
        assert_eq!(tlc_states, rust_states, "{}\n{combined}", fixture.name);
    } else {
        assert!(receipt.counterexample().is_none(), "{}", fixture.name);
    }

    fs::remove_dir_all(&temp_root).expect("remove TLC oracle directory");
}

fn rust_model(fixture: &OracleFixture) -> TransitionSystem {
    let chain = (0..=fixture.chain_length).map(fact_id).collect::<Vec<_>>();
    let guard = FactId::from_canonical_bytes(b"oracle:guard").expect("guard identity");
    let mut allowed = chain.clone();
    allowed.push(guard);
    let actions = (0..fixture.chain_length)
        .map(|index| {
            Action::new(
                ActionId::from_canonical_bytes(format!("oracle:action:{index}"))
                    .expect("action identity"),
                Precondition::all(vec![StatePredicate::Present(chain[index])]),
                Effect::new(vec![chain[index + 1]], vec![]).expect("effect"),
            )
        })
        .collect();
    let forbidden = if fixture.unsafe_model {
        chain[fixture.chain_length]
    } else {
        guard
    };
    TransitionSystem::admit(
        StateSchema::new(allowed).expect("state schema"),
        InitialState::new(StateSnapshot::from_facts(vec![chain[0]]).expect("initial state")),
        actions,
        vec![Invariant::forbidden_all("Inv", vec![forbidden]).expect("invariant")],
    )
    .expect("transition system")
}

fn fact_id(index: usize) -> FactId {
    FactId::from_canonical_bytes(format!("oracle:fact:{index}")).expect("fact identity")
}

fn write_tla_fixture(root: &Path, fixture: &OracleFixture) {
    let mut source = String::from(
        "---- MODULE MrrOracle ----\nEXTENDS Naturals, FiniteSets\nVARIABLE facts\n\n",
    );
    source.push_str("Init == facts = {\"f0\"}\n\n");
    for index in 0..fixture.chain_length {
        source.push_str(&format!(
            "A{index} == /\\ \"f{index}\" \\in facts\n           /\\ facts' = facts \\cup {{\"f{}\"}}\n\n",
            index + 1
        ));
    }
    let actions = (0..fixture.chain_length)
        .map(|index| format!("A{index}"))
        .collect::<Vec<_>>()
        .join(" \\/ ");
    source.push_str(&format!(
        "Next == {actions}\n\nSpec == Init /\\ [][Next]_facts\n\n"
    ));
    let forbidden = if fixture.unsafe_model {
        format!("f{}", fixture.chain_length)
    } else {
        "guard".to_owned()
    };
    source.push_str(&format!("Inv == ~(\"{forbidden}\" \\in facts)\n\n====\n"));
    fs::write(root.join("MrrOracle.tla"), source).expect("write TLA fixture");
    fs::write(
        root.join("MrrOracle.cfg"),
        "SPECIFICATION Spec\nINVARIANT Inv\n",
    )
    .expect("write TLC configuration");
}
