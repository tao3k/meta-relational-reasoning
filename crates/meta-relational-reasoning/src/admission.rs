//! Atomic composition of evaluated closure candidates into canonical MRR objects.

use std::{collections::BTreeSet, fmt};

use mrr_ascent::{ClosureReceipt, ClosureStatus, DerivationCandidate};
use mrr_identity::{DerivationId, FactId, GenerationId, RulePackId};
use mrr_lineage::{Derivation, LineageError};
use mrr_relation::{
    EvidenceCompleteness, Fact, FactProvenance, FactValidity, RelationAuthority, RelationContext,
};
use mrr_transition::{Transition, TransitionError};
use sha2::{Digest, Sha256};

const DERIVATION_RECEIPT_SCHEMA: &[u8] = b"mrr.materialized-derivation-receipt.v1";

/// Caller-owned stable identities assigned to one sorted closure candidate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CandidateIdentities {
    fact: FactId,
    derivation: DerivationId,
}

impl CandidateIdentities {
    /// Binds a fact identity and a derivation identity to one candidate.
    #[must_use]
    pub const fn new(fact: FactId, derivation: DerivationId) -> Self {
        Self { fact, derivation }
    }

    /// Returns the identity assigned to the materialized fact.
    #[must_use]
    pub const fn fact(self) -> FactId {
        self.fact
    }

    /// Returns the identity assigned to the admitted derivation.
    #[must_use]
    pub const fn derivation(self) -> DerivationId {
        self.derivation
    }
}

/// Fail-closed reasons why a closure receipt cannot be materialized atomically.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClosureAdmissionError {
    /// A truncated receipt cannot define a complete semantic-generation delta.
    IncompleteReceipt,
    /// Every sorted candidate must have exactly one caller-assigned identity pair.
    IdentityCountMismatch {
        candidates: usize,
        identities: usize,
    },
    /// Fact identities must be unique within one admission attempt.
    DuplicateFactId(FactId),
    /// Derivation identities must be unique within one admission attempt.
    DuplicateDerivationId(DerivationId),
    /// Candidates may only be admitted into the generation at which they were evaluated.
    GenerationMismatch {
        expected: GenerationId,
        actual: GenerationId,
    },
    /// The canonical lineage owner rejected a candidate.
    Lineage(LineageError),
    /// The canonical transition owner rejected the complete delta.
    Transition(TransitionError),
}

impl fmt::Display for ClosureAdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ClosureAdmissionError {}

/// Fully validated lineage and transition outputs from one admission attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterializedClosure {
    transition: Transition,
    derivations: Vec<Derivation>,
    receipt: DerivationReceipt,
}

impl MaterializedClosure {
    /// Returns the canonical immutable semantic-generation delta.
    #[must_use]
    pub const fn transition(&self) -> &Transition {
        &self.transition
    }

    /// Returns derivations in the deterministic candidate order.
    #[must_use]
    pub fn derivations(&self) -> &[Derivation] {
        &self.derivations
    }

    /// Returns the immutable identity-complete derivation receipt.
    #[must_use]
    pub const fn receipt(&self) -> &DerivationReceipt {
        &self.receipt
    }
}

/// Identity-complete receipt produced only after lineage and transition admission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivationReceipt {
    rule_pack: RulePackId,
    input_generation: GenerationId,
    input_fact_ids: Vec<FactId>,
    derived_fact_ids: Vec<FactId>,
    derivation_ids: Vec<DerivationId>,
    closure_status: ClosureStatus,
    digest: [u8; 32],
}

impl DerivationReceipt {
    #[must_use]
    pub const fn rule_pack(&self) -> RulePackId {
        self.rule_pack
    }

    #[must_use]
    pub const fn input_generation(&self) -> GenerationId {
        self.input_generation
    }

    #[must_use]
    pub fn input_fact_ids(&self) -> &[FactId] {
        &self.input_fact_ids
    }

    #[must_use]
    pub fn derived_fact_ids(&self) -> &[FactId] {
        &self.derived_fact_ids
    }

    #[must_use]
    pub fn derivation_ids(&self) -> &[DerivationId] {
        &self.derivation_ids
    }

    #[must_use]
    pub const fn closure_status(&self) -> ClosureStatus {
        self.closure_status
    }

    #[must_use]
    pub const fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
}

/// Atomically validates and materializes a complete closure receipt.
///
/// This function does not allocate identities or mutate live state. It returns
/// the complete transition and lineage collection only after every canonical
/// owner has accepted its respective object.
pub fn admit_closure_candidates(
    receipt: &ClosureReceipt,
    from: GenerationId,
    to: GenerationId,
    identities: &[CandidateIdentities],
) -> Result<MaterializedClosure, ClosureAdmissionError> {
    validate_receipt_binding(receipt, to, identities)?;
    let derivations = build_derivations(receipt.candidates(), to, identities)?;
    let insertions = derivations
        .iter()
        .map(|derivation| derivation.output().clone())
        .collect();
    let transition = Transition::new(from, to, insertions, Vec::new())
        .map_err(ClosureAdmissionError::Transition)?;
    let receipt = materialized_receipt(receipt, identities);
    Ok(MaterializedClosure {
        transition,
        derivations,
        receipt,
    })
}

fn materialized_receipt(
    closure: &ClosureReceipt,
    identities: &[CandidateIdentities],
) -> DerivationReceipt {
    let derived_fact_ids: Vec<_> = identities.iter().map(|identity| identity.fact()).collect();
    let derivation_ids: Vec<_> = identities
        .iter()
        .map(|identity| identity.derivation())
        .collect();
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, DERIVATION_RECEIPT_SCHEMA);
    hash_field(&mut hasher, closure.digest().as_bytes());
    hash_len(&mut hasher, derived_fact_ids.len());
    for identity in &derived_fact_ids {
        hash_field(&mut hasher, identity.digest_bytes());
    }
    hash_len(&mut hasher, derivation_ids.len());
    for identity in &derivation_ids {
        hash_field(&mut hasher, identity.digest_bytes());
    }
    DerivationReceipt {
        rule_pack: closure.rule_pack(),
        input_generation: closure.input_generation(),
        input_fact_ids: closure.input_fact_ids().to_vec(),
        derived_fact_ids,
        derivation_ids,
        closure_status: closure.status(),
        digest: hasher.finalize().into(),
    }
}

fn hash_len(hasher: &mut Sha256, len: usize) {
    hasher.update((len as u64).to_be_bytes());
}

fn hash_field(hasher: &mut Sha256, bytes: &[u8]) {
    hash_len(hasher, bytes.len());
    hasher.update(bytes);
}

fn validate_receipt_binding(
    receipt: &ClosureReceipt,
    to: GenerationId,
    identities: &[CandidateIdentities],
) -> Result<(), ClosureAdmissionError> {
    if receipt.status() != ClosureStatus::Complete {
        return Err(ClosureAdmissionError::IncompleteReceipt);
    }
    if receipt.candidates().len() != identities.len() {
        return Err(ClosureAdmissionError::IdentityCountMismatch {
            candidates: receipt.candidates().len(),
            identities: identities.len(),
        });
    }
    validate_unique_identities(identities)?;
    if let Some(actual) = receipt
        .candidates()
        .iter()
        .map(DerivationCandidate::generation)
        .find(|generation| *generation != to)
    {
        return Err(ClosureAdmissionError::GenerationMismatch {
            expected: to,
            actual,
        });
    }
    Ok(())
}

fn validate_unique_identities(
    identities: &[CandidateIdentities],
) -> Result<(), ClosureAdmissionError> {
    let mut facts = BTreeSet::new();
    let mut derivations = BTreeSet::new();
    for identity in identities {
        if !facts.insert(identity.fact()) {
            return Err(ClosureAdmissionError::DuplicateFactId(identity.fact()));
        }
        if !derivations.insert(identity.derivation()) {
            return Err(ClosureAdmissionError::DuplicateDerivationId(
                identity.derivation(),
            ));
        }
    }
    Ok(())
}

fn build_derivations(
    candidates: &[DerivationCandidate],
    generation: GenerationId,
    identities: &[CandidateIdentities],
) -> Result<Vec<Derivation>, ClosureAdmissionError> {
    candidates
        .iter()
        .zip(identities)
        .map(|(candidate, identity)| {
            let output = Fact::new(
                identity.fact(),
                candidate.relation(),
                candidate.values().to_vec(),
                RelationContext::new(
                    generation,
                    RelationAuthority::Rule(candidate.rule()),
                    FactProvenance::Derivation(identity.derivation()),
                    EvidenceCompleteness::Complete,
                    FactValidity::Valid,
                ),
            );
            Derivation::new(
                identity.derivation(),
                candidate.rule(),
                generation,
                output,
                candidate.support().to_vec(),
            )
            .map_err(ClosureAdmissionError::Lineage)
        })
        .collect()
}
