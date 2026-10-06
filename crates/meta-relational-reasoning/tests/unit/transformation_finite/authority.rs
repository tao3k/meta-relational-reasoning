use super::{
    TransformationError, TransformationPlanCandidate, TransformationResultStore, TruthStatus,
    Value, admit_transformation, admit_transformation_plan, finite_catalog, limits,
};

#[test]
fn authenticated_kernel_grants_enforce_signature_binding_operations_and_durable_revocation() {
    use crate::{
        AuthenticatedFiniteTransformationCatalog as Auth, SignedTransformationSourceGrant,
        TransformationCapabilities, TransformationExecutionBudget, TransformationGrantLedger,
        TransformationSourceGrant, TransformationVerifier, transformation_grant_binding_digest,
        transformation_grant_catalog_digest,
    };
    use ed25519_dalek::{Signer, SigningKey};
    let (native, mut candidate) = finite_catalog();
    let mut bounded = limits();
    bounded.max_bytes = std::num::NonZeroUsize::new(65536).unwrap();
    let elan = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|p| p.join("elan"))
        .find(|p| p.is_file())
        .unwrap();
    let checked = crate::KernelCheckedFiniteCatalog::check(
        ciborium::de::from_reader(native.to_bytes().unwrap().as_slice()).unwrap(),
        candidate.binding.clone(),
        bounded,
        &elan,
        &std::env::temp_dir(),
    )
    .unwrap();
    let key = SigningKey::from_bytes(&[42; 32]);
    let path = std::env::temp_dir().join(format!("mrr-grants-{}.cbor", std::process::id()));
    let archive = path.with_extension("answer");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&archive);
    let ledger =
        TransformationGrantLedger::open(&path, key.verifying_key().to_bytes(), bounded).unwrap();
    assert!(matches!(
        TransformationGrantLedger::open(&path, key.verifying_key().to_bytes(), bounded),
        Err(TransformationError::Conflict)
    ));
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let grant = TransformationSourceGrant {
        version: 1,
        issuer: key.verifying_key().to_bytes(),
        nonce: [13; 32],
        binding: transformation_grant_binding_digest(&candidate.binding, bounded).unwrap(),
        catalog: transformation_grant_catalog_digest(&checked, bounded).unwrap(),
        not_before: now,
        expires: now + 3600,
        capabilities: TransformationCapabilities {
            forward: true,
            solve: true,
            extract: true,
            publish: true,
        },
        resources: TransformationExecutionBudget {
            max_operations: 5,
            max_value_bytes: 64,
        },
    };
    let sign = |g: TransformationSourceGrant| SignedTransformationSourceGrant {
        signature: key
            .sign(&g.signing_bytes(bounded).unwrap())
            .to_bytes()
            .to_vec(),
        grant: g,
    };
    let wire = sign(grant.clone()).to_bytes(bounded).unwrap();
    assert_eq!(
        SignedTransformationSourceGrant::from_bytes(&wire, bounded).unwrap(),
        sign(grant.clone())
    );
    let mut trailing = wire.clone();
    trailing.push(0);
    assert_eq!(
        SignedTransformationSourceGrant::from_bytes(&trailing, bounded).unwrap_err(),
        TransformationError::Encoding
    );
    let mut tampered = sign(grant.clone());
    tampered.grant.capabilities.publish = false;
    assert_eq!(
        Auth::authenticate(checked.clone(), tampered, ledger.clone(), bounded).unwrap_err(),
        TransformationError::Rejected
    );
    let mut different = grant.clone();
    different.binding = [99; 32];
    assert_eq!(
        Auth::authenticate(checked.clone(), sign(different), ledger.clone(), bounded).unwrap_err(),
        TransformationError::BindingMismatch
    );
    let mut expired = grant.clone();
    expired.not_before = now - 2;
    expired.expires = now - 1;
    assert_eq!(
        Auth::authenticate(checked.clone(), sign(expired), ledger.clone(), bounded).unwrap_err(),
        TransformationError::Revoked
    );
    let mut future = grant.clone();
    future.not_before = now + 3600;
    future.expires = now + 7200;
    assert_eq!(
        Auth::authenticate(checked.clone(), sign(future), ledger.clone(), bounded).unwrap_err(),
        TransformationError::Revoked
    );
    let authenticated = Auth::authenticate(
        checked.clone(),
        sign(grant.clone()),
        ledger.clone(),
        bounded,
    )
    .unwrap();
    assert_ne!(authenticated.policy(), checked.policy());
    let bind_candidate = |owner: &Auth, c: &mut TransformationPlanCandidate| {
        for (i, step) in c.steps.iter_mut().enumerate() {
            step.admission = admit_transformation(
                step.admission.definition(),
                &owner.evidence(i).unwrap(),
                &c.binding,
                bounded,
                owner,
            )
            .unwrap();
        }
    };
    bind_candidate(&authenticated, &mut candidate);
    let mut store = TransformationResultStore::open(&archive, bounded).unwrap();
    let result = store
        .execute_and_publish(&candidate, Value::Integer(1), &authenticated)
        .unwrap();
    assert_eq!(result.answer(), &Value::Integer(1));
    assert_eq!(store.freshness(), TruthStatus::True);
    let mut denied = grant.clone();
    denied.capabilities.publish = false;
    let denied =
        Auth::authenticate(checked.clone(), sign(denied), ledger.clone(), bounded).unwrap();
    let mut blocked = candidate.clone();
    bind_candidate(&denied, &mut blocked);
    assert_eq!(
        store
            .execute_and_publish(&blocked, Value::Integer(1), &denied)
            .unwrap_err(),
        TransformationError::Rejected
    );
    assert_eq!(store.historical_answer(), Some(&Value::Integer(1)));
    let mut restricted = grant.clone();
    restricted.resources.max_operations = 4;
    let restricted =
        Auth::authenticate(checked.clone(), sign(restricted), ledger.clone(), bounded).unwrap();
    bind_candidate(&restricted, &mut blocked);
    assert_eq!(
        admit_transformation_plan(&blocked, bounded, &restricted).unwrap_err(),
        TransformationError::Budget
    );
    let mut denied_solver = grant.clone();
    denied_solver.capabilities.solve = false;
    let denied_solver = Auth::authenticate(
        checked.clone(),
        sign(denied_solver),
        ledger.clone(),
        bounded,
    )
    .unwrap();
    bind_candidate(&denied_solver, &mut blocked);
    assert_eq!(
        admit_transformation_plan(&blocked, bounded, &denied_solver).unwrap_err(),
        TransformationError::Rejected
    );
    let mut small = grant.clone();
    small.resources.max_value_bytes = 1;
    let small = Auth::authenticate(checked.clone(), sign(small), ledger.clone(), bounded).unwrap();
    bind_candidate(&small, &mut blocked);
    assert_eq!(
        store
            .execute_and_publish(&blocked, Value::Integer(1), &small)
            .unwrap_err(),
        TransformationError::Budget
    );
    drop(small);
    {
        let mut expiring = grant.clone();
        expiring.nonce = [14; 32];
        expiring.expires = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 2;
        let deadline = expiring.expires;
        let expiring =
            Auth::authenticate(checked.clone(), sign(expiring), ledger.clone(), bounded).unwrap();
        bind_candidate(&expiring, &mut blocked);
        store
            .execute_and_publish(&blocked, Value::Integer(1), &expiring)
            .unwrap();
        assert_eq!(store.freshness(), TruthStatus::True);
        while std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            < deadline
        {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(store.freshness(), TruthStatus::Stale);
        assert_eq!(
            store
                .replay(&blocked, Value::Integer(1), &expiring)
                .unwrap_err(),
            TransformationError::Revoked
        );
        assert_eq!(store.historical_answer(), Some(&Value::Integer(1)));
    }
    store
        .execute_and_publish(&candidate, Value::Integer(1), &authenticated)
        .unwrap();
    assert_eq!(store.affected_plans(authenticated.grant_support()).len(), 1);
    ledger.revoke(grant.nonce).unwrap();
    assert_eq!(store.freshness(), TruthStatus::Stale);
    assert_eq!(
        authenticated.evidence(0).unwrap_err(),
        TransformationError::Revoked
    );
    assert_eq!(
        store
            .replay(&candidate, Value::Integer(1), &authenticated)
            .unwrap_err(),
        TransformationError::Revoked
    );
    assert_eq!(store.historical_answer(), Some(&Value::Integer(1)));
    drop(store);
    drop(authenticated);
    drop(denied);
    drop(restricted);
    drop(denied_solver);
    drop(ledger);
    let reopened =
        TransformationGrantLedger::open(&path, key.verifying_key().to_bytes(), bounded).unwrap();
    assert_eq!(
        Auth::authenticate(
            checked.clone(),
            sign(grant.clone()),
            reopened.clone(),
            bounded
        )
        .unwrap_err(),
        TransformationError::Revoked
    );
    drop(reopened);
    // Failed writes cannot be acknowledged by a later idempotent retry.
    let failure_path = path.with_extension("failure");
    let failed =
        TransformationGrantLedger::open(&failure_path, key.verifying_key().to_bytes(), bounded)
            .unwrap();
    let mut other_grant = grant.clone();
    other_grant.nonce = [45; 32];
    let failed_owner = Auth::authenticate(
        checked.clone(),
        sign(other_grant.clone()),
        failed.clone(),
        bounded,
    )
    .unwrap();
    let pending = std::path::PathBuf::from(format!("{}.pending", failure_path.display()));
    std::fs::create_dir(&pending).unwrap();
    assert_eq!(
        failed.revoke(other_grant.nonce),
        Err(TransformationError::Encoding)
    );
    assert_eq!(
        failed_owner.evidence(0).unwrap_err(),
        TransformationError::Revoked
    );
    assert_eq!(
        failed.revoke(other_grant.nonce),
        Err(TransformationError::Encoding)
    );
    std::fs::remove_dir(&pending).unwrap();
    failed.revoke(other_grant.nonce).unwrap();
    drop(failed_owner);
    drop(failed);
    let reopened_failure =
        TransformationGrantLedger::open(&failure_path, key.verifying_key().to_bytes(), bounded)
            .unwrap();
    assert_eq!(
        Auth::authenticate(
            checked.clone(),
            sign(other_grant.clone()),
            reopened_failure.clone(),
            bounded
        )
        .unwrap_err(),
        TransformationError::Revoked
    );
    drop(reopened_failure);
    std::fs::remove_file(&failure_path).unwrap();
    let _ = std::fs::remove_file(format!("{}.lock", failure_path.display()));
    // Exhausting revocation capacity disables authority, including after restart.
    let quota_path = path.with_extension("quota");
    let mut quota = bounded;
    quota.max_dependencies = std::num::NonZeroUsize::new(1).unwrap();
    let limited =
        TransformationGrantLedger::open(&quota_path, key.verifying_key().to_bytes(), quota)
            .unwrap();
    let quota_owner = Auth::authenticate(
        checked.clone(),
        sign(other_grant.clone()),
        limited.clone(),
        bounded,
    )
    .unwrap();
    limited.revoke([44; 32]).unwrap();
    assert_eq!(
        limited.revoke(other_grant.nonce),
        Err(TransformationError::Budget)
    );
    assert_eq!(
        quota_owner.evidence(0).unwrap_err(),
        TransformationError::Revoked
    );
    drop(quota_owner);
    drop(limited);
    let reopened_quota =
        TransformationGrantLedger::open(&quota_path, key.verifying_key().to_bytes(), quota)
            .unwrap();
    assert_eq!(
        Auth::authenticate(checked, sign(other_grant), reopened_quota.clone(), bounded)
            .unwrap_err(),
        TransformationError::Revoked
    );
    drop(reopened_quota);
    std::fs::remove_file(&quota_path).unwrap();
    let _ = std::fs::remove_file(format!("{}.lock", quota_path.display()));
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_file(&archive).unwrap();
    let _ = std::fs::remove_file(format!("{}.lock", path.display()));
    let _ = std::fs::remove_file(archive.with_extension("lock"));
}
