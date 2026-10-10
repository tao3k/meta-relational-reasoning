//! Persistent isolated POO owner preserves Host child statuses and proof state.
use mrr_gerbil::{NativeWorker, NativeWorkerError, TemporalHost, TemporalRuntimeError};
fn text(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).unwrap()
}
// Digests have no escape syntax; this is a test assertion for these fixed fields.
fn digest_field(value: &str, name: &str) -> String {
    let marker = format!("(\"{name}\" \"");
    let value = value
        .split_once(&marker)
        .unwrap()
        .1
        .split('"')
        .next()
        .unwrap();
    assert!(value.starts_with("sha256:"));
    assert!(value[7..].bytes().all(|b| b.is_ascii_hexdigit()));
    value.to_owned()
}
fn policy(generation: i32, now: i32) -> String {
    let instant = |n| {
        format!(
            "(object (\"identity\" \"{n}\") (\"domain\" \"txn\") (\"coordinate\" {n}) (\"provenance\" \"host-clock\") (\"modality\" \"observed\"))"
        )
    };
    format!(
        "(object (\"schema\" \"poo-flow.temporal-policy-refresh-request.v1\") (\"policy\" (object (\"identity\" \"mrr-production-policy\") (\"revision\" \"v1\") (\"start\" {}) (\"end\" {}))) (\"generation\" {generation}) (\"effectiveAt\" {}))",
        instant(1),
        instant(4),
        instant(now)
    )
}
fn current(registration: &str, state: i32, policy: i32, digest: &str) -> String {
    format!(
        "(object (\"schema\" \"poo-flow.temporal-proof-current-request.v1\") (\"registration\" \"{registration}\") (\"expectedStateGeneration\" {state}) (\"expectedPolicyGeneration\" {policy}) (\"expectedPolicyDigest\" \"{digest}\") (\"budget\" 128))"
    )
}
#[test]
fn persistent_worker_preserves_proofs_and_host_children() {
    let mut host = NativeWorker::start(std::path::Path::new(env!(
        "CARGO_BIN_EXE_mrr-native-worker"
    )))
    .unwrap();
    assert_eq!(
        TemporalHost.refresh_policy(policy(1, 1).as_bytes()),
        Err(TemporalRuntimeError::RuntimeUnavailable)
    );
    let source = "MATCH (n) RETURN n\n";
    for language in [
        mrr_gerbil::ParserLanguage::Gql,
        mrr_gerbil::ParserLanguage::Cypher,
    ] {
        let artifact = host.parse_artifact(language, source).unwrap();
        assert_eq!(artifact.language, language);
        assert_eq!(
            artifact.to_rowan_cst().unwrap().root().text().to_string(),
            source
        );
        println!("CASE original {language:?} parser and catalog admitted across Scheme v1");
    }
    let finite = host
        .evaluate_finite_relations(3, vec![(0, 1), (1, 2)], vec![0])
        .unwrap();
    assert!(finite.paths.contains(&(0, 2, 2)));
    assert!(finite.influences.contains(&(0, 0, 2, 2)));
    println!("CASE original Scheme finite candidate re-admitted by parent");
    for exit in [0, 7, 23] {
        let mut child = std::process::Command::new("/bin/sh")
            .args(["-c", &format!("sleep 0.1; exit {exit}")])
            .spawn()
            .unwrap();
        host.refresh_policy(policy(1, 1).as_bytes()).unwrap();
        assert_eq!(child.wait().unwrap().code(), Some(exit));
        println!("CASE Host child exit {exit} preserved");
    }
    assert!(matches!(
        host.refresh_policy(b"\0"),
        Err(NativeWorkerError::InvalidInput)
    ));
    let p = text(host.refresh_policy(policy(1, 1).as_bytes()).unwrap());
    let pd = digest_field(&p, "policyDigest");
    let state = include_bytes!("../fixtures/temporal-state-v1.ss");
    let s = text(host.refresh_proof_state(state).unwrap());
    let sd = digest_field(&s, "stateDigest");
    let task =
        std::str::from_utf8(include_bytes!("../fixtures/temporal-derivation-v1.ss")).unwrap();
    let request = format!(
        "(object (\"schema\" \"poo-flow.temporal-proof-register-request.v1\") (\"stateIdentity\" \"mrr-production-source\") (\"expectedStateDigest\" \"{sd}\") (\"policyIdentity\" \"mrr-production-policy\") (\"task\" {task}))"
    );
    let registered = text(host.register_proof(request.as_bytes()).unwrap());
    let registration = digest_field(&registered, "registration");
    let query = current(&registration, 1, 1, &pd);
    let checked = text(host.current_proof(query.as_bytes()).unwrap());
    assert!(checked.contains("(\"status\" \"current\")"));
    assert!(checked.contains("(\"current\" #t)"));
    for flag in [
        "sourceAuthenticated",
        "selectionAdmitted",
        "actionAuthorized",
        "durable",
    ] {
        assert!(checked.contains(&format!("(\"{flag}\" #f)")));
    }
    println!("CASE original MRR derivation registered and current POO proof checked");
    assert!(
        host.current_proof(b"(object (\"schema\" \"forged\"))")
            .is_err()
    );
    assert_eq!(text(host.current_proof(query.as_bytes()).unwrap()), checked);
    host.refresh_proof_state(include_bytes!("../fixtures/temporal-corrected-state-v1.ss"))
        .unwrap();
    host.refresh_policy(policy(2, 2).as_bytes()).unwrap();
    assert!(host.current_proof(query.as_bytes()).is_err());
    let corrected = text(
        host.current_proof(current(&registration, 2, 2, &pd).as_bytes())
            .unwrap(),
    );
    assert!(corrected.contains("(\"status\" \"unsupported\")"));
    assert!(corrected.contains("(\"current\" #f)"));
    assert!(host.refresh_proof_state(state).is_err());
    assert!(host.register_proof(request.as_bytes()).is_err());
    println!("CASE late correction and stale generations reject on the same owner");
    assert_eq!(
        host.evaluate_finite_relations(3, vec![(0, 1), (1, 2)], vec![0])
            .unwrap(),
        finite
    );
    host.close().unwrap();
    println!("CASE worker graceful EOF reaped");
    let mut cancelled = NativeWorker::start(std::path::Path::new(env!(
        "CARGO_BIN_EXE_mrr-native-worker"
    )))
    .unwrap();
    assert!(matches!(
        cancelled.current_proof(query.as_bytes()),
        Err(NativeWorkerError::Native(_))
    ));
    println!("CASE restarted worker cannot reuse prior proof registration");
    cancelled.cancel();
    assert!(matches!(
        cancelled.current_proof(b"()"),
        Err(NativeWorkerError::Closed)
    ));
    println!("CASE worker cancellation is terminal");
}

#[cfg(unix)]
#[test]
fn mismatched_reply_closes_the_session() {
    use std::os::unix::fs::PermissionsExt;
    let path =
        std::env::temp_dir().join(format!("mrr-worker-invalid-reply-{}", std::process::id()));
    std::fs::write(
        &path,
        b"#!/bin/sh\nread request\nprintf '(mrr.native-worker.response.v1 99 0 \"\")\\n'\n",
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut worker = NativeWorker::start(&path).unwrap();
    let result = worker.current_proof(b"()");
    std::fs::remove_file(path).unwrap();
    assert!(matches!(result, Err(NativeWorkerError::Protocol)));
    assert!(matches!(
        worker.current_proof(b"()"),
        Err(NativeWorkerError::Closed)
    ));
    println!("CASE mismatched reply terminally rejects and reaps worker");
}

#[cfg(unix)]
#[test]
fn forged_finite_rows_fail_parent_admission_and_close_session() {
    use std::os::unix::fs::PermissionsExt;
    let path =
        std::env::temp_dir().join(format!("mrr-worker-forged-finite-{}", std::process::id()));
    std::fs::write(
        &path,
        b"#!/bin/sh\nread request\nprintf '(mrr.native-worker.response.v1 1 0 \"(list (list (list 0 99 1)) (list))\")\\n'\n",
    ).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut worker = NativeWorker::start(&path).unwrap();
    let result = worker.evaluate_finite_relations(2, vec![(0, 1)], vec![]);
    std::fs::remove_file(path).unwrap();
    assert!(matches!(
        result,
        Err(NativeWorkerError::Finite(
            mrr_gerbil::FiniteInferenceError::InvalidNativeCandidate
        ))
    ));
    assert!(matches!(
        worker.evaluate_finite_relations(2, vec![(0, 1)], vec![]),
        Err(NativeWorkerError::Closed)
    ));
    println!("CASE forged child rows fail original parent candidate admission and close worker");
}
