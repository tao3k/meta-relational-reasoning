//! Production table operations isolated for source extraction.
//!
//! These functions implement access only. Catalog admission establishes the
//! transport laws; the caller supplies source authority and value conversion.

#[allow(
    clippy::manual_map,
    reason = "Explicit branches keep source extraction independent of Option::map."
)]
pub(crate) fn forward(table: &[usize], input: usize) -> Option<usize> {
    match table.get(input) {
        Some(answer) => Some(*answer),
        None => None,
    }
}

pub(crate) fn solve(table: &[Vec<usize>], input: usize) -> Option<usize> {
    match table.get(input) {
        Some(row) => forward(row, 0),
        None => None,
    }
}

pub(crate) fn extract(table: &[Vec<usize>], input: usize, answer: usize) -> Option<usize> {
    match table.get(input) {
        Some(row) => forward(row, answer),
        None => None,
    }
}
