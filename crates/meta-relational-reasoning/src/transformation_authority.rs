//! Authenticated source grants for the kernel-qualified finite runtime.
//! The configured issuer key and persistent ledger are explicit trust roots.
use super::{
    transformation::{
        TransformationBinding, TransformationDefinition, TransformationEndpoint,
        TransformationError, TransformationEvidence, TransformationLimits, TransformationStep,
        TransformationVerifier, digest,
    },
    transformation_execution::TransformationRuntime,
    transformation_finite::{FiniteTransformationCatalog, FiniteTransformationOwner},
    transformation_kernel::KernelCheckedFiniteCatalog,
};
use ed25519_dalek::{Signature, VerifyingKey};
use mrr_relation::Value;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

/// Checked arithmetic mirrors the natural-number affine contracts in Lean.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransformationAffineBound {
    pub slope: u64,
    pub offset: u64,
}
impl TransformationAffineBound {
    pub fn apply(self, size: u64) -> Result<u64, TransformationError> {
        self.slope
            .checked_mul(size)
            .and_then(|n| n.checked_add(self.offset))
            .ok_or(TransformationError::Budget)
    }
    pub fn compose(self, next: Self) -> Result<Self, TransformationError> {
        Ok(Self {
            slope: next
                .slope
                .checked_mul(self.slope)
                .ok_or(TransformationError::Budget)?,
            offset: next.apply(self.offset)?,
        })
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransformationResourceContract {
    pub size: TransformationAffineBound,
    pub cost: TransformationAffineBound,
}
impl TransformationResourceContract {
    pub fn compose(self, next: Self) -> Result<Self, TransformationError> {
        let later = self.size.compose(next.cost)?;
        Ok(Self {
            size: self.size.compose(next.size)?,
            cost: TransformationAffineBound {
                slope: self
                    .cost
                    .slope
                    .checked_add(later.slope)
                    .ok_or(TransformationError::Budget)?,
                offset: self
                    .cost
                    .offset
                    .checked_add(later.offset)
                    .ok_or(TransformationError::Budget)?,
            },
        })
    }
}
/// Exact operations permitted by an issuer. No grant permits arbitrary effects.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransformationCapabilities {
    pub forward: bool,
    pub solve: bool,
    pub extract: bool,
    pub publish: bool,
}
/// Per-plan limits on ordered table operations and each encoded integer value.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransformationExecutionBudget {
    pub max_operations: u64,
    pub max_value_bytes: u64,
}
impl TransformationExecutionBudget {
    fn value(&self, value: &Value) -> Result<(), TransformationError> {
        if !matches!(value, Value::Integer(_)) {
            return Err(TransformationError::InvalidSchema);
        }
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(value, &mut bytes).map_err(|_| TransformationError::Encoding)?;
        if bytes.len() as u64 > self.max_value_bytes {
            return Err(TransformationError::Budget);
        }
        Ok(())
    }
}
/// An unsigned payload is canonicalized by `signing_bytes` before issuer signing.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransformationSourceGrant {
    pub version: u16,
    pub issuer: [u8; 32],
    pub nonce: [u8; 32],
    pub binding: [u8; 32],
    pub catalog: [u8; 32],
    pub not_before: u64,
    pub expires: u64,
    pub capabilities: TransformationCapabilities,
    pub resources: TransformationExecutionBudget,
}
impl TransformationSourceGrant {
    pub fn signing_bytes(
        &self,
        limits: TransformationLimits,
    ) -> Result<Vec<u8>, TransformationError> {
        if self.version != 1
            || self.nonce == [0; 32]
            || self.not_before >= self.expires
            || self.resources.max_operations == 0
            || self.resources.max_value_bytes == 0
        {
            return Err(TransformationError::Rejected);
        }
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(&("mrr.source-grant.ed25519.v1", self), &mut bytes)
            .map_err(|_| TransformationError::Encoding)?;
        if bytes.len() > limits.max_bytes.get() {
            return Err(TransformationError::Budget);
        }
        Ok(bytes)
    }
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SignedTransformationSourceGrant {
    pub grant: TransformationSourceGrant,
    pub signature: Vec<u8>,
}
impl SignedTransformationSourceGrant {
    pub fn from_bytes(
        bytes: &[u8],
        limits: TransformationLimits,
    ) -> Result<Self, TransformationError> {
        if bytes.len() > limits.max_bytes.get() {
            return Err(TransformationError::Budget);
        }
        let mut input = bytes;
        let signed: Self =
            ciborium::de::from_reader(&mut input).map_err(|_| TransformationError::Encoding)?;
        if !input.is_empty() || signed.signature.len() != 64 {
            return Err(TransformationError::Encoding);
        }
        signed.grant.signing_bytes(limits)?;
        let mut canonical = Vec::new();
        ciborium::ser::into_writer(&signed, &mut canonical)
            .map_err(|_| TransformationError::Encoding)?;
        if canonical != bytes {
            return Err(TransformationError::Encoding);
        }
        Ok(signed)
    }
    pub fn to_bytes(&self, limits: TransformationLimits) -> Result<Vec<u8>, TransformationError> {
        self.grant.signing_bytes(limits)?;
        if self.signature.len() != 64 {
            return Err(TransformationError::Encoding);
        }
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(self, &mut bytes).map_err(|_| TransformationError::Encoding)?;
        if bytes.len() > limits.max_bytes.get() {
            return Err(TransformationError::Budget);
        }
        Ok(bytes)
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LedgerState {
    version: u16,
    issuer: [u8; 32],
    epoch: u64,
    revoked: BTreeSet<[u8; 32]>,
    quarantined: bool,
}
#[derive(Debug)]
struct LedgerLock(File);
impl Drop for LedgerLock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}
fn ledger_sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}
/// A single owner keeps the ledger lock through all clones and read leases.
#[derive(Debug)]
pub struct TransformationGrantLedger {
    path: PathBuf,
    _lock: LedgerLock,
    state: RwLock<LedgerState>,
    key: VerifyingKey,
    limits: TransformationLimits,
}
impl TransformationGrantLedger {
    pub fn open(
        path: impl AsRef<Path>,
        issuer: [u8; 32],
        limits: TransformationLimits,
    ) -> Result<Arc<Self>, TransformationError> {
        let key = VerifyingKey::from_bytes(&issuer).map_err(|_| TransformationError::Rejected)?;
        if key.is_weak() {
            return Err(TransformationError::Rejected);
        }
        let path = path.as_ref().to_path_buf();
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .ok_or(TransformationError::Encoding)?;
        fs::create_dir_all(parent).map_err(|_| TransformationError::Encoding)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(ledger_sidecar(&path, ".lock"))
            .map_err(|_| TransformationError::Encoding)?;
        file.try_lock().map_err(|_| TransformationError::Conflict)?;
        let lock = LedgerLock(file);
        // An interrupted revocation write has an uncertain outcome, so reopening fails closed.
        if ledger_sidecar(&path, ".pending").exists() {
            return Err(TransformationError::PublicationUncertain);
        }
        let state = match File::open(&path) {
            Ok(file) => {
                let mut bytes = Vec::new();
                file.take(limits.max_bytes.get() as u64 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|_| TransformationError::Encoding)?;
                if bytes.len() > limits.max_bytes.get() {
                    return Err(TransformationError::Budget);
                }
                let mut input = bytes.as_slice();
                let state: LedgerState = ciborium::de::from_reader(&mut input)
                    .map_err(|_| TransformationError::Encoding)?;
                let mut canonical = Vec::new();
                ciborium::ser::into_writer(&state, &mut canonical)
                    .map_err(|_| TransformationError::Encoding)?;
                if !input.is_empty()
                    || canonical != bytes
                    || state.version != 1
                    || state.issuer != issuer
                    || state.revoked.len() > limits.max_dependencies.get()
                {
                    return Err(TransformationError::Rejected);
                }
                state
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => LedgerState {
                version: 1,
                issuer,
                epoch: 0,
                revoked: BTreeSet::new(),
                quarantined: false,
            },
            Err(_) => return Err(TransformationError::Encoding),
        };
        Ok(Arc::new(Self {
            path,
            _lock: lock,
            state: RwLock::new(state),
            key,
            limits,
        }))
    }
    /// Revoke the issuer nonce durably; a failed write still revokes this live owner.
    pub fn revoke(&self, nonce: [u8; 32]) -> Result<(), TransformationError> {
        let mut state = self
            .state
            .write()
            .map_err(|_| TransformationError::Revoked)?;
        if state.quarantined {
            return Err(TransformationError::Revoked);
        }
        if state.revoked.contains(&nonce) {
            // Retrying a failed write must not falsely acknowledge durable revocation.
            return self.persist(&state);
        }
        if state.revoked.len() >= self.limits.max_dependencies.get() || state.epoch == u64::MAX {
            state.quarantined = true;
            self.persist(&state)?;
            return Err(TransformationError::Budget);
        }
        state.epoch += 1;
        state.revoked.insert(nonce);
        self.persist(&state)
    }
    fn persist(&self, state: &LedgerState) -> Result<(), TransformationError> {
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(state, &mut bytes).map_err(|_| TransformationError::Encoding)?;
        if bytes.len() > self.limits.max_bytes.get() {
            return Err(TransformationError::Budget);
        }
        let temporary = ledger_sidecar(&self.path, ".pending");
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary)
            .map_err(|_| TransformationError::Encoding)?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|_| TransformationError::Encoding)?;
        fs::rename(&temporary, &self.path).map_err(|_| TransformationError::Encoding)?;
        File::open(self.path.parent().ok_or(TransformationError::Encoding)?)
            .and_then(|f| f.sync_all())
            .map_err(|_| TransformationError::PublicationUncertain)
    }
    fn with_grant<T>(
        &self,
        grant: &TransformationSourceGrant,
        action: impl FnOnce() -> Result<T, TransformationError>,
    ) -> Result<T, TransformationError> {
        let state = self
            .state
            .read()
            .map_err(|_| TransformationError::Revoked)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| TransformationError::Unknown)?
            .as_secs();
        if state.quarantined || state.issuer != grant.issuer || state.revoked.contains(&grant.nonce)
        {
            return Err(TransformationError::Revoked);
        }
        if now < grant.not_before || now >= grant.expires {
            return Err(TransformationError::Revoked);
        }
        let result = action()?;
        // A long action may cross expiry even while revocation is locked out.
        if SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| TransformationError::Unknown)?
            .as_secs()
            >= grant.expires
        {
            return Err(TransformationError::Revoked);
        }
        Ok(result)
    }
}
/// Content identity of the authentication/execution/publication adapters and dependency lock.
pub fn transformation_authority_implementation_digest() -> [u8; 32] {
    let files: &[&[u8]] = &[
        include_bytes!("transformation_authority.rs"),
        include_bytes!("transformation_execution.rs"),
        include_bytes!("transformation_store.rs"),
        include_bytes!("transformation_profiles.rs"),
        include_bytes!("../../../Cargo.lock"),
    ];
    let mut hash = Sha256::new();
    hash.update(b"mrr.authenticated-kernel-implementation.v1");
    for file in files {
        hash.update(Sha256::digest(file));
    }
    hash.finalize().into()
}
/// Exact catalog plus pinned kernel identity; callers cannot sign a different artifact under it.
pub fn transformation_grant_catalog_digest(
    catalog: &KernelCheckedFiniteCatalog,
    limits: TransformationLimits,
) -> Result<[u8; 32], TransformationError> {
    digest(
        &(
            "mrr.kernel-grant-catalog.v1",
            catalog.native().to_bytes()?,
            catalog.policy(),
            transformation_authority_implementation_digest(),
            catalog.certificate().source_digest(),
            catalog.certificate().kernel_digest(),
        ),
        limits,
    )
}
pub fn transformation_grant_binding_digest(
    binding: &TransformationBinding,
    limits: TransformationLimits,
) -> Result<[u8; 32], TransformationError> {
    digest(&("mrr.source-grant.binding.v1", binding), limits)
}
/// Opaque live authority retained by result stores; freshness observes grant revocation/expiry.
#[derive(Clone, Debug)]
pub struct TransformationAuthorityLease {
    native: FiniteTransformationCatalog,
    authenticated: Option<(Arc<TransformationGrantLedger>, TransformationSourceGrant)>,
}
impl TransformationAuthorityLease {
    pub(super) fn native(native: &FiniteTransformationCatalog) -> Self {
        Self {
            native: native.clone(),
            authenticated: None,
        }
    }
    pub fn is_current(&self) -> bool {
        self.with_current(self.native.selected_binding(), || Ok(()))
            .is_ok()
    }
    pub fn with_current<T>(
        &self,
        binding: &TransformationBinding,
        action: impl FnOnce() -> Result<T, TransformationError>,
    ) -> Result<T, TransformationError> {
        match &self.authenticated {
            Some((ledger, grant)) => {
                ledger.with_grant(grant, || self.native.with_current(binding, action))
            }
            None => self.native.with_current(binding, action),
        }
    }
}
/// Signature authentication supplements the actual kernel checker, never replacing it.
#[derive(Clone, Debug)]
pub struct AuthenticatedFiniteTransformationCatalog {
    kernel: KernelCheckedFiniteCatalog,
    lease: TransformationAuthorityLease,
    policy: [u8; 32],
    grant_support: [u8; 32],
}
impl AuthenticatedFiniteTransformationCatalog {
    pub fn authenticate(
        kernel: KernelCheckedFiniteCatalog,
        signed: SignedTransformationSourceGrant,
        ledger: Arc<TransformationGrantLedger>,
        limits: TransformationLimits,
    ) -> Result<Self, TransformationError> {
        let grant = signed.grant;
        if grant.issuer != ledger.key.to_bytes()
            || grant.binding
                != transformation_grant_binding_digest(kernel.native().selected_binding(), limits)?
            || grant.catalog != transformation_grant_catalog_digest(&kernel, limits)?
        {
            return Err(TransformationError::BindingMismatch);
        }
        let signature: [u8; 64] = signed
            .signature
            .try_into()
            .map_err(|_| TransformationError::Rejected)?;
        let bytes = grant.signing_bytes(limits)?;
        ledger
            .key
            .verify_strict(&bytes, &Signature::from_bytes(&signature))
            .map_err(|_| TransformationError::Rejected)?;
        let policy = digest(
            &("mrr.authenticated-kernel.v1", kernel.policy(), bytes),
            limits,
        )?;
        ledger.with_grant(&grant, || Ok(()))?;
        let grant_support = digest(
            &(
                "mrr.source-grant.nonce-support.v1",
                grant.issuer,
                grant.nonce,
            ),
            limits,
        )?;
        let lease = TransformationAuthorityLease {
            native: kernel.native().clone(),
            authenticated: Some((ledger, grant)),
        };
        Ok(Self {
            kernel,
            lease,
            policy,
            grant_support,
        })
    }
    pub fn grant_support(&self) -> &[u8; 32] {
        &self.grant_support
    }
    /// Persist issuer-nonce revocation first, then invalidate this result's reverse support index.
    /// A result-write failure still leaves every live lease revoked.
    pub fn revoke_grant(
        &self,
        store: &mut super::transformation_store::TransformationResultStore,
    ) -> Result<bool, TransformationError> {
        let (ledger, grant) = self
            .lease
            .authenticated
            .as_ref()
            .ok_or(TransformationError::Rejected)?;
        ledger.revoke(grant.nonce)?;
        store.invalidate(&self.grant_support)
    }
    pub fn evidence(&self, ordinal: usize) -> Result<TransformationEvidence, TransformationError> {
        self.allowed(self.kernel.native().selected_binding(), true, || {
            self.kernel.evidence(ordinal)
        })
    }

    fn allowed<T>(
        &self,
        binding: &TransformationBinding,
        allowed: bool,
        action: impl FnOnce() -> Result<T, TransformationError>,
    ) -> Result<T, TransformationError> {
        if !allowed {
            return Err(TransformationError::Rejected);
        }
        // Native runtime performs its own native read lock. Avoid nested native guards.
        let (ledger, grant) = self
            .lease
            .authenticated
            .as_ref()
            .ok_or(TransformationError::Rejected)?;
        ledger.with_grant(grant, || {
            if binding != self.kernel.native().selected_binding() {
                return Err(TransformationError::BindingMismatch);
            }
            action()
        })
    }
    fn resources(&self) -> TransformationExecutionBudget {
        self.lease
            .authenticated
            .as_ref()
            .expect("authenticated construction")
            .1
            .resources
    }
    fn caps(&self) -> TransformationCapabilities {
        self.lease
            .authenticated
            .as_ref()
            .expect("authenticated construction")
            .1
            .capabilities
    }
}
impl TransformationVerifier for AuthenticatedFiniteTransformationCatalog {
    fn policy(&self) -> [u8; 32] {
        self.policy
    }
    fn check_definition(
        &self,
        d: &TransformationDefinition,
        e: &TransformationEvidence,
        b: &TransformationBinding,
    ) -> Result<[u8; 32], TransformationError> {
        self.allowed(b, true, || self.kernel.check_definition(d, e, b))
    }
    fn check_step(
        &self,
        step: &TransformationStep,
        b: &TransformationBinding,
        ordinal: usize,
        prior: &[u8; 32],
    ) -> Result<[u8; 32], TransformationError> {
        let operations = u64::try_from(ordinal)
            .ok()
            .and_then(|n| n.checked_add(1))
            .and_then(|n| n.checked_mul(3))
            .and_then(|n| n.checked_add(2))
            .ok_or(TransformationError::Budget)?;
        if operations > self.resources().max_operations {
            return Err(TransformationError::Budget);
        }
        self.allowed(b, self.caps().forward && self.caps().extract, || {
            self.kernel.check_step(step, b, ordinal, prior)
        })
    }
    fn check_solver(
        &self,
        target: &TransformationEndpoint,
        solver: &[u8; 32],
        input: &[u8; 32],
        b: &TransformationBinding,
        prior: &[u8; 32],
    ) -> Result<[u8; 32], TransformationError> {
        self.allowed(b, self.caps().solve, || {
            self.kernel.check_solver(target, solver, input, b, prior)
        })
    }
}
impl TransformationRuntime for AuthenticatedFiniteTransformationCatalog {
    fn identity(&self) -> [u8; 32] {
        self.policy
    }
    fn forward(
        &self,
        s: &TransformationStep,
        i: &Value,
        b: &TransformationBinding,
    ) -> Result<Value, TransformationError> {
        self.resources().value(i)?;
        self.allowed(b, self.caps().forward, || self.kernel.forward(s, i, b))
            .and_then(|value| {
                self.resources().value(&value)?;
                Ok(value)
            })
    }
    fn solve(
        &self,
        solver: &[u8; 32],
        target: &TransformationEndpoint,
        i: &Value,
        b: &TransformationBinding,
    ) -> Result<Value, TransformationError> {
        self.resources().value(i)?;
        self.allowed(b, self.caps().solve, || {
            self.kernel.solve(solver, target, i, b)
        })
        .and_then(|value| {
            self.resources().value(&value)?;
            Ok(value)
        })
    }
    fn extract(
        &self,
        s: &TransformationStep,
        i: &Value,
        a: &Value,
        b: &TransformationBinding,
    ) -> Result<Value, TransformationError> {
        self.resources().value(i)?;
        self.resources().value(a)?;
        self.allowed(b, self.caps().extract, || self.kernel.extract(s, i, a, b))
            .and_then(|value| {
                self.resources().value(&value)?;
                Ok(value)
            })
    }
    fn check_answer(
        &self,
        e: &TransformationEndpoint,
        i: &Value,
        a: &Value,
        b: &TransformationBinding,
    ) -> Result<[u8; 32], TransformationError> {
        self.resources().value(i)?;
        self.resources().value(a)?;
        self.allowed(b, true, || self.kernel.check_answer(e, i, a, b))
    }
}
impl FiniteTransformationOwner for AuthenticatedFiniteTransformationCatalog {
    fn authority_supports(&self) -> Vec<[u8; 32]> {
        vec![self.policy, self.grant_support]
    }
    fn authority(&self) -> &FiniteTransformationCatalog {
        self.kernel.native()
    }
    fn authority_lease(&self) -> TransformationAuthorityLease {
        self.lease.clone()
    }
    fn publication_lease(&self) -> Result<TransformationAuthorityLease, TransformationError> {
        if !self.caps().publish {
            return Err(TransformationError::Rejected);
        }
        Ok(self.lease.clone())
    }
}
