//! Pure lookahead predicates for path-pattern dispatch.

use crate::syntax::{Token, TokenKind};

pub(super) fn looks_like_named_path_pattern(tokens: &[Token], index: usize) -> bool {
    if tokens.get(index).map(|token| token.kind) != Some(TokenKind::Identifier) {
        return false;
    }

    let mut offset = 1;
    while matches!(
        tokens.get(index + offset).map(|token| token.kind),
        Some(TokenKind::Whitespace | TokenKind::Comment)
    ) {
        offset += 1;
    }

    matches!(
        tokens.get(index + offset).map(|token| token.kind),
        Some(TokenKind::Punctuation('='))
    )
}
