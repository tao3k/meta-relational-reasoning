//! Immutable witness-transport contracts and bounded, snapshot-bound admission.
//!
//! Verifiers are trusted integration points. A wire claim or digest alone is
//! never proof. This module checks bindings and structure; it does not execute
//! Lean, authenticate source owners, or authorize runtime effects.
use std::{fmt, io, num::NonZeroUsize};

use mrr_identity::{RevisionId, TransformationId};
use mrr_relation::ValueSchema;
use mrr_revision::SemanticSnapshot;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A semantic type includes its full input/result schemas and specification.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransformationEndpoint {
    pub semantics: [u8; 32],
    pub input: ValueSchema,
    pub result: ValueSchema,
}

/// The only admitted V1 profile: total deterministic, one-way witness transport.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TransformationProfile {
    TotalWitnessTransport,
}

/// Untrusted immutable description. Generation is deliberately excluded.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransformationDefinition {
    pub version: u16,
    pub profile: TransformationProfile,
    pub source: TransformationEndpoint,
    pub target: TransformationEndpoint,
    pub forward_artifact: [u8; 32],
    pub extract_artifact: [u8; 32],
    pub codec: [u8; 32],
    pub parameter_contract: [u8; 32],
    pub statement: [u8; 32],
    /// Semantic dependencies; sorted for identity and duplicates rejected.
    pub dependencies: Vec<[u8; 32]>,
    /// Bound premises, including domain, context and resource conditions.
    pub requirements: Vec<[u8; 32]>,
}

/// Explicit checker claims. They remain untrusted until verifier acceptance.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransformationEvidence {
    pub transformation: TransformationId,
    pub statement: [u8; 32],
    pub forward_artifact: [u8; 32],
    pub extract_artifact: [u8; 32],
    pub codec: [u8; 32],
    pub checker: [u8; 32],
    pub toolchain: [u8; 32],
    pub environment: [u8; 32],
    pub assumptions: [u8; 32],
    pub source_revisions: Vec<RevisionId>,
}

/// Exact source selection and request coordinates, constructed from a snapshot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TransformationBinding {
    pub(super) snapshot: [u8; 32],
    pub(super) generation: mrr_identity::GenerationId,
    revisions: Vec<RevisionId>,
    pub request: [u8; 32],
    pub context: [u8; 32],
    pub scope: [u8; 32],
    pub valid_time: i64,
    pub knowledge_time: i64,
}

impl TransformationBinding {
    pub fn new(
        snapshot: &SemanticSnapshot,
        request: [u8; 32],
        context: [u8; 32],
        scope: [u8; 32],
        valid_time: i64,
        knowledge_time: i64,
    ) -> Result<Self, TransformationError> {
        nonzero(&[request, context, scope])?;
        Ok(Self {
            snapshot: *snapshot.digest(),
            generation: snapshot.generation(),
            revisions: snapshot.revisions().iter().map(|r| r.revision()).collect(),
            request,
            context,
            scope,
            valid_time,
            knowledge_time,
        })
    }
    #[must_use]
    pub fn snapshot_digest(&self) -> &[u8; 32] {
        &self.snapshot
    }
    #[must_use]
    pub fn revisions(&self) -> &[RevisionId] {
        &self.revisions
    }
}

/// Work/storage bounds are nonzero and apply before expensive verification.
#[derive(Clone, Copy, Debug)]
pub struct TransformationLimits {
    pub max_bytes: NonZeroUsize,
    pub max_schema_nodes: NonZeroUsize,
    pub max_schema_depth: NonZeroUsize,
    pub max_dependencies: NonZeroUsize,
    pub max_steps: NonZeroUsize,
}

/// Distinct failure states never become a negative solvability claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransformationError {
    Version,
    InvalidDigest,
    InvalidSchema,
    DuplicateDependency,
    Budget,
    EvidenceMismatch,
    SourceMismatch,
    BindingMismatch,
    EndpointMismatch,
    InstanceMismatch,
    EmptyPlan,
    Unknown,
    Conflict,
    Revoked,
    Rejected,
    Encoding,
    PublicationUncertain,
    KernelRejected { diagnostics: String },
    KernelUnavailable { diagnostics: String },
}
impl fmt::Display for TransformationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for TransformationError {}

/// A trusted verifier must return a receipt for the exact supplied request.
/// Implementations must check statement/axioms, artifact refinement, codec,
/// toolchain, dependencies, current source authority and revocation. A checker
/// that merely compares hashes is insufficient for semantic certification.
pub trait TransformationVerifier {
    /// Content identity of the verifier implementation and admission policy.
    fn policy(&self) -> [u8; 32];
    fn check_definition(
        &self,
        definition: &TransformationDefinition,
        evidence: &TransformationEvidence,
        binding: &TransformationBinding,
    ) -> Result<[u8; 32], TransformationError>;
    /// Check concrete forward-instance transport, parameters and all premises
    /// at this ordered step, including current revocation and source authority.
    /// The verifier must remain under one immutable policy during admission. `prior` binds earlier steps and their receipts.
    fn check_step(
        &self,
        step: &TransformationStep,
        binding: &TransformationBinding,
        index: usize,
        prior: &[u8; 32],
    ) -> Result<[u8; 32], TransformationError>;
    /// Check terminal solver correctness/domain and authorization for this cut.
    fn check_solver(
        &self,
        target: &TransformationEndpoint,
        solver: &[u8; 32],
        input: &[u8; 32],
        binding: &TransformationBinding,
        prior: &[u8; 32],
    ) -> Result<[u8; 32], TransformationError>;
}

/// Opaque receipt; neither deserializable nor directly constructible.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransformationAdmission {
    id: TransformationId,
    pub(super) definition: TransformationDefinition,
    evidence: TransformationEvidence,
    pub(super) binding: TransformationBinding,
    receipt: [u8; 32],
    policy: [u8; 32],
    pub(super) digest: [u8; 32],
}
impl TransformationAdmission {
    #[must_use]
    pub fn id(&self) -> TransformationId {
        self.id
    }
    #[must_use]
    pub fn definition(&self) -> &TransformationDefinition {
        &self.definition
    }
    #[must_use]
    pub fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
    #[must_use]
    pub fn evidence(&self) -> &TransformationEvidence {
        &self.evidence
    }
    #[must_use]
    pub fn binding(&self) -> &TransformationBinding {
        &self.binding
    }
    #[must_use]
    pub fn check_receipt(&self) -> &[u8; 32] {
        &self.receipt
    }
}

/// A concrete ordered edge instance; a reference alone cannot establish it.
#[derive(Clone, Debug)]
pub struct TransformationStep {
    pub admission: TransformationAdmission,
    pub input: [u8; 32],
    pub target_input: [u8; 32],
    pub parameters: [u8; 32],
}

/// Untrusted route, independent of the search engine that found it.
#[derive(Clone, Debug)]
pub struct TransformationPlanCandidate {
    pub binding: TransformationBinding,
    pub source: TransformationEndpoint,
    pub target: TransformationEndpoint,
    pub input: [u8; 32],
    pub steps: Vec<TransformationStep>,
    pub solver: [u8; 32],
}

/// Completed structural/obligation admission, not a runtime execution receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransformationPlanAdmission {
    pub(super) binding: TransformationBinding,
    edges: Vec<TransformationId>,
    step_receipts: Vec<[u8; 32]>,
    solver_receipt: [u8; 32],
    policy: [u8; 32],
    pub(super) digest: [u8; 32],
}
impl TransformationPlanAdmission {
    #[must_use]
    pub fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
    #[must_use]
    pub fn edges(&self) -> &[TransformationId] {
        &self.edges
    }
    #[must_use]
    pub fn binding(&self) -> &TransformationBinding {
        &self.binding
    }
    #[must_use]
    pub fn step_receipts(&self) -> &[[u8; 32]] {
        &self.step_receipts
    }
    #[must_use]
    pub fn solver_receipt(&self) -> &[u8; 32] {
        &self.solver_receipt
    }
}

/// Derive stable identity after complete validation and canonical set ordering.
pub fn transformation_identity(
    definition: &TransformationDefinition,
    limits: TransformationLimits,
) -> Result<TransformationId, TransformationError> {
    TransformationId::from_canonical_bytes(encode_transformation_definition(definition, limits)?)
        .map_err(|_| TransformationError::Encoding)
}

/// V1 CBOR wire frame: namespace, then the normalized declaration in field
/// declaration order. No maps with caller-controlled key ordering are accepted.
pub fn encode_transformation_definition(
    definition: &TransformationDefinition,
    limits: TransformationLimits,
) -> Result<Vec<u8>, TransformationError> {
    encode(
        &("mrr.transformation.v1", normalize(definition, limits)?),
        limits.max_bytes.get(),
    )
}

/// Bounded decoding yields a validated candidate, never an admitted receipt.
/// Unknown/duplicate fields (including nested schemas), trailing data,
/// noncanonical CBOR, unsupported frames and invalid schemas are rejected. The CBOR parser also has its own recursion limit.
pub fn decode_transformation_definition(
    bytes: &[u8],
    limits: TransformationLimits,
) -> Result<TransformationDefinition, TransformationError> {
    if bytes.len() > limits.max_bytes.get() {
        return Err(TransformationError::Budget);
    }
    let mut cursor = io::Cursor::new(bytes);
    let (namespace, declaration): (String, TransformationDefinition) =
        ciborium::from_reader(&mut cursor).map_err(|_| TransformationError::Encoding)?;
    if namespace != "mrr.transformation.v1" {
        return Err(TransformationError::Version);
    }
    if cursor.position() != bytes.len() as u64 {
        return Err(TransformationError::Encoding);
    }
    let normalized = normalize(&declaration, limits)?;
    if encode_transformation_definition(&normalized, limits)? != bytes {
        return Err(TransformationError::Encoding);
    }
    Ok(normalized)
}

/// Admit exact evidence under one trusted checker policy and source selection.
pub fn admit_transformation(
    definition: &TransformationDefinition,
    evidence: &TransformationEvidence,
    binding: &TransformationBinding,
    limits: TransformationLimits,
    verifier: &impl TransformationVerifier,
) -> Result<TransformationAdmission, TransformationError> {
    let policy = verifier.policy();
    nonzero(&[binding.request, binding.context, binding.scope, policy])?;
    let definition = normalize(definition, limits)?;
    let id = transformation_identity(&definition, limits)?;
    if evidence.transformation != id
        || evidence.statement != definition.statement
        || evidence.forward_artifact != definition.forward_artifact
        || evidence.extract_artifact != definition.extract_artifact
        || evidence.codec != definition.codec
    {
        return Err(TransformationError::EvidenceMismatch);
    }
    nonzero(&[
        evidence.checker,
        evidence.toolchain,
        evidence.environment,
        evidence.assumptions,
    ])?;
    if evidence.source_revisions.len() > limits.max_dependencies.get() {
        return Err(TransformationError::Budget);
    }
    let mut revisions = evidence.source_revisions.clone();
    revisions.sort();
    if revisions.windows(2).any(|r| r[0] == r[1]) {
        return Err(TransformationError::DuplicateDependency);
    }
    if revisions != binding.revisions {
        return Err(TransformationError::SourceMismatch);
    }
    let mut canonical_evidence = evidence.clone();
    canonical_evidence.source_revisions = revisions;
    let evidence = &canonical_evidence;
    let evidence_bytes = encode(&(evidence, binding), limits.max_bytes.get())?;
    let receipt = verifier.check_definition(&definition, evidence, binding)?;
    nonzero(&[receipt])?;
    if verifier.policy() != policy {
        return Err(TransformationError::BindingMismatch);
    }
    let digest = digest(
        &(
            "mrr.transformation-admission.v1",
            id,
            evidence_bytes,
            receipt,
            policy,
        ),
        limits,
    )?;
    Ok(TransformationAdmission {
        id,
        definition,
        evidence: canonical_evidence,
        binding: binding.clone(),
        receipt,
        policy,
        digest,
    })
}

/// Validate each ordered witness and publish only after the terminal check.
/// Repeated edges/cycles are allowed within the explicit step bound; they
/// provide no exhaustive-search or complexity guarantee. Empty routes are
/// currently rejected until an explicit identity artifact is supplied.
pub fn admit_transformation_plan(
    candidate: &TransformationPlanCandidate,
    limits: TransformationLimits,
    verifier: &impl TransformationVerifier,
) -> Result<TransformationPlanAdmission, TransformationError> {
    let policy = verifier.policy();
    nonzero(&[
        candidate.binding.request,
        candidate.binding.context,
        candidate.binding.scope,
        policy,
    ])?;
    if candidate.steps.is_empty() {
        return Err(TransformationError::EmptyPlan);
    }
    if candidate.steps.len() > limits.max_steps.get() {
        return Err(TransformationError::Budget);
    }
    endpoint(&candidate.source, limits)?;
    endpoint(&candidate.target, limits)?;
    nonzero(&[candidate.input, candidate.solver])?;
    let mut current = &candidate.source;
    let mut input = candidate.input;
    let mut edges = Vec::new();
    let mut receipts = Vec::new();
    let mut prior = digest(
        &(
            "mrr.transformation-plan.v1",
            &candidate.binding,
            &candidate.source,
            &candidate.target,
            candidate.input,
            candidate.solver,
        ),
        limits,
    )?;
    for (index, step) in candidate.steps.iter().enumerate() {
        if step.admission.binding != candidate.binding || step.admission.policy != policy {
            return Err(TransformationError::BindingMismatch);
        }
        if &step.admission.definition.source != current {
            return Err(TransformationError::EndpointMismatch);
        }
        if step.input != input {
            return Err(TransformationError::InstanceMismatch);
        }
        nonzero(&[step.input, step.target_input, step.parameters])?;
        let receipt = verifier.check_step(step, &candidate.binding, index, &prior)?;
        nonzero(&[receipt])?;
        prior = digest(
            &(
                "mrr.transformation-plan-step.v1",
                prior,
                index as u64,
                step.admission.digest,
                step.admission.receipt,
                step.input,
                step.target_input,
                step.parameters,
                receipt,
            ),
            limits,
        )?;
        edges.push(step.admission.id);
        receipts.push(receipt);
        current = &step.admission.definition.target;
        input = step.target_input;
    }
    if current != &candidate.target {
        return Err(TransformationError::EndpointMismatch);
    }
    let solver_receipt = verifier.check_solver(
        current,
        &candidate.solver,
        &input,
        &candidate.binding,
        &prior,
    )?;
    nonzero(&[solver_receipt])?;
    if verifier.policy() != policy {
        return Err(TransformationError::BindingMismatch);
    }
    let digest = digest(
        &(
            "mrr.transformation-plan-completed.v1",
            prior,
            solver_receipt,
        ),
        limits,
    )?;
    Ok(TransformationPlanAdmission {
        binding: candidate.binding.clone(),
        edges,
        step_receipts: receipts,
        solver_receipt,
        policy,
        digest,
    })
}

fn normalize(
    definition: &TransformationDefinition,
    limits: TransformationLimits,
) -> Result<TransformationDefinition, TransformationError> {
    if definition.version != 1 {
        return Err(TransformationError::Version);
    }
    endpoint(&definition.source, limits)?;
    endpoint(&definition.target, limits)?;
    nonzero(&[
        definition.forward_artifact,
        definition.extract_artifact,
        definition.codec,
        definition.parameter_contract,
        definition.statement,
    ])?;
    if definition
        .dependencies
        .len()
        .saturating_add(definition.requirements.len())
        > limits.max_dependencies.get()
    {
        return Err(TransformationError::Budget);
    }
    // Bound canonical bytes before cloning untrusted strings or vectors.
    encode(definition, limits.max_bytes.get())?;
    let mut result = definition.clone();
    for set in [&mut result.dependencies, &mut result.requirements] {
        nonzero(set)?;
        set.sort();
        if set.windows(2).any(|v| v[0] == v[1]) {
            return Err(TransformationError::DuplicateDependency);
        }
    }
    Ok(result)
}
fn endpoint(
    value: &TransformationEndpoint,
    limits: TransformationLimits,
) -> Result<(), TransformationError> {
    nonzero(&[value.semantics])?;
    let mut work = vec![(&value.input, 1_usize), (&value.result, 1)];
    let mut nodes = 0_usize;
    while let Some((schema, depth)) = work.pop() {
        nodes += 1;
        if nodes > limits.max_schema_nodes.get() || depth > limits.max_schema_depth.get() {
            return Err(TransformationError::Budget);
        }
        match schema {
            ValueSchema::List { element, .. } => work.push((element, depth + 1)),
            ValueSchema::Record { fields } => {
                if fields.len()
                    > limits
                        .max_schema_nodes
                        .get()
                        .saturating_sub(nodes.saturating_add(work.len()))
                {
                    return Err(TransformationError::Budget);
                }
                work.extend(fields.iter().map(|f| (f.schema(), depth + 1)));
            }
            _ => {}
        }
    }
    value
        .input
        .validate()
        .map_err(|_| TransformationError::InvalidSchema)?;
    value
        .result
        .validate()
        .map_err(|_| TransformationError::InvalidSchema)?;
    Ok(())
}
fn nonzero(values: &[[u8; 32]]) -> Result<(), TransformationError> {
    if values.contains(&[0; 32]) {
        return Err(TransformationError::InvalidDigest);
    }
    Ok(())
}
struct BoundedBytes {
    bytes: Vec<u8>,
    limit: usize,
}
impl io::Write for BoundedBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other("transformation byte budget"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn encode(value: &impl Serialize, limit: usize) -> Result<Vec<u8>, TransformationError> {
    let mut writer = BoundedBytes {
        bytes: Vec::new(),
        limit,
    };
    ciborium::into_writer(value, &mut writer).map_err(|_| TransformationError::Budget)?;
    Ok(writer.bytes)
}
pub(super) fn digest(
    value: &impl Serialize,
    limits: TransformationLimits,
) -> Result<[u8; 32], TransformationError> {
    Ok(Sha256::digest(encode(value, limits.max_bytes.get())?).into())
}

/// Project failed admission without asserting global unsolvability. A rejected
/// route/answer cannot prove FALSE; closed-world WHY-NOT needs separate coverage.
#[must_use]
pub const fn transformation_failure_truth(error: &TransformationError) -> crate::TruthStatus {
    match error {
        TransformationError::Budget => crate::TruthStatus::Incomplete,
        TransformationError::Conflict => crate::TruthStatus::Conflict,
        TransformationError::Revoked => crate::TruthStatus::Stale,
        _ => crate::TruthStatus::Unknown,
    }
}
