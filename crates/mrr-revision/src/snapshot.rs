// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Admission for one immutable semantic generation and its source revisions.

use std::collections::BTreeSet;

use mrr_identity::{GenerationId, RevisionId};
use sha2::{Digest, Sha256};

use crate::RevisionBinding;

const SEMANTIC_SNAPSHOT_SCHEMA: &[u8] = b"mrr.semantic-snapshot.v1";

/// An admitted immutable binding from one semantic generation to source revisions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticSnapshot {
    generation: GenerationId,
    revisions: Vec<RevisionBinding>,
    digest: [u8; 32],
}

/// Reasons a set of revision bindings cannot define one semantic snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticSnapshotError {
    /// A semantic generation must be anchored to at least one source revision.
    Empty,
    /// Every source revision in the snapshot must name the snapshot generation.
    GenerationMismatch {
        revision: RevisionId,
        expected: GenerationId,
        actual: GenerationId,
    },
    /// A source revision may occur only once in a snapshot.
    DuplicateRevision(RevisionId),
    /// One logical source cannot resolve to multiple content revisions in one snapshot.
    ConflictingLogicalChange {
        provider: String,
        logical_change: String,
    },
}

impl SemanticSnapshot {
    /// Validates and canonically orders all revision bindings for one generation.
    pub fn admit(
        generation: GenerationId,
        mut revisions: Vec<RevisionBinding>,
    ) -> Result<Self, SemanticSnapshotError> {
        if revisions.is_empty() {
            return Err(SemanticSnapshotError::Empty);
        }
        revisions.sort_by_key(RevisionBinding::revision);

        let mut revision_ids = BTreeSet::new();
        let mut logical_sources = BTreeSet::new();
        for binding in &revisions {
            if binding.generation() != generation {
                return Err(SemanticSnapshotError::GenerationMismatch {
                    revision: binding.revision(),
                    expected: generation,
                    actual: binding.generation(),
                });
            }
            if !revision_ids.insert(binding.revision()) {
                return Err(SemanticSnapshotError::DuplicateRevision(binding.revision()));
            }
            let external = binding.external();
            let logical = (external.provider(), external.logical_change());
            if !logical_sources.insert(logical) {
                return Err(SemanticSnapshotError::ConflictingLogicalChange {
                    provider: external.provider().to_owned(),
                    logical_change: external.logical_change().to_owned(),
                });
            }
        }

        let digest = snapshot_digest(generation, &revisions);
        Ok(Self {
            generation,
            revisions,
            digest,
        })
    }

    /// Returns the exact semantic generation named by this snapshot.
    #[must_use]
    pub const fn generation(&self) -> GenerationId {
        self.generation
    }

    /// Returns source revisions in canonical `RevisionId` order.
    #[must_use]
    pub fn revisions(&self) -> &[RevisionBinding] {
        &self.revisions
    }

    /// Returns the V1 semantic snapshot digest.
    #[must_use]
    pub const fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
}

fn snapshot_digest(generation: GenerationId, revisions: &[RevisionBinding]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, SEMANTIC_SNAPSHOT_SCHEMA);
    hash_field(&mut hasher, generation.digest_bytes());
    hash_len(&mut hasher, revisions.len());
    for binding in revisions {
        hash_field(&mut hasher, binding.revision().digest_bytes());
    }
    hasher.finalize().into()
}

fn hash_len(hasher: &mut Sha256, len: usize) {
    hasher.update((len as u64).to_be_bytes());
}

fn hash_field(hasher: &mut Sha256, bytes: &[u8]) {
    hash_len(hasher, bytes.len());
    hasher.update(bytes);
}
