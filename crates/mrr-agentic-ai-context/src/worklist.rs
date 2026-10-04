//! Shared traversal control; adapters own identity storage and evidence errors.

use mrr_identity::FactId;

/// Source-extractable stack pop. Resizing only shrinks a Copy identity vector:
/// it neither allocates nor clones the fill value, and has constant-time cost.
pub(crate) fn pop_identity(pending: &mut Vec<FactId>) -> Option<FactId> {
    let length = pending.len();
    if length == 0 {
        return None;
    }
    let id = pending[length - 1];
    pending.resize(length - 1, id);
    Some(id)
}

pub(crate) fn append_identities(pending: &mut Vec<FactId>, dependencies: &[FactId]) {
    pending.extend_from_slice(dependencies);
}

pub(crate) trait Worklist {
    /// Advance one pop. Return false when finished, including rejected evidence.
    fn advance(&mut self) -> bool;
}

pub(crate) fn run<W: Worklist>(mut state: W) -> W {
    while state.advance() {}
    state
}
