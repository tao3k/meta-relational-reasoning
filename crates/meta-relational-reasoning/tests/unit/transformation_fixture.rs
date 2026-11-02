use crate::{
    ExternalRevisionIdentity, GenerationId, RevisionBinding, SemanticSnapshot,
    TransformationBinding, TransformationLimits,
};
use std::num::NonZeroUsize;

pub(super) fn limits() -> TransformationLimits {
    TransformationLimits {
        max_bytes: NonZeroUsize::new(16384).unwrap(),
        max_schema_nodes: NonZeroUsize::new(32).unwrap(),
        max_schema_depth: NonZeroUsize::new(8).unwrap(),
        max_dependencies: NonZeroUsize::new(16).unwrap(),
        max_steps: NonZeroUsize::new(4).unwrap(),
    }
}
pub(super) fn binding(generation: u8) -> TransformationBinding {
    let generation = GenerationId::from_canonical_bytes([generation]).unwrap();
    let revision = RevisionBinding::admit(
        ExternalRevisionIdentity::new("fixture", "source", "v1").unwrap(),
        generation,
    )
    .unwrap();
    let snapshot = SemanticSnapshot::admit(generation, vec![revision]).unwrap();
    TransformationBinding::new(&snapshot, [20; 32], [21; 32], [22; 32], 100, 200).unwrap()
}
