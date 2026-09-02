//! Value-type parameter, field, and nullability parsing support.

use crate::syntax::{SyntaxKind, TokenKind};

use crate::parser::core::engine::implementation::{Event, Parser, node};

impl Parser<'_> {
    pub(super) fn parse_type_parameter_list(
        &mut self,
        start: u32,
        diagnostic: &'static str,
    ) -> Vec<Event> {
        let mut children = vec![self.bump_event()];
        loop {
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation(')')) {
                children.push(self.bump_event());
                break;
            }
            if self.matches_kind(TokenKind::Number)
                || self.matches_word("YEAR")
                || self.matches_word("MONTH")
                || self.matches_word("DAY")
                || self.matches_word("SECOND")
                || self.matches_word("TO")
            {
                children.push(self.bump_event());
            } else {
                self.emit_match_syntax(
                    diagnostic,
                    "type parameter list requires numeric bounds or a duration qualifier",
                    self.next_span_or(start),
                );
                break;
            }
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation(',')) {
                children.push(self.bump_event());
                continue;
            }
            if !self.matches_kind(TokenKind::Punctuation(')')) {
                continue;
            }
        }
        node(SyntaxKind::TypeParameterList, children)
    }

    pub(super) fn parse_optional_type_bound(
        &mut self,
        start: u32,
        diagnostic: &'static str,
    ) -> Vec<Event> {
        if !self.matches_kind(TokenKind::Punctuation('[')) {
            return Vec::new();
        }
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        if self.matches_kind(TokenKind::Number) {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
        } else {
            self.emit_match_syntax(
                diagnostic,
                "LIST or ARRAY bound requires an unsigned integer",
                self.next_span_or(start),
            );
        }
        if self.matches_kind(TokenKind::Punctuation(']')) {
            children.push(self.bump_event());
        } else {
            self.emit_match_syntax(
                diagnostic,
                "LIST or ARRAY bound is missing `]`",
                self.next_span_or(start),
            );
        }
        node(SyntaxKind::TypeParameterList, children)
    }

    pub(super) fn parse_field_type_list(
        &mut self,
        start: u32,
        diagnostic: &'static str,
    ) -> Vec<Event> {
        let mut children = vec![self.bump_event()];
        loop {
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation('}')) {
                children.push(self.bump_event());
                break;
            }
            if !self.matches_kind(TokenKind::Identifier) {
                self.emit_match_syntax(
                    diagnostic,
                    "record field requires a field name",
                    self.next_span_or(start),
                );
                break;
            }
            let mut field = vec![self.bump_event()];
            field.extend(self.skip_trivia());
            field.extend(self.parse_typed_marker());
            if self.is_value_type_start() {
                field.extend(self.parse_property_value_type(start, diagnostic));
            } else {
                self.emit_match_syntax(
                    diagnostic,
                    "record field requires an ISO GQL value type",
                    self.next_span_or(start),
                );
            }
            children.extend(node(SyntaxKind::FieldType, field));
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation(',')) {
                children.push(self.bump_event());
                continue;
            }
            if !self.matches_kind(TokenKind::Punctuation('}')) {
                self.emit_match_syntax(
                    diagnostic,
                    "record fields require `,` or `}`",
                    self.next_span_or(start),
                );
                break;
            }
        }
        node(SyntaxKind::FieldTypeList, children)
    }

    pub(super) fn parse_optional_not_null(
        &mut self,
        start: u32,
        diagnostic: &'static str,
    ) -> Vec<Event> {
        if !self.matches_word("NOT") {
            return Vec::new();
        }
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        if self.matches_word("NULL") {
            children.push(self.bump_event());
        } else {
            self.emit_match_syntax(
                diagnostic,
                "NOT in a value type must be followed by NULL",
                self.next_span_or(start),
            );
        }
        node(SyntaxKind::NotNullConstraint, children)
    }

    pub(super) fn is_value_type_start(&self) -> bool {
        self.matches_word("ANY")
            || self.matches_word("LIST")
            || self.matches_word("ARRAY")
            || self.matches_word("RECORD")
            || self.matches_word("TABLE")
            || self.matches_word("BINDING")
            || self.matches_word("PROPERTY")
            || self.matches_word("GRAPH")
            || self.matches_word("NODE")
            || self.matches_word("VERTEX")
            || self.matches_word("EDGE")
            || self.matches_word("RELATIONSHIP")
            || self.matches_word("DIRECTED")
            || self.matches_word("UNDIRECTED")
            || self.matches_kind(TokenKind::Punctuation('{'))
            || self.is_predefined_type_start()
    }

    pub(super) fn is_predefined_type_start(&self) -> bool {
        const NAMES: &[&str] = &[
            "BOOL",
            "BOOLEAN",
            "STRING",
            "CHAR",
            "VARCHAR",
            "BYTES",
            "BINARY",
            "VARBINARY",
            "INT8",
            "INT16",
            "INT32",
            "INT64",
            "INT128",
            "INT256",
            "SMALLINT",
            "INT",
            "BIGINT",
            "UINT8",
            "UINT16",
            "UINT32",
            "UINT64",
            "UINT128",
            "UINT256",
            "USMALLINT",
            "UINT",
            "UBIGINT",
            "SIGNED",
            "UNSIGNED",
            "INTEGER8",
            "INTEGER16",
            "INTEGER32",
            "INTEGER64",
            "INTEGER128",
            "INTEGER256",
            "SMALL",
            "INTEGER",
            "BIG",
            "DECIMAL",
            "DEC",
            "FLOAT16",
            "FLOAT32",
            "FLOAT64",
            "FLOAT128",
            "FLOAT256",
            "FLOAT",
            "REAL",
            "DOUBLE",
            "ZONED",
            "LOCAL",
            "TIMESTAMP",
            "DATE",
            "TIME",
            "DURATION",
            "PATH",
            "NULL",
            "NOTHING",
        ];
        NAMES.iter().any(|name| self.matches_word(name))
    }
}
