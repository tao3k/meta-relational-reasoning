use core::num::NonZeroUsize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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
fn ten_transition_fixtures_match_quint_status_and_counterexample_length() {
    if std::env::var("MRR_QUINT_ORACLE_VERIFY").as_deref() != Ok("1") {
        eprintln!(
            "registered external gate: set MRR_QUINT_ORACLE_VERIFY=1 to execute Quint parity"
        );
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
        compare_with_quint(&fixture);
    }
}

fn compare_with_quint(fixture: &OracleFixture) {
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
        "mrr-quint-{}-{}-{}",
        std::process::id(),
        fixture.name,
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    fs::create_dir_all(&temp_root).expect("create Quint oracle directory");
    write_quint_fixture(&temp_root, fixture);
    let config = temp_root.join("tlc-config.json");
    fs::write(&config, "{\"workers\":\"1\",\"maxHeap\":\"-Xmx1G\"}\n")
        .expect("write finite checker configuration");
    let quint_bin = std::env::var("MRR_QUINT_BIN").unwrap_or_else(|_| "quint".to_owned());
    let mut child = Command::new(&quint_bin)
        .args([
            "verify",
            "MrrOracleFixture.qnt",
            "--main",
            "MrrOracleFixture",
            "--backend",
            "tlc",
            "--apalache-version",
            "0.62.1",
            "--invariant",
            "Inv",
            "--tlc-config",
        ])
        .arg(&config)
        .args(["--verbosity", "5"])
        .current_dir(&temp_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("pinned Quint CLI must be provisioned by the formal gate");
    let (sender, receiver) = mpsc::channel();
    let stdout = forward_checker_lines(
        child.stdout.take().expect("Quint stdout pipe"),
        sender.clone(),
    );
    let stderr = forward_checker_lines(child.stderr.take().expect("Quint stderr pipe"), sender);
    let mut combined = String::new();
    let mut last_cpu = HashMap::new();
    let mut last_progress = Instant::now();
    loop {
        match receiver.recv_timeout(Duration::from_secs(3)) {
            Ok(line) => {
                let line = line.expect("read Quint checker output");
                eprintln!("QUINT-ORACLE {}: {line}", fixture.name);
                combined.push_str(&line);
                combined.push('\n');
                last_progress = Instant::now();
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let cpu = checker_process_cpu(child.id()).expect("sample Quint process CPU");
                if cpu.iter().any(|(pid, seconds)| {
                    *seconds > last_cpu.get(pid).copied().unwrap_or_default() + 0.01
                }) {
                    eprintln!(
                        "QUINT-ORACLE {}: measured checker CPU {:.2}s",
                        fixture.name,
                        cpu.values().sum::<f64>()
                    );
                    last_progress = Instant::now();
                }
                last_cpu = cpu;
                if last_progress.elapsed() >= Duration::from_secs(8) {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!(
                        "{}: no checker output or measured CPU progress for 8 seconds",
                        fixture.name
                    );
                }
            }
        }
    }
    stdout.join().expect("read Quint stdout");
    stderr.join().expect("read Quint stderr");
    let status = child.wait().expect("wait for Quint checker");

    if fixture.unsafe_model {
        assert_eq!(status.code(), Some(1), "{}\n{combined}", fixture.name);
        assert!(
            combined.contains("Error: Invariant q_inv is violated.")
                && combined.contains("error: found a counterexample"),
            "{}\n{combined}",
            fixture.name
        );
        let rust_states = receipt
            .counterexample()
            .expect("unsafe receipt has counterexample")
            .states()
            .len();
        let quint_states = combined
            .lines()
            .filter_map(|line| line.trim_start().strip_prefix("State "))
            .filter_map(|line| {
                line.split_once(':')
                    .and_then(|(index, _)| index.parse::<usize>().ok())
            })
            .collect::<Vec<_>>();
        assert_eq!(rust_states, fixture.chain_length + 1, "{}", fixture.name);
        assert_eq!(
            quint_states,
            (1..=rust_states).collect::<Vec<_>>(),
            "{}\n{combined}",
            fixture.name
        );
        eprintln!(
            "QUINT-ORACLE-OK {} unsafe states={rust_states}",
            fixture.name
        );
    } else {
        assert!(status.success(), "{}\n{combined}", fixture.name);
        assert!(
            combined.contains("[ok] No violation found"),
            "{}\n{combined}",
            fixture.name
        );
        assert!(receipt.counterexample().is_none(), "{}", fixture.name);
        eprintln!("QUINT-ORACLE-OK {} safe", fixture.name);
    }

    fs::remove_dir_all(&temp_root).expect("remove Quint oracle directory");
}

fn forward_checker_lines<R: Read + Send + 'static>(
    reader: R,
    sender: mpsc::Sender<io::Result<String>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        for line in BufReader::new(reader).lines() {
            if sender.send(line).is_err() {
                break;
            }
        }
    })
}

fn checker_process_cpu(root: u32) -> io::Result<HashMap<u32, f64>> {
    let output = Command::new("ps")
        .args(["-Ao", "pid=,ppid=,time="])
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other("ps could not sample Quint processes"));
    }
    let rows = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            Some((
                fields.next()?.parse::<u32>().ok()?,
                fields.next()?.parse::<u32>().ok()?,
                parse_cpu_seconds(fields.next()?)?,
            ))
        })
        .collect::<Vec<_>>();
    let mut descendants = HashSet::from([root]);
    loop {
        let old_len = descendants.len();
        for (pid, parent, _) in &rows {
            if descendants.contains(parent) {
                descendants.insert(*pid);
            }
        }
        if descendants.len() == old_len {
            break;
        }
    }
    Ok(rows
        .into_iter()
        .filter(|(pid, _, _)| descendants.contains(pid))
        .map(|(pid, _, seconds)| (pid, seconds))
        .collect())
}

fn parse_cpu_seconds(value: &str) -> Option<f64> {
    let (days, time) = match value.split_once('-') {
        Some((days, time)) => (days.parse::<u64>().ok()?, time),
        None => (0, value),
    };
    let parts = time.split(':').collect::<Vec<_>>();
    let seconds = match parts.as_slice() {
        [minutes, seconds] => minutes.parse::<f64>().ok()? * 60.0 + seconds.parse::<f64>().ok()?,
        [hours, minutes, seconds] => {
            hours.parse::<f64>().ok()? * 3600.0
                + minutes.parse::<f64>().ok()? * 60.0
                + seconds.parse::<f64>().ok()?
        }
        _ => return None,
    };
    Some(days as f64 * 86_400.0 + seconds)
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

fn write_quint_fixture(root: &Path, fixture: &OracleFixture) {
    let source = format!(
        "{}\nmodule MrrOracleFixture {{\n  import MrrTransitionOracle(LAST = {}, UNSAFE = {}).*\n}}\n",
        include_str!("../../quint/MrrTransitionOracle.qnt"),
        fixture.chain_length,
        fixture.unsafe_model
    );
    fs::write(root.join("MrrOracleFixture.qnt"), source).expect("write Quint fixture");
}
