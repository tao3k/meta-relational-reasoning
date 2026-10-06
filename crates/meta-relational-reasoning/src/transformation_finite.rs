//! Exhaustively checked native transport over explicit, bounded finite problems.
//! The configured source owner supplies the specification relation. Every native
//! forward/extract table entry is checked against that relation before admission.
use std::sync::{Arc, RwLock};

use mrr_relation::{Value, ValueSchema};
use serde::{Deserialize, Serialize};

use super::transformation::{
    TransformationBinding, TransformationDefinition, TransformationEndpoint, TransformationError,
    TransformationEvidence, TransformationLimits, TransformationProfile, TransformationStep,
    TransformationVerifier, admit_transformation, digest, transformation_identity,
};
use super::transformation_execution::{TransformationRuntime, transformation_value_digest};

/// Explicit finite specification: each row lists all correct answer indices.
/// Empty rows mean the endpoint has no total solver and are rejected.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FiniteProblem {
    /// Content identity of the owner-selected domain/corpus interpretation.
    pub domain: [u8; 32],
    pub answers: usize,
    pub correct: Vec<Vec<usize>>,
}
impl FiniteProblem {
    pub(super) fn validate(&self, bound: usize) -> Result<(), TransformationError> {
        if self.domain == [0; 32] {
            return Err(TransformationError::InvalidDigest);
        }
        if self.answers == 0
            || self.correct.is_empty()
            || self.answers > bound
            || self.correct.len() > bound
        {
            return Err(TransformationError::Budget);
        }
        let mut cells = 0_usize;
        for row in &self.correct {
            cells = cells
                .checked_add(row.len())
                .ok_or(TransformationError::Budget)?;
            if cells > bound || row.is_empty() {
                return Err(TransformationError::Budget);
            }
            if row.iter().any(|a| *a >= self.answers)
                || row.windows(2).any(|pair| pair[0] >= pair[1])
            {
                return Err(TransformationError::Rejected);
            }
        }
        Ok(())
    }
    fn endpoint(
        &self,
        limits: TransformationLimits,
    ) -> Result<TransformationEndpoint, TransformationError> {
        Ok(TransformationEndpoint {
            semantics: digest(&("mrr.finite-problem.v1", self), limits)?,
            input: ValueSchema::Integer,
            result: ValueSchema::Integer,
        })
    }
}

/// Native implementation is the exact finite table, with no foreign code hook.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FiniteTransport {
    /// Ordered stable leaf identities for an explicitly composed table artifact.
    pub lineage: Vec<[u8; 32]>,
    pub source: FiniteProblem,
    pub target: FiniteProblem,
    pub forward: Vec<usize>,
    /// One row per original input; one entry per target answer.
    pub extract: Vec<Vec<usize>>,
}
impl FiniteTransport {
    fn validate(&self, bound: usize) -> Result<(), TransformationError> {
        self.source.validate(bound)?;
        self.target.validate(bound)?;
        if self.forward.len() != self.source.correct.len()
            || self.extract.len() != self.source.correct.len()
        {
            return Err(TransformationError::Rejected);
        }
        let cells = self
            .extract
            .len()
            .checked_mul(self.target.answers)
            .ok_or(TransformationError::Budget)?;
        if cells > bound {
            return Err(TransformationError::Budget);
        }
        for (input, (&target, extract)) in self.forward.iter().zip(&self.extract).enumerate() {
            if target >= self.target.correct.len()
                || extract.len() != self.target.answers
                || extract.iter().any(|answer| *answer >= self.source.answers)
            {
                return Err(TransformationError::Rejected);
            }
            // Exhaustive soundness, including every correct target witness.
            for answer in &self.target.correct[target] {
                if self.source.correct[input]
                    .binary_search(&extract[*answer])
                    .is_err()
                {
                    return Err(TransformationError::Rejected);
                }
            }
        }
        Ok(())
    }
    fn definition(
        &self,
        limits: TransformationLimits,
    ) -> Result<TransformationDefinition, TransformationError> {
        let mut dependencies = self.lineage.clone();
        dependencies.sort_unstable();
        dependencies.dedup();
        Ok(TransformationDefinition {
            version: 1,
            profile: TransformationProfile::TotalWitnessTransport,
            source: self.source.endpoint(limits)?,
            target: self.target.endpoint(limits)?,
            forward_artifact: digest(&("mrr.finite-forward.v1", &self.forward), limits)?,
            extract_artifact: digest(&("mrr.finite-extract.v1", &self.extract), limits)?,
            codec: digest(&"mrr.finite-index.i64.v1", limits)?,
            parameter_contract: digest(&"mrr.finite-no-parameters.v1", limits)?,
            statement: digest(&("mrr.finite-total-witness.v1", self), limits)?,
            dependencies,
            requirements: vec![],
        })
    }
}

/// Immutable native catalog plus an owner-controlled source selection lock.
/// A write revokes every receipt from the old selection; publication holds a
/// read lock until its atomic replacement, excluding concurrent invalidation.
#[derive(Clone, Debug)]
pub struct FiniteTransformationCatalog {
    transports: Vec<FiniteTransport>,
    definitions: Vec<TransformationDefinition>,
    selected: Arc<RwLock<Option<TransformationBinding>>>,
    binding: TransformationBinding,
    limits: TransformationLimits,
    policy: [u8; 32],
}
impl FiniteTransformationCatalog {
    pub fn new(
        transports: Vec<FiniteTransport>,
        binding: TransformationBinding,
        limits: TransformationLimits,
    ) -> Result<Self, TransformationError> {
        if transports.is_empty() || transports.len() > limits.max_dependencies.get() {
            return Err(TransformationError::Budget);
        }
        let bound = limits.max_bytes.get();
        let mut definitions = Vec::new();
        let mut work = 0_usize;
        for transport in &transports {
            if transport.lineage.len() > limits.max_steps.get()
                || transport.lineage.contains(&[0; 32])
            {
                return Err(TransformationError::Budget);
            }
            work = work
                .checked_add(
                    transport
                        .extract
                        .len()
                        .checked_mul(transport.target.answers)
                        .ok_or(TransformationError::Budget)?,
                )
                .ok_or(TransformationError::Budget)?;
            if work > bound {
                return Err(TransformationError::Budget);
            }
            transport.validate(bound)?;
            definitions.push(transport.definition(limits)?);
        }
        let policy = digest(
            &(
                "mrr.finite-native-checker.v1",
                include_str!("transformation_finite.rs"),
            ),
            TransformationLimits {
                max_bytes: std::num::NonZeroUsize::new(1_048_576)
                    .ok_or(TransformationError::Budget)?,
                ..limits
            },
        )?;
        let catalog = Self {
            transports,
            definitions,
            selected: Arc::new(RwLock::new(Some(binding.clone()))),
            binding,
            limits,
            policy,
        };
        catalog.qualify_native()?;
        Ok(catalog)
    }
    // Exhaustively compare the actual runtime entry points against the model
    // tables, not merely a second reader of their hashes. The complete finite
    // domain, every target answer, and every solver input are enumerated.
    fn qualify_native(&self) -> Result<(), TransformationError> {
        for (ordinal, transport) in self.transports.iter().enumerate() {
            let definition = &self.definitions[ordinal];
            let admission = admit_transformation(
                definition,
                &self.evidence(ordinal)?,
                &self.binding,
                self.limits,
                self,
            )?;
            for input in 0..transport.source.correct.len() {
                let source = value(input)?;
                let target = value(transport.forward[input])?;
                let step = TransformationStep {
                    admission: admission.clone(),
                    input: transformation_value_digest(
                        &ValueSchema::Integer,
                        &source,
                        self.limits,
                    )?,
                    target_input: transformation_value_digest(
                        &ValueSchema::Integer,
                        &target,
                        self.limits,
                    )?,
                    parameters: self.parameters()?,
                };
                if self.forward(&step, &source, &self.binding)? != target {
                    return Err(TransformationError::Rejected);
                }
                for answer in 0..transport.target.answers {
                    let witness = value(answer)?;
                    let extracted = self.extract(&step, &source, &witness, &self.binding)?;
                    if extracted != value(transport.extract[input][answer])? {
                        return Err(TransformationError::Rejected);
                    }
                    let target_correct = transport.target.correct[transport.forward[input]]
                        .binary_search(&answer)
                        .is_ok();
                    let source_correct = transport.source.correct[input]
                        .binary_search(&index(&extracted)?)
                        .is_ok();
                    if self
                        .check_answer(&definition.target, &target, &witness, &self.binding)
                        .is_ok()
                        != target_correct
                        || self
                            .check_answer(&definition.source, &source, &extracted, &self.binding)
                            .is_ok()
                            != source_correct
                        || (target_correct && !source_correct)
                    {
                        return Err(TransformationError::Rejected);
                    }
                }
            }
            for input in 0..transport.target.correct.len() {
                let source = value(input)?;
                let answer = self.solve(
                    &self.solver(&definition.target)?,
                    &definition.target,
                    &source,
                    &self.binding,
                )?;
                self.check_answer(&definition.target, &source, &answer, &self.binding)?;
            }
        }
        Ok(())
    }
    /// Materialize and freshly qualify a composite table; its ordered stable
    /// lineage participates in content identity. It shares source revocation
    /// with the original catalog and never admits a graph route by itself.
    pub fn compose(&self, first: usize, second: usize) -> Result<Self, TransformationError> {
        self.with_current(&self.binding, || {
            let f = self
                .transports
                .get(first)
                .ok_or(TransformationError::Unknown)?;
            let g = self
                .transports
                .get(second)
                .ok_or(TransformationError::Unknown)?;
            if f.target != g.source {
                return Err(TransformationError::EndpointMismatch);
            }
            let cells = f
                .source
                .correct
                .len()
                .checked_mul(g.target.answers)
                .ok_or(TransformationError::Budget)?;
            if cells > self.limits.max_bytes.get() {
                return Err(TransformationError::Budget);
            }
            let mut lineage = Vec::new();
            for ordinal in [first, second] {
                let transport = &self.transports[ordinal];
                if transport.lineage.is_empty() {
                    lineage.push(
                        *transformation_identity(&self.definitions[ordinal], self.limits)?
                            .digest_bytes(),
                    );
                } else {
                    lineage.extend(&transport.lineage);
                }
            }
            if lineage.len() > self.limits.max_steps.get() {
                return Err(TransformationError::Budget);
            }
            let forward = f.forward.iter().map(|input| g.forward[*input]).collect();
            let extract = f
                .forward
                .iter()
                .enumerate()
                .map(|(input, middle)| {
                    g.extract[*middle]
                        .iter()
                        .map(|answer| f.extract[input][*answer])
                        .collect()
                })
                .collect();
            let mut composite = Self::new(
                vec![FiniteTransport {
                    source: f.source.clone(),
                    target: g.target.clone(),
                    forward,
                    extract,
                    lineage,
                }],
                self.binding.clone(),
                self.limits,
            )?;
            composite.selected = Arc::clone(&self.selected);
            Ok(composite)
        })
    }
    pub(super) fn share_authority(&mut self, parent: &Self) -> Result<(), TransformationError> {
        parent.with_current(&self.binding, || {
            self.selected = Arc::clone(&parent.selected);
            Ok(())
        })
    }
    /// Decode one exact canonical catalog under byte, cell, and total-work bounds.
    pub fn from_bytes(
        bytes: &[u8],
        binding: TransformationBinding,
        limits: TransformationLimits,
    ) -> Result<Self, TransformationError> {
        if bytes.len() > limits.max_bytes.get() {
            return Err(TransformationError::Budget);
        }
        let mut reader = bytes;
        let transports: Vec<FiniteTransport> =
            ciborium::de::from_reader(&mut reader).map_err(|_| TransformationError::Encoding)?;
        if !reader.is_empty() {
            return Err(TransformationError::Encoding);
        }
        let catalog = Self::new(transports, binding, limits)?;
        if catalog.to_bytes()? != bytes {
            return Err(TransformationError::Encoding);
        }
        Ok(catalog)
    }
    /// Export implementation tables; serialized bytes never constitute admission.
    pub fn to_bytes(&self) -> Result<Vec<u8>, TransformationError> {
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(&self.transports, &mut bytes)
            .map_err(|_| TransformationError::Encoding)?;
        if bytes.len() > self.limits.max_bytes.get() {
            return Err(TransformationError::Budget);
        }
        Ok(bytes)
    }
    /// Exact number of extractor cells admitted by the exhaustive native check.
    #[must_use]
    pub fn extraction_cells(&self) -> usize {
        self.transports
            .iter()
            .map(|t| t.extract.len() * t.target.answers)
            .sum()
    }
    pub(super) fn tables(&self) -> &[FiniteTransport] {
        &self.transports
    }
    #[must_use]
    pub fn selected_binding(&self) -> &TransformationBinding {
        &self.binding
    }
    #[must_use]
    pub fn definitions(&self) -> &[TransformationDefinition] {
        &self.definitions
    }
    pub fn evidence(&self, index: usize) -> Result<TransformationEvidence, TransformationError> {
        let definition = self
            .definitions
            .get(index)
            .ok_or(TransformationError::Unknown)?;
        Ok(TransformationEvidence {
            transformation: transformation_identity(definition, self.limits)?,
            statement: definition.statement,
            forward_artifact: definition.forward_artifact,
            extract_artifact: definition.extract_artifact,
            codec: definition.codec,
            checker: self.policy,
            toolchain: digest(&"rust-finite-exhaustive-v1", self.limits)?,
            environment: self.policy,
            assumptions: digest(&"owner-selected-finite-spec.v1", self.limits)?,
            source_revisions: self.binding.revisions().to_vec(),
        })
    }
    #[must_use]
    pub fn is_current(&self) -> bool {
        self.with_current(&self.binding, || Ok(())).is_ok()
    }
    pub fn revoke(&self) -> Result<(), TransformationError> {
        *self
            .selected
            .write()
            .map_err(|_| TransformationError::Conflict)? = None;
        Ok(())
    }
    /// Hold exclusive authority while the owner durably records revocation.
    /// A persistence failure leaves this live authority revoked.
    pub(super) fn revoke_with<T>(
        &self,
        action: impl FnOnce() -> Result<T, TransformationError>,
    ) -> Result<T, TransformationError> {
        let mut selected = self
            .selected
            .write()
            .map_err(|_| TransformationError::Conflict)?;
        *selected = None;
        action()
    }
    /// Execute a publication transaction while the owner selection cannot change.
    pub fn with_current<T>(
        &self,
        binding: &TransformationBinding,
        action: impl FnOnce() -> Result<T, TransformationError>,
    ) -> Result<T, TransformationError> {
        let selection = self
            .selected
            .read()
            .map_err(|_| TransformationError::Conflict)?;
        if selection.as_ref() != Some(binding) || binding != &self.binding {
            return Err(TransformationError::Revoked);
        }
        action()
    }
    fn index(&self, definition: &TransformationDefinition) -> Result<usize, TransformationError> {
        self.definitions
            .iter()
            .position(|d| d == definition)
            .ok_or(TransformationError::Unknown)
    }
    fn problem(
        &self,
        endpoint: &TransformationEndpoint,
    ) -> Result<&FiniteProblem, TransformationError> {
        for transport in &self.transports {
            for problem in [&transport.source, &transport.target] {
                if &problem.endpoint(self.limits)? == endpoint {
                    return Ok(problem);
                }
            }
        }
        Err(TransformationError::Unknown)
    }
    pub fn solver(
        &self,
        endpoint: &TransformationEndpoint,
    ) -> Result<[u8; 32], TransformationError> {
        digest(
            &("mrr.finite-solver.v1", self.problem(endpoint)?),
            self.limits,
        )
    }
    pub fn parameters(&self) -> Result<[u8; 32], TransformationError> {
        digest(&"mrr.finite-no-parameters.v1", self.limits)
    }
    fn input(
        &self,
        problem: &FiniteProblem,
        input: &[u8; 32],
    ) -> Result<usize, TransformationError> {
        for index in 0..problem.correct.len() {
            if transformation_value_digest(&ValueSchema::Integer, &value(index)?, self.limits)?
                == *input
            {
                return Ok(index);
            }
        }
        Err(TransformationError::InstanceMismatch)
    }
}
fn index(value: &Value) -> Result<usize, TransformationError> {
    match value {
        Value::Integer(i) => usize::try_from(*i).map_err(|_| TransformationError::Rejected),
        _ => Err(TransformationError::InvalidSchema),
    }
}
fn value(index: usize) -> Result<Value, TransformationError> {
    i64::try_from(index)
        .map(Value::Integer)
        .map_err(|_| TransformationError::Budget)
}
impl TransformationVerifier for FiniteTransformationCatalog {
    fn policy(&self) -> [u8; 32] {
        self.policy
    }
    fn check_definition(
        &self,
        definition: &TransformationDefinition,
        evidence: &TransformationEvidence,
        binding: &TransformationBinding,
    ) -> Result<[u8; 32], TransformationError> {
        self.with_current(binding, || {
            let index = self.index(definition)?;
            if evidence != &self.evidence(index)? {
                return Err(TransformationError::EvidenceMismatch);
            }
            digest(&(self.policy, definition, evidence, binding), self.limits)
        })
    }
    fn check_step(
        &self,
        step: &TransformationStep,
        binding: &TransformationBinding,
        ordinal: usize,
        prior: &[u8; 32],
    ) -> Result<[u8; 32], TransformationError> {
        self.with_current(binding, || {
            let t = &self.transports[self.index(step.admission.definition())?];
            let input = self.input(&t.source, &step.input)?;
            let target = transformation_value_digest(
                &ValueSchema::Integer,
                &value(t.forward[input])?,
                self.limits,
            )?;
            if target != step.target_input || step.parameters != self.parameters()? {
                return Err(TransformationError::InstanceMismatch);
            }
            digest(
                &(
                    self.policy,
                    ordinal,
                    prior,
                    step.input,
                    target,
                    step.parameters,
                ),
                self.limits,
            )
        })
    }
    fn check_solver(
        &self,
        target: &TransformationEndpoint,
        solver: &[u8; 32],
        input: &[u8; 32],
        binding: &TransformationBinding,
        prior: &[u8; 32],
    ) -> Result<[u8; 32], TransformationError> {
        self.with_current(binding, || {
            if solver != &self.solver(target)? {
                return Err(TransformationError::Rejected);
            }
            let index = self.input(self.problem(target)?, input)?;
            digest(&(self.policy, solver, index, prior), self.limits)
        })
    }
}
impl TransformationRuntime for FiniteTransformationCatalog {
    fn identity(&self) -> [u8; 32] {
        self.policy
    }
    fn forward(
        &self,
        step: &TransformationStep,
        input: &Value,
        binding: &TransformationBinding,
    ) -> Result<Value, TransformationError> {
        self.with_current(binding, || {
            let t = &self.transports[self.index(step.admission.definition())?];
            value(
                *t.forward
                    .get(index(input)?)
                    .ok_or(TransformationError::Rejected)?,
            )
        })
    }
    fn solve(
        &self,
        solver: &[u8; 32],
        target: &TransformationEndpoint,
        input: &Value,
        binding: &TransformationBinding,
    ) -> Result<Value, TransformationError> {
        self.with_current(binding, || {
            if solver != &self.solver(target)? {
                return Err(TransformationError::Rejected);
            }
            let p = self.problem(target)?;
            value(
                *p.correct
                    .get(index(input)?)
                    .and_then(|row| row.first())
                    .ok_or(TransformationError::Rejected)?,
            )
        })
    }
    fn extract(
        &self,
        step: &TransformationStep,
        source_input: &Value,
        target_answer: &Value,
        binding: &TransformationBinding,
    ) -> Result<Value, TransformationError> {
        self.with_current(binding, || {
            let t = &self.transports[self.index(step.admission.definition())?];
            value(
                *t.extract
                    .get(index(source_input)?)
                    .and_then(|row| row.get(index(target_answer).ok()?))
                    .ok_or(TransformationError::Rejected)?,
            )
        })
    }
    fn check_answer(
        &self,
        endpoint: &TransformationEndpoint,
        input: &Value,
        answer: &Value,
        binding: &TransformationBinding,
    ) -> Result<[u8; 32], TransformationError> {
        self.with_current(binding, || {
            let p = self.problem(endpoint)?;
            let row = p
                .correct
                .get(index(input)?)
                .ok_or(TransformationError::Rejected)?;
            if row.binary_search(&index(answer)?).is_err() {
                return Err(TransformationError::Rejected);
            }
            digest(
                &("mrr.finite-spec-answer.v1", endpoint, input, answer),
                self.limits,
            )
        })
    }
}

/// Trusted finite owner used by durable publication; kernel wrappers expose the
/// same immutable catalog authority while retaining their own verifier policy.
pub trait FiniteTransformationOwner: TransformationVerifier + TransformationRuntime {
    fn authority(&self) -> &FiniteTransformationCatalog;
    fn authority_supports(&self) -> Vec<[u8; 32]> {
        Vec::new()
    }
    fn authority_lease(&self) -> super::transformation_authority::TransformationAuthorityLease {
        super::transformation_authority::TransformationAuthorityLease::native(self.authority())
    }
    fn publication_lease(
        &self,
    ) -> Result<super::transformation_authority::TransformationAuthorityLease, TransformationError>
    {
        Ok(self.authority_lease())
    }
}
impl FiniteTransformationOwner for FiniteTransformationCatalog {
    fn authority(&self) -> &FiniteTransformationCatalog {
        self
    }
}
impl FiniteTransformationOwner for super::transformation_kernel::KernelCheckedFiniteCatalog {
    fn authority(&self) -> &FiniteTransformationCatalog {
        self.native()
    }
}
