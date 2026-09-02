//! Statement entrypoints owned by the lossless parser engine.

use gql_source::Span;

use crate::parser::path_pattern;
use crate::syntax::{Keyword, SyntaxKind, TokenKind, recovery_diagnostic};

use super::{Event, Parser, node};

impl Parser<'_> {
    pub(super) fn parse_create_schema_statement(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());

        let replace = if self.matches_kind(TokenKind::Keyword(Keyword::Or)) {
            let mut policy = vec![self.bump_event()];
            policy.extend(self.skip_trivia());
            let admitted = self.matches_kind(TokenKind::Keyword(Keyword::Replace));
            if admitted {
                policy.push(self.bump_event());
            } else {
                self.emit_match_syntax(
                    recovery_diagnostic("create-graph")
                        .expect("Gerbil grammar owns CREATE GRAPH recovery"),
                    "CREATE OR requires REPLACE",
                    self.next_span_or(start),
                );
            }
            children.extend(node(SyntaxKind::CatalogConflictClause, policy));
            children.extend(self.skip_trivia());
            admitted
        } else {
            false
        };

        if self.matches_kind(TokenKind::Keyword(Keyword::Schema)) {
            if replace {
                self.emit_match_syntax(
                    recovery_diagnostic("create-schema")
                        .expect("Gerbil grammar owns CREATE SCHEMA recovery"),
                    "CREATE OR REPLACE SCHEMA is not admitted by this profile",
                    self.next_span_or(start),
                );
            }
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Identifier) {
                children.push(self.bump_event());
            } else {
                self.emit_match_syntax(
                    recovery_diagnostic("create-schema")
                        .expect("Gerbil grammar owns CREATE SCHEMA recovery"),
                    "CREATE SCHEMA requires a schema name",
                    self.next_span_or(start),
                );
            }
            return node(SyntaxKind::CreateSchemaStatement, children);
        }

        if self.matches_kind(TokenKind::Keyword(Keyword::Property)) {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
        }
        if !self.matches_kind(TokenKind::Keyword(Keyword::Graph)) {
            self.emit_match_syntax(
                recovery_diagnostic("unsupported-statement")
                    .expect("Gerbil grammar owns unsupported-statement recovery"),
                "CREATE requires SCHEMA or GRAPH in this catalog profile",
                Span::new(start, start + 6),
            );
            return node(SyntaxKind::CreateSchemaStatement, children);
        }
        children.push(self.bump_event());
        children.extend(self.skip_trivia());

        let graph_type = if self.matches_kind(TokenKind::Keyword(Keyword::Type)) {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
            true
        } else {
            false
        };
        if !replace && self.matches_kind(TokenKind::Keyword(Keyword::If)) {
            children.extend(self.parse_catalog_conflict_clause(start, true));
            children.extend(self.skip_trivia());
        }

        let recovery = if graph_type {
            "create-graph-type"
        } else {
            "create-graph"
        };
        let name = self.parse_catalog_object_name(start, recovery);
        if name.is_empty() {
            return node(
                if graph_type {
                    SyntaxKind::CreateGraphTypeStatement
                } else {
                    SyntaxKind::CreateGraphStatement
                },
                children,
            );
        }
        children.extend(name);

        children.extend(self.skip_trivia());
        if graph_type {
            children.extend(self.parse_graph_type_source(start));
            return node(SyntaxKind::CreateGraphTypeStatement, children);
        }
        if self.matches_kind(TokenKind::Keyword(Keyword::Typed)) {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
        }
        if self.matches_kind(TokenKind::Keyword(Keyword::Any)) {
            children.push(self.bump_event());
        } else {
            self.emit_match_syntax(
                recovery_diagnostic("create-graph")
                    .expect("Gerbil grammar owns CREATE GRAPH recovery"),
                "CREATE GRAPH requires an ANY open graph type in this profile",
                self.next_span_or(start),
            );
        }

        node(SyntaxKind::CreateGraphStatement, children)
    }

    pub(super) fn parse_drop_schema_statement(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());

        if self.matches_kind(TokenKind::Keyword(Keyword::Schema)) {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Identifier) {
                children.push(self.bump_event());
            } else {
                self.emit_match_syntax(
                    recovery_diagnostic("drop-schema")
                        .expect("Gerbil grammar owns DROP SCHEMA recovery"),
                    "DROP SCHEMA requires a schema name",
                    self.next_span_or(start),
                );
            }
            return node(SyntaxKind::DropSchemaStatement, children);
        }

        if self.matches_kind(TokenKind::Keyword(Keyword::Property)) {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
        }
        if !self.matches_kind(TokenKind::Keyword(Keyword::Graph)) {
            self.emit_match_syntax(
                recovery_diagnostic("unsupported-statement")
                    .expect("Gerbil grammar owns unsupported statement recovery"),
                "DROP requires SCHEMA or GRAPH in this catalog profile",
                Span::new(start, start + 4),
            );
            return node(SyntaxKind::DropSchemaStatement, children);
        }
        children.push(self.bump_event());
        children.extend(self.skip_trivia());

        let graph_type = if self.matches_kind(TokenKind::Keyword(Keyword::Type)) {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
            true
        } else {
            false
        };
        if self.matches_kind(TokenKind::Keyword(Keyword::If)) {
            children.extend(self.parse_catalog_conflict_clause(start, false));
            children.extend(self.skip_trivia());
        }

        let recovery = if graph_type {
            "drop-graph-type"
        } else {
            "drop-graph"
        };
        children.extend(self.parse_catalog_object_name(start, recovery));

        node(
            if graph_type {
                SyntaxKind::DropGraphTypeStatement
            } else {
                SyntaxKind::DropGraphStatement
            },
            children,
        )
    }

    fn parse_catalog_conflict_clause(&mut self, start: u32, create: bool) -> Vec<Event> {
        let recovery = if create { "create-graph" } else { "drop-graph" };
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        if create {
            if self.matches_kind(TokenKind::Keyword(Keyword::Not)) {
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
            } else {
                self.emit_match_syntax(
                    recovery_diagnostic(recovery).expect("Gerbil grammar owns graph recovery"),
                    "CREATE GRAPH IF requires NOT EXISTS",
                    self.next_span_or(start),
                );
            }
        }
        if self.matches_kind(TokenKind::Keyword(Keyword::Exists)) {
            children.push(self.bump_event());
        } else {
            self.emit_match_syntax(
                recovery_diagnostic(recovery).expect("Gerbil grammar owns graph recovery"),
                if create {
                    "CREATE GRAPH IF NOT requires EXISTS"
                } else {
                    "DROP GRAPH IF requires EXISTS"
                },
                self.next_span_or(start),
            );
        }
        node(SyntaxKind::CatalogConflictClause, children)
    }

    fn parse_graph_type_source(&mut self, start: u32) -> Vec<Event> {
        let mut children = Vec::new();
        if self.matches_kind(TokenKind::Keyword(Keyword::As)) {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
        }
        if self.matches_kind(TokenKind::Punctuation('{')) {
            children.extend(self.parse_nested_graph_type_specification(start));
            return node(SyntaxKind::GraphTypeSource, children);
        }
        if self.matches_kind(TokenKind::Keyword(Keyword::Copy)) {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Keyword(Keyword::Of)) {
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
            } else {
                self.emit_match_syntax(
                    recovery_diagnostic("create-graph-type")
                        .expect("Gerbil grammar owns graph type recovery"),
                    "COPY graph type source requires OF",
                    self.next_span_or(start),
                );
            }
        } else if self.matches_kind(TokenKind::Keyword(Keyword::Like)) {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
        } else {
            self.emit_match_syntax(
                recovery_diagnostic("create-graph-type")
                    .expect("Gerbil grammar owns graph type recovery"),
                "CREATE GRAPH TYPE requires COPY OF or LIKE source",
                self.next_span_or(start),
            );
        }
        children.extend(self.parse_catalog_object_name(start, "create-graph-type"));
        node(SyntaxKind::GraphTypeSource, children)
    }

    fn parse_catalog_object_name(&mut self, start: u32, recovery: &'static str) -> Vec<Event> {
        if !self.matches_kind(TokenKind::Identifier) {
            self.emit_match_syntax(
                recovery_diagnostic(recovery).expect("Gerbil grammar owns catalog recovery"),
                "catalog object name requires an identifier",
                self.next_span_or(start),
            );
            return Vec::new();
        }
        let mut children = vec![self.bump_event()];
        loop {
            let checkpoint = self.index;
            let trivia = self.skip_trivia();
            if !self.matches_kind(TokenKind::Punctuation('.')) {
                self.index = checkpoint;
                break;
            }
            children.extend(trivia);
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Identifier) {
                children.push(self.bump_event());
            } else {
                self.emit_match_syntax(
                    recovery_diagnostic(recovery).expect("Gerbil grammar owns catalog recovery"),
                    "catalog object name requires an identifier after `.`",
                    self.next_span_or(start),
                );
                break;
            }
        }
        node(SyntaxKind::CatalogObjectName, children)
    }

    pub(super) fn parse_call_statement(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        let mut name = Vec::new();
        if self.matches_kind(TokenKind::Identifier) {
            name.push(self.bump_event());
            loop {
                let checkpoint = self.index;
                let trivia = self.skip_trivia();
                if !self.matches_kind(TokenKind::Punctuation('.')) {
                    self.index = checkpoint;
                    break;
                }
                name.extend(trivia);
                name.push(self.bump_event());
                name.extend(self.skip_trivia());
                if self.matches_kind(TokenKind::Identifier) {
                    name.push(self.bump_event());
                } else {
                    self.emit_match_syntax(
                        recovery_diagnostic("call-statement")
                            .expect("Gerbil grammar owns CALL recovery"),
                        "procedure name requires an identifier after `.`",
                        self.next_span_or(start),
                    );
                    break;
                }
            }
            children.extend(node(SyntaxKind::ProcedureName, name));
        } else {
            self.emit_match_syntax(
                recovery_diagnostic("call-statement").expect("Gerbil grammar owns CALL recovery"),
                "CALL requires a named procedure invocation",
                self.next_span_or(start),
            );
            return node(SyntaxKind::CallStatement, children);
        }

        children.extend(self.skip_trivia());
        if self.matches_kind(TokenKind::Punctuation('(')) {
            children.push(self.bump_event());
        } else {
            self.emit_match_syntax(
                recovery_diagnostic("call-statement").expect("Gerbil grammar owns CALL recovery"),
                "named procedure call requires `(`",
                self.next_span_or(start),
            );
            return node(SyntaxKind::CallStatement, children);
        }
        loop {
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation(')')) {
                children.push(self.bump_event());
                break;
            }
            if self.at_eof() {
                self.emit_match_syntax(
                    "GQL-PARSE-FUNCTION-CALL-SYNTAX",
                    "procedure call is missing `)`",
                    self.next_span_or(start),
                );
                break;
            }
            children.extend(self.parse_expression());
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation(',')) {
                children.push(self.bump_event());
                continue;
            }
            if self.matches_kind(TokenKind::Punctuation(')')) {
                children.push(self.bump_event());
                break;
            }
            self.emit_match_syntax(
                recovery_diagnostic("call-statement").expect("Gerbil grammar owns CALL recovery"),
                "procedure arguments require `,` or `)`",
                self.next_span_or(start),
            );
            break;
        }
        node(SyntaxKind::CallStatement, children)
    }

    pub(super) fn parse_start_transaction_statement(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        if self.matches_kind(TokenKind::Keyword(Keyword::Transaction)) {
            children.push(self.bump_event());
        } else {
            self.emit_match_syntax(
                recovery_diagnostic("transaction-command")
                    .expect("Gerbil grammar owns transaction recovery"),
                "START must be followed by TRANSACTION",
                self.next_span_or(start),
            );
            return node(SyntaxKind::StartTransactionStatement, children);
        }

        let checkpoint = self.index;
        children.extend(self.skip_trivia());
        if self.at_eof() {
            return node(SyntaxKind::StartTransactionStatement, children);
        }
        if self.matches_kind(TokenKind::Keyword(Keyword::Read)) {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
            if matches!(
                self.current_kind(),
                Some(TokenKind::Keyword(Keyword::Only | Keyword::Write))
            ) {
                children.push(self.bump_event());
            } else {
                self.emit_match_syntax(
                    recovery_diagnostic("transaction-command")
                        .expect("Gerbil grammar owns transaction recovery"),
                    "READ transaction mode requires ONLY or WRITE",
                    self.next_span_or(start),
                );
            }
        } else {
            self.index = checkpoint;
            self.emit_match_syntax(
                recovery_diagnostic("transaction-command")
                    .expect("Gerbil grammar owns transaction recovery"),
                "transaction mode must be READ ONLY or READ WRITE",
                self.next_span_or(start),
            );
        }
        node(SyntaxKind::StartTransactionStatement, children)
    }

    pub(super) fn parse_commit_statement(&mut self) -> Vec<Event> {
        node(SyntaxKind::CommitStatement, vec![self.bump_event()])
    }

    pub(super) fn parse_rollback_statement(&mut self) -> Vec<Event> {
        node(SyntaxKind::RollbackStatement, vec![self.bump_event()])
    }

    pub(super) fn parse_session_statement(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        let kind = match self.current_kind() {
            Some(TokenKind::Keyword(Keyword::Set)) => {
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
                if self.matches_kind(TokenKind::Keyword(Keyword::Schema)) {
                    children.push(self.bump_event());
                    children.extend(self.skip_trivia());
                    if self.matches_kind(TokenKind::Identifier) {
                        children.push(self.bump_event());
                    } else {
                        self.emit_match_syntax(
                            recovery_diagnostic("session-command")
                                .expect("Gerbil grammar owns session recovery"),
                            "SESSION SET SCHEMA requires a schema name",
                            self.next_span_or(start),
                        );
                    }
                } else {
                    self.emit_match_syntax(
                        recovery_diagnostic("session-command")
                            .expect("Gerbil grammar owns session recovery"),
                        "SESSION SET requires a supported setting",
                        self.next_span_or(start),
                    );
                }
                SyntaxKind::SessionSetStatement
            }
            Some(TokenKind::Keyword(Keyword::Reset)) => {
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
                if self.matches_kind(TokenKind::Keyword(Keyword::Schema)) {
                    children.push(self.bump_event());
                } else {
                    self.emit_match_syntax(
                        recovery_diagnostic("session-command")
                            .expect("Gerbil grammar owns session recovery"),
                        "SESSION RESET requires a supported setting",
                        self.next_span_or(start),
                    );
                }
                SyntaxKind::SessionResetStatement
            }
            Some(TokenKind::Keyword(Keyword::Close)) => {
                children.push(self.bump_event());
                SyntaxKind::SessionCloseStatement
            }
            _ => {
                self.emit_match_syntax(
                    recovery_diagnostic("session-command")
                        .expect("Gerbil grammar owns session recovery"),
                    "SESSION requires SET, RESET, or CLOSE",
                    self.next_span_or(start),
                );
                SyntaxKind::SessionSetStatement
            }
        };
        node(kind, children)
    }

    pub(super) fn parse_match_clause(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());

        let has_path_mode = matches!(
            self.current_kind(),
            Some(TokenKind::Keyword(
                Keyword::Walk | Keyword::Trail | Keyword::Acyclic | Keyword::Simple
            ))
        );
        if has_path_mode {
            children.extend(node(SyntaxKind::PathMode, vec![self.bump_event()]));
            children.extend(self.skip_trivia());
        }

        if path_pattern::looks_like_named_path_pattern(self.tokens, self.index)
            || self.matches_kind(TokenKind::Punctuation('('))
        {
            children.extend(self.parse_graph_pattern_list());
        } else {
            self.emit_match_syntax(
                if has_path_mode {
                    recovery_diagnostic("path-mode")
                        .expect("Gerbil grammar owns path mode recovery")
                } else {
                    "GQL-PARSE-MATCH-SYNTAX"
                },
                if has_path_mode {
                    "path mode must be followed by a graph pattern"
                } else {
                    "MATCH clause must start with a graph pattern"
                },
                self.next_span_or(start),
            );
        }

        node(SyntaxKind::MatchClause, children)
    }

    pub(in crate::parser) fn parse_graph_pattern_list(&mut self) -> Vec<Event> {
        let start = self.span_start();
        let diagnostic = recovery_diagnostic("match-pattern-list")
            .expect("Gerbil grammar owns MATCH pattern-list recovery");
        let mut children = Vec::new();

        loop {
            if path_pattern::looks_like_named_path_pattern(self.tokens, self.index) {
                children.extend(self.parse_named_path_pattern());
            } else if self.matches_kind(TokenKind::Punctuation('(')) {
                children.extend(self.parse_graph_pattern());
            } else {
                self.emit_match_syntax(
                    diagnostic,
                    "MATCH pattern separator must be followed by a graph pattern",
                    self.next_span_or(start),
                );
                break;
            }

            let separator_checkpoint = self.index;
            let separator_trivia = self.skip_trivia();
            if !self.matches_kind(TokenKind::Punctuation(',')) {
                self.index = separator_checkpoint;
                break;
            }
            children.extend(separator_trivia);
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
        }

        node(SyntaxKind::GraphPatternList, children)
    }
}
