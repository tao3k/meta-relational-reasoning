//! Shared traversal control; adapters own identity storage and evidence errors.

pub(crate) trait Worklist {
    /// Advance one pop. Return false when finished, including rejected evidence.
    fn advance(&mut self) -> bool;
}

pub(crate) fn run<W: Worklist>(mut state: W) -> W {
    while state.advance() {}
    state
}
