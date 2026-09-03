//! Token-boundary predicates shared by parser responsibilities.
#![forbid(unsafe_code)]

use super::Parser;
use crate::syntax::{
    GrammarParserAction, Keyword, TokenKind, is_non_reserved_word, top_level_parser_entrypoint,
};

impl Parser<'_> {
    pub(in crate::parser) fn is_expression_start(&self, kind: TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Identifier
                | TokenKind::DynamicParameter
                | TokenKind::SubstitutedParameter
                | TokenKind::String
                | TokenKind::ByteString
                | TokenKind::Number
                | TokenKind::Keyword(_)
                | TokenKind::Punctuation('+')
                | TokenKind::Punctuation('-')
                | TokenKind::Punctuation('(')
                | TokenKind::Punctuation('[')
                | TokenKind::Punctuation('{')
        )
    }

    pub(in crate::parser) fn is_clause_keyword(&self, kind: TokenKind) -> bool {
        let TokenKind::Keyword(keyword) = kind else {
            return false;
        };
        top_level_parser_entrypoint(keyword).is_some_and(|entrypoint| {
            matches!(
                entrypoint.action,
                GrammarParserAction::MatchClause
                    | GrammarParserAction::OptionalMatchClause
                    | GrammarParserAction::ReturnClause
                    | GrammarParserAction::FinishStatement
                    | GrammarParserAction::WhereClause
                    | GrammarParserAction::LetClause
                    | GrammarParserAction::FilterStatement
                    | GrammarParserAction::ForStatement
                    | GrammarParserAction::UnionClause
                    | GrammarParserAction::LimitClause
                    | GrammarParserAction::OrderByClause
                    | GrammarParserAction::OffsetClause
                    | GrammarParserAction::GroupByClause
                    | GrammarParserAction::InsertStatement
                    | GrammarParserAction::SetStatement
                    | GrammarParserAction::RemoveStatement
                    | GrammarParserAction::DeleteStatement
            )
        })
    }

    pub(in crate::parser) fn is_clause_boundary(&self, kind: TokenKind) -> bool {
        self.is_clause_keyword(kind) || kind == TokenKind::Punctuation(',')
    }

    pub(in crate::parser) fn is_expression_boundary(&self) -> bool {
        self.current_kind().is_none_or(|kind| {
            self.is_clause_boundary(kind)
                || matches!(
                    kind,
                    TokenKind::Punctuation(')' | ']')
                        | TokenKind::Keyword(
                            Keyword::When | Keyword::Then | Keyword::Else | Keyword::End
                        )
                )
        })
    }

    pub(in crate::parser) fn matches_kind(&self, kind: TokenKind) -> bool {
        self.current_kind() == Some(kind)
    }

    pub(in crate::parser) fn matches_keyword(&self, keyword: Keyword) -> bool {
        self.matches_kind(TokenKind::Keyword(keyword))
    }

    pub(in crate::parser) fn matches_regular_identifier(&self) -> bool {
        self.current().is_some_and(|token| match token.kind {
            TokenKind::Identifier => !is_delimited_identifier(token.text()),
            TokenKind::Keyword(_) => is_non_reserved_word(token.text()),
            _ => false,
        })
    }

    pub(in crate::parser) fn matches_identifier(&self) -> bool {
        self.current().is_some_and(|token| {
            token.kind == TokenKind::Identifier
                || matches!(token.kind, TokenKind::Keyword(_)) && is_non_reserved_word(token.text())
        })
    }

    pub(in crate::parser) fn peek_kind(&self, offset: usize) -> Option<TokenKind> {
        self.tokens.get(self.index + offset).map(|token| token.kind)
    }

    pub(in crate::parser) fn previous_kind(&self) -> Option<TokenKind> {
        self.index
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
            .map(|token| token.kind)
    }
}

fn is_delimited_identifier(text: &str) -> bool {
    matches!(text.as_bytes().first(), Some(b'"' | b'`'))
        || text.starts_with("@\"")
        || text.starts_with("@`")
}
