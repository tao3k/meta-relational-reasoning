//! Signed source authority for physical providers, independent of certification.
//! Expected binding/catalog digests and the ledger issuer are caller trust roots.
use super::{
    SignedTransformationSourceGrant, TransformationCapabilities, TransformationError,
    TransformationExecutionBudget, TransformationGrantLedger, TransformationLimits,
    TransformationSourceGrant, digest,
};
use ed25519_dalek::Signature;
use std::sync::Arc;

/// Explicit operation authorized by a source grant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransformationGrantOperation {
    Forward,
    Solve,
    Extract,
    Publish,
}

/// Authenticated current grant. This is not a transformation proof certificate.
#[derive(Clone, Debug)]
pub struct AuthenticatedTransformationGrant {
    ledger: Arc<TransformationGrantLedger>,
    grant: TransformationSourceGrant,
    support: [u8; 32],
    identity: [u8; 32],
}
impl AuthenticatedTransformationGrant {
    /// Authenticate exactly the caller's expected physical binding and catalog.
    /// The issuer comes from the separately configured persistent ledger.
    pub fn authenticate(
        signed: SignedTransformationSourceGrant,
        ledger: Arc<TransformationGrantLedger>,
        expected_binding: [u8; 32],
        expected_catalog: [u8; 32],
        limits: TransformationLimits,
    ) -> Result<Self, TransformationError> {
        let grant = signed.grant;
        if grant.issuer != ledger.key.to_bytes()
            || grant.binding != expected_binding
            || grant.catalog != expected_catalog
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
        let identity = digest(&("mrr.authenticated-source-grant.v1", &bytes), limits)?;
        let support = digest(
            &(
                "mrr.source-grant.nonce-support.v1",
                grant.issuer,
                grant.nonce,
            ),
            limits,
        )?;
        ledger.with_grant(&grant, || Ok(()))?;
        Ok(Self {
            ledger,
            grant,
            support,
            identity,
        })
    }
    /// Exact signed statement identity, including expiry, permissions and bounds.
    #[must_use]
    pub fn identity(&self) -> &[u8; 32] {
        &self.identity
    }
    #[must_use]
    pub fn binding_digest(&self) -> &[u8; 32] {
        &self.grant.binding
    }
    #[must_use]
    pub fn catalog_digest(&self) -> &[u8; 32] {
        &self.grant.catalog
    }
    #[must_use]
    pub fn support_digest(&self) -> &[u8; 32] {
        &self.support
    }
    #[must_use]
    pub fn capabilities(&self) -> TransformationCapabilities {
        self.grant.capabilities
    }
    #[must_use]
    pub fn resources(&self) -> TransformationExecutionBudget {
        self.grant.resources
    }
    /// Refresh expiry, quarantine and durable nonce revocation.
    pub fn check_current(&self) -> Result<(), TransformationError> {
        self.ledger.with_grant(&self.grant, || Ok(()))
    }
    /// Hold the ledger read lock during a synchronous authorization-sensitive
    /// action. Expiry is checked both before and after. Do not hold across await.
    pub fn with_current<T>(
        &self,
        action: impl FnOnce() -> Result<T, TransformationError>,
    ) -> Result<T, TransformationError> {
        self.ledger.with_grant(&self.grant, action)
    }
    /// Check capability and declared cumulative operation/value budgets, then
    /// hold the current lease throughout the action. The provider owns counting.
    pub fn authorize<T>(
        &self,
        operation: TransformationGrantOperation,
        operations: u64,
        value_bytes: u64,
        action: impl FnOnce() -> Result<T, TransformationError>,
    ) -> Result<T, TransformationError> {
        let caps = self.capabilities();
        let allowed = match operation {
            TransformationGrantOperation::Forward => caps.forward,
            TransformationGrantOperation::Solve => caps.solve,
            TransformationGrantOperation::Extract => caps.extract,
            TransformationGrantOperation::Publish => caps.publish,
        };
        if !allowed {
            return Err(TransformationError::Rejected);
        }
        if operations == 0
            || operations > self.resources().max_operations
            || value_bytes > self.resources().max_value_bytes
        {
            return Err(TransformationError::Budget);
        }
        self.with_current(action)
    }
    /// Persist revocation before acknowledging it. Reopened ledgers reject the
    /// same nonce; failure leaves the live owner revoked or quarantined.
    pub fn revoke(&self) -> Result<(), TransformationError> {
        self.ledger.revoke(self.grant.nonce)
    }
}
