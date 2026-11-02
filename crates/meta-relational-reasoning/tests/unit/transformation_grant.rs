use super::limits;
use crate::{
    AuthenticatedTransformationGrant as Auth, SignedTransformationSourceGrant,
    TransformationCapabilities, TransformationError, TransformationExecutionBudget,
    TransformationGrantLedger, TransformationGrantOperation as Op, TransformationSourceGrant,
};
use ed25519_dalek::{Signer, SigningKey};

#[test]
fn physical_grant_checks_signature_cut_capabilities_budgets_and_durable_revocation() {
    let limits = limits();
    let key = SigningKey::from_bytes(&[193; 32]);
    let path = std::env::temp_dir().join(format!("mrr-physical-grant-{}.cbor", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let ledger =
        TransformationGrantLedger::open(&path, key.verifying_key().to_bytes(), limits).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let grant = TransformationSourceGrant {
        version: 1,
        issuer: key.verifying_key().to_bytes(),
        nonce: [194; 32],
        binding: [195; 32],
        catalog: [196; 32],
        not_before: now,
        expires: now + 3600,
        capabilities: TransformationCapabilities {
            forward: true,
            solve: true,
            extract: true,
            publish: false,
        },
        resources: TransformationExecutionBudget {
            max_operations: 5,
            max_value_bytes: 100,
        },
    };
    let signed = SignedTransformationSourceGrant {
        signature: key
            .sign(&grant.signing_bytes(limits).unwrap())
            .to_bytes()
            .to_vec(),
        grant,
    };
    let mut forged = signed.clone();
    forged.signature[0] ^= 1;
    assert!(Auth::authenticate(forged, ledger.clone(), [195; 32], [196; 32], limits).is_err());
    assert!(
        Auth::authenticate(signed.clone(), ledger.clone(), [197; 32], [196; 32], limits).is_err()
    );
    assert!(
        Auth::authenticate(signed.clone(), ledger.clone(), [195; 32], [197; 32], limits).is_err()
    );
    let auth =
        Auth::authenticate(signed.clone(), ledger.clone(), [195; 32], [196; 32], limits).unwrap();
    assert_eq!(auth.authorize(Op::Solve, 1, 100, || Ok(7)).unwrap(), 7);
    let calls = std::cell::Cell::new(0);
    for (op, count, bytes) in [(Op::Publish, 1, 1), (Op::Solve, 6, 1), (Op::Solve, 1, 101)] {
        assert!(
            auth.authorize(op, count, bytes, || {
                calls.set(calls.get() + 1);
                Ok(())
            })
            .is_err()
        );
    }
    assert_eq!(calls.get(), 0);
    auth.revoke().unwrap();
    assert_eq!(auth.check_current(), Err(TransformationError::Revoked));
    drop(auth);
    drop(ledger);
    let reopened =
        TransformationGrantLedger::open(&path, key.verifying_key().to_bytes(), limits).unwrap();
    assert!(matches!(
        Auth::authenticate(signed, reopened, [195; 32], [196; 32], limits),
        Err(TransformationError::Revoked)
    ));
    std::fs::remove_file(path).unwrap();
}
