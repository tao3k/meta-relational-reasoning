//! Point-of-use checks over source admission and an external authority observation.
//! The authority owner authenticates policy, revocation and time. This module
//! creates no Session, clock, capability, durable permit or effect authorization.

use super::{
    AdmittedAgenticAiContext, AdmittedAgenticAiContextMaterialization,
    AgenticAiContextAdmissionError, AgenticAiContextManifest,
};
use crate::{AgenticAiContextContract, ReasoningBundle, SemanticSnapshot};

/// The owner must observe permission for this particular use, not a prior turn.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgenticAiContextUsePurpose {
    Display,
    Action,
}

/// Current policy/time assessment supplied by the designated authority owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgenticAiContextUseDecision {
    Allowed,
    Denied,
    Revoked,
    Expired,
}

/// An observation is a declaration, not an authenticated MRR authorization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgenticAiContextUseObservation {
    pub manifest_digest: [u8; 32],
    pub purpose: AgenticAiContextUsePurpose,
    pub current_contract: AgenticAiContextContract,
    pub decision: AgenticAiContextUseDecision,
}

/// POO Flow / runtime authority adapter. Implementations must obtain a fresh,
/// authenticated decision and the effective contract at the actual use boundary.
/// Caller-supplied source snapshots must be current for that same boundary.
pub trait AgenticAiContextUseAuthority {
    type Error;
    fn observe(
        &self,
        manifest: &AgenticAiContextManifest,
        purpose: AgenticAiContextUsePurpose,
    ) -> Result<AgenticAiContextUseObservation, Self::Error>;
}

/// Typed refusal; adapter failures retain their original cause.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgenticAiContextUseError<E> {
    Source(AgenticAiContextAdmissionError),
    Authority(E),
    ObservationMismatch,
    ContractMismatch,
    Denied,
    Revoked,
    Expired,
}

fn check_observation<A: AgenticAiContextUseAuthority>(
    manifest: &AgenticAiContextManifest,
    purpose: AgenticAiContextUsePurpose,
    authority: &A,
) -> Result<(), AgenticAiContextUseError<A::Error>> {
    let observed = authority
        .observe(manifest, purpose)
        .map_err(AgenticAiContextUseError::Authority)?;
    if observed.manifest_digest != *manifest.digest() || observed.purpose != purpose {
        return Err(AgenticAiContextUseError::ObservationMismatch);
    }
    // Full equality includes actor, task, policy, required/temporal receipts and
    // completeness. Matching bytes or a shared generation cannot widen scope.
    if observed.current_contract != manifest.record().contract {
        return Err(AgenticAiContextUseError::ContractMismatch);
    }
    match observed.decision {
        AgenticAiContextUseDecision::Allowed => Ok(()),
        AgenticAiContextUseDecision::Denied => Err(AgenticAiContextUseError::Denied),
        AgenticAiContextUseDecision::Revoked => Err(AgenticAiContextUseError::Revoked),
        AgenticAiContextUseDecision::Expired => Err(AgenticAiContextUseError::Expired),
    }
}

impl AdmittedAgenticAiContext {
    /// Re-enter at every use, including after restore. Success is a point-in-time
    /// check and must not be stored as a permit. The runtime coordinates changes
    /// between observation and disclosure/effect commit; this is not atomic IO.
    pub fn check_current_use<A: AgenticAiContextUseAuthority>(
        &self,
        bundle: &ReasoningBundle,
        snapshot: &SemanticSnapshot,
        purpose: AgenticAiContextUsePurpose,
        authority: &A,
    ) -> Result<(), AgenticAiContextUseError<A::Error>> {
        self.check_source(bundle, snapshot)
            .map_err(AgenticAiContextUseError::Source)?;
        check_observation(self.manifest(), purpose, authority)
    }
}

impl AdmittedAgenticAiContextMaterialization {
    /// Byte identity does not bypass current source, policy, revocation or expiry.
    pub fn check_current_use<A: AgenticAiContextUseAuthority>(
        &self,
        bundle: &ReasoningBundle,
        snapshot: &SemanticSnapshot,
        purpose: AgenticAiContextUsePurpose,
        authority: &A,
    ) -> Result<(), AgenticAiContextUseError<A::Error>> {
        self.check_source(bundle, snapshot)
            .map_err(AgenticAiContextUseError::Source)?;
        check_observation(self.manifest(), purpose, authority)
    }
}

#[cfg(feature = "agentic-ai-context-tokens")]
impl super::SourceBoundAgenticAiContextTokens {
    /// Serving must check current use independently of computational eligibility.
    pub fn check_current_use<A: AgenticAiContextUseAuthority>(
        &self,
        bundle: &ReasoningBundle,
        snapshot: &SemanticSnapshot,
        purpose: AgenticAiContextUsePurpose,
        authority: &A,
    ) -> Result<(), AgenticAiContextUseError<A::Error>> {
        self.presentation()
            .check_current_use(bundle, snapshot, purpose, authority)
    }
}

impl<E: std::fmt::Debug> std::fmt::Display for AgenticAiContextUseError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for AgenticAiContextUseError<E> {}
