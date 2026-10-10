//! Built-in proof identity for the single source-extracted byte copy primitive.
//! Acceptance relies on the pinned source extraction, Lean and axiom CI gate.
use sha2::{Digest, Sha256};

/// Opaque evidence for byte sequence equality only. It says nothing about
/// endpoint semantics, codecs, provider guards, solver or external authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransformationByteIdentityProof {
    identity: [u8; 32],
    assumptions: [u8; 32],
}
impl TransformationByteIdentityProof {
    #[must_use]
    pub const fn identity(&self) -> &[u8; 32] {
        &self.identity
    }
    #[must_use]
    pub const fn assumptions(&self) -> &[u8; 32] {
        &self.assumptions
    }
}

/// Select the library's checked byte identity theorem. No arbitrary artifact
/// or caller-supplied certificate can be promoted by this constructor.
#[must_use]
pub fn transformation_byte_identity_proof() -> TransformationByteIdentityProof {
    let mut hash = Sha256::new();
    hash.update(b"mrr.byte-identity.source-proof.v1");
    for source in [
        include_bytes!("transformation_table.rs").as_slice(),
        include_bytes!(
            "../../../proofs/MRRProof/AgenticAIContextRust/TransformGenerated/Funs.lean"
        ),
        include_bytes!(
            "../../../proofs/MRRProof/AgenticAIContextRust/TransformGenerated/Types.lean"
        ),
        include_bytes!("../../../proofs/MRRProof/AgenticAIContextRust/TransformationTable.lean"),
        include_bytes!(
            "../../../proofs/MRRProof/src/mrr_proof_validation/transformation_source_proof.py"
        ),
        include_bytes!("../../../proofs/MRRProof/src/mrr_proof_validation/context_source_proof.py"),
    ] {
        hash.update((source.len() as u64).to_le_bytes());
        hash.update(source);
    }
    TransformationByteIdentityProof {
        identity: hash.finalize().into(),
        assumptions: Sha256::digest(b"Aeneas pinned Slice/Vec/CloneU8 models; Rust allocator; propext; Classical.choice; Quot.sound").into(),
    }
}
