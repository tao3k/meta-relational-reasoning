//! Lossless parser owner for graph match modes and per-path search prefixes.
#![forbid(unsafe_code)]

use super::{Event, Parser, node};
use crate::syntax::{SyntaxKind, TokenKind, recovery_diagnostic};

impl Parser<'_> {
    pub(super) fn parse_named_path_pattern(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        if self.matches_kind(TokenKind::Punctuation('=')) {
            children.push(self.bump_event());
        } else {
            self.emit_match_syntax(
                "GQL-PARSE-PATH-SYNTAX",
                "named path pattern must contain `=`",
                self.next_span_or(start),
            );
        }
        children.extend(self.skip_trivia());
        children.extend(self.parse_path_pattern_tail(start));
        node(SyntaxKind::PathPattern, children)
    }

    pub(super) fn parse_unnamed_path_pattern(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let children = self.parse_path_pattern_tail(start);
        node(SyntaxKind::PathPattern, children)
    }

    fn parse_path_pattern_tail(&mut self, start: u32) -> Vec<Event> {
        let mut children = Vec::new();
        let mut missing_pattern_recovery = "path";
        if self.is_path_prefix_start() {
            missing_pattern_recovery = if self.is_path_mode_start() {
                "path-mode"
            } else {
                "path-search-prefix"
            };
            children.extend(self.parse_path_prefix());
            children.extend(self.skip_trivia());
        }
        if self.matches_kind(TokenKind::Punctuation('(')) {
            children.extend(self.parse_graph_pattern());
        } else if missing_pattern_recovery == "path" {
            self.emit_match_syntax(
                "GQL-PARSE-PATH-SYNTAX",
                "named path binding must be followed by a path pattern expression",
                self.next_span_or(start),
            );
        } else {
            self.emit_path_syntax(
                missing_pattern_recovery,
                "path prefix must be followed by a path pattern expression",
                start,
            );
        }
        children
    }

    pub(super) fn is_graph_match_mode_start(&self) -> bool {
        self.matches_word("REPEATABLE") || self.matches_word("DIFFERENT")
    }

    pub(super) fn parse_graph_match_mode(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let repeatable = self.matches_word("REPEATABLE");
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());

        let singular_target = if repeatable {
            self.matches_word("ELEMENT")
        } else {
            self.matches_word("EDGE") || self.matches_word("RELATIONSHIP")
        };
        let plural_target = if repeatable {
            self.matches_word("ELEMENTS")
        } else {
            self.matches_word("EDGES") || self.matches_word("RELATIONSHIPS")
        };
        if singular_target || plural_target {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
            if singular_target && self.matches_word("BINDINGS") {
                children.push(self.bump_event());
            } else if self.matches_word("BINDING")
                || (plural_target && self.matches_word("BINDINGS"))
            {
                self.emit_path_syntax(
                    "graph-match-mode",
                    "match-mode bindings require a singular target followed by BINDINGS",
                    start,
                );
                children.push(self.bump_event());
            }
        } else {
            self.emit_path_syntax(
                "graph-match-mode",
                if repeatable {
                    "REPEATABLE requires ELEMENT, ELEMENTS, or ELEMENT BINDINGS"
                } else {
                    "DIFFERENT requires an edge synonym with optional BINDINGS"
                },
                start,
            );
            if matches!(
                self.current_kind(),
                Some(TokenKind::Identifier | TokenKind::Keyword(_))
            ) {
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
            }
        }
        node(SyntaxKind::GraphMatchMode, children)
    }

    pub(super) fn is_path_prefix_start(&self) -> bool {
        self.is_path_mode_start()
            || self.matches_word("ALL")
            || self.matches_word("ANY")
            || self.matches_word("SHORTEST")
    }

    fn is_path_mode_start(&self) -> bool {
        self.matches_word("WALK")
            || self.matches_word("TRAIL")
            || self.matches_word("SIMPLE")
            || self.matches_word("ACYCLIC")
    }

    pub(super) fn parse_path_prefix(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = Vec::new();
        let mut shortest = false;
        let mut bare_shortest = false;
        let mut count = false;
        let mut valid = true;

        if self.matches_word("ALL") || self.matches_word("ANY") || self.matches_word("SHORTEST") {
            let starts_all = self.matches_word("ALL");
            let starts_any = self.matches_word("ANY");
            let starts_shortest = self.matches_word("SHORTEST");
            bare_shortest = starts_shortest;
            let mut search = vec![self.bump_event()];
            search.extend(self.skip_trivia());

            if (starts_all || starts_any) && self.matches_word("SHORTEST") {
                shortest = true;
                search.push(self.bump_event());
                search.extend(self.skip_trivia());
            } else if (starts_any || starts_shortest) && self.is_path_count() {
                count = true;
                search.extend(self.parse_non_negative_integer_specification());
                search.extend(self.skip_trivia());
            } else if (starts_any || starts_shortest) && self.has_invalid_path_count() {
                valid = false;
                if self.matches_kind(TokenKind::Punctuation('-')) {
                    search.push(self.bump_event());
                }
                if self.current_kind() == Some(TokenKind::Number) {
                    search.push(self.bump_event());
                }
                search.extend(self.skip_trivia());
            }
            shortest |= starts_shortest;
            children.extend(node(SyntaxKind::PathSearch, search));
        }

        if self.matches_word("WALK")
            || self.matches_word("TRAIL")
            || self.matches_word("SIMPLE")
            || self.matches_word("ACYCLIC")
        {
            children.extend(node(SyntaxKind::PathMode, vec![self.bump_event()]));
            children.extend(self.skip_trivia());
        }

        if self.matches_word("PATH") || self.matches_word("PATHS") {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
        }

        let grouped = self.matches_word("GROUP") || self.matches_word("GROUPS");
        if grouped {
            children.push(self.bump_event());
        }
        if grouped && !shortest {
            valid = false;
        }
        if bare_shortest && !count && !grouped {
            valid = false;
        }
        if children.is_empty() {
            valid = false;
        }
        if !valid {
            self.emit_path_syntax(
                "path-search-prefix",
                "path search prefix has an invalid count, grouping, or path mode combination",
                start,
            );
        }
        node(SyntaxKind::PathPrefix, children)
    }

    fn is_path_count(&self) -> bool {
        self.is_non_negative_integer_specification()
    }

    fn has_invalid_path_count(&self) -> bool {
        matches!(
            self.current_kind(),
            Some(TokenKind::Number | TokenKind::Punctuation('-'))
        )
    }

    pub(super) fn parse_keep_clause(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        if self.is_path_prefix_start() {
            children.extend(self.parse_path_prefix());
        } else {
            self.emit_path_syntax(
                "keep-clause",
                "KEEP requires a path mode or path search prefix",
                start,
            );
        }
        node(SyntaxKind::KeepClause, children)
    }

    fn emit_path_syntax(&mut self, recovery: &'static str, message: &'static str, start: u32) {
        self.emit_match_syntax(
            recovery_diagnostic(recovery).expect("Gerbil grammar owns path-prefix recovery"),
            message,
            self.next_span_or(start),
        );
    }
}
