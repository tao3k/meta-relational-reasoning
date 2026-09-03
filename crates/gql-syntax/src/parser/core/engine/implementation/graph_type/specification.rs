//! Nested graph-type parser owner.

use crate::syntax::{Keyword, SyntaxKind, TokenKind, recovery_diagnostic};

use crate::parser::core::engine::implementation::{Event, Parser, node};

impl Parser<'_> {
    pub(in crate::parser::core::engine::implementation) fn parse_nested_graph_type_specification(
        &mut self,
        start: u32,
    ) -> Vec<Event> {
        let diagnostic = recovery_diagnostic("nested-graph-type")
            .expect("Gerbil grammar owns nested graph type recovery");
        let mut children = vec![self.bump_event()];
        loop {
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation('}')) {
                children.push(self.bump_event());
                break;
            }
            if self.at_eof() {
                self.emit_match_syntax(
                    diagnostic,
                    "nested graph type is missing `}`",
                    self.next_span_or(start),
                );
                break;
            }
            if self.matches_contextual_identifier("EDGE")
                || self.matches_contextual_identifier("RELATIONSHIP")
                || self.matches_contextual_identifier("DIRECTED")
                || self.matches_contextual_identifier("UNDIRECTED")
                || self.looks_like_unnamed_edge_type()
            {
                children.extend(self.parse_edge_type_specification(start));
            } else {
                children.extend(self.parse_node_type_specification(start));
            }
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation(',')) {
                children.push(self.bump_event());
                continue;
            }
            if !self.matches_kind(TokenKind::Punctuation('}')) {
                self.emit_match_syntax(
                    diagnostic,
                    "nested graph type elements require `,` or `}`",
                    self.next_span_or(start),
                );
                break;
            }
        }
        node(SyntaxKind::NestedGraphTypeSpecification, children)
    }

    pub(super) fn parse_node_type_specification(&mut self, start: u32) -> Vec<Event> {
        let diagnostic = recovery_diagnostic("nested-graph-type")
            .expect("Gerbil grammar owns nested graph type recovery");
        let mut children = Vec::new();
        let mut has_synonym = false;
        let mut has_name = false;
        if self.matches_contextual_identifier("NODE")
            || self.matches_contextual_identifier("VERTEX")
        {
            has_synonym = true;
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Keyword(Keyword::Type)) {
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
            }
            if self.matches_kind(TokenKind::Identifier) && !self.is_type_filler_start() {
                has_name = true;
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
            }
        }
        if self.matches_kind(TokenKind::Punctuation('(')) {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Identifier) && !self.is_type_filler_start() {
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
            }
            if self.is_type_filler_start() {
                children.extend(self.parse_type_filler(start, diagnostic));
                children.extend(self.skip_trivia());
            }
            if self.matches_kind(TokenKind::Punctuation(')')) {
                children.push(self.bump_event());
            } else {
                self.emit_match_syntax(
                    diagnostic,
                    "node type specification is missing `)`",
                    self.next_span_or(start),
                );
            }
        } else if has_synonym {
            if !has_name && !self.is_type_filler_start() {
                self.emit_match_syntax(
                    diagnostic,
                    "node type phrase requires a name or filler",
                    self.next_span_or(start),
                );
            }
            if self.is_type_filler_start() {
                children.extend(self.parse_type_filler(start, diagnostic));
                children.extend(self.skip_trivia());
            }
            if self.matches_kind(TokenKind::Keyword(Keyword::As)) {
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
                if self.matches_kind(TokenKind::Identifier) {
                    children.push(self.bump_event());
                } else {
                    self.emit_match_syntax(
                        diagnostic,
                        "node type phrase AS requires a local alias",
                        self.next_span_or(start),
                    );
                }
            }
        } else {
            self.emit_match_syntax(
                diagnostic,
                "node type pattern requires `(`",
                self.next_span_or(start),
            );
        }
        node(SyntaxKind::NodeTypeSpecification, children)
    }

    pub(super) fn parse_edge_type_specification(&mut self, start: u32) -> Vec<Event> {
        let diagnostic = recovery_diagnostic("nested-graph-type")
            .expect("Gerbil grammar owns nested graph type recovery");
        let mut children = Vec::new();
        if self.matches_contextual_identifier("DIRECTED")
            || self.matches_contextual_identifier("UNDIRECTED")
        {
            children.extend(node(SyntaxKind::EdgeKind, vec![self.bump_event()]));
            children.extend(self.skip_trivia());
        }
        if self.matches_contextual_identifier("EDGE")
            || self.matches_contextual_identifier("RELATIONSHIP")
        {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Keyword(Keyword::Type)) {
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
            }
            if self.matches_kind(TokenKind::Identifier)
                && !self.matches_contextual_identifier("CONNECTING")
                && !self.is_type_filler_start()
            {
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
            }
        }

        if self.matches_kind(TokenKind::Punctuation('(')) {
            children.extend(self.parse_edge_type_pattern(start, diagnostic));
        } else {
            if self.is_type_filler_start() {
                children.extend(self.parse_type_filler(start, diagnostic));
                children.extend(self.skip_trivia());
            }
            children.extend(self.parse_endpoint_pair_phrase(start, diagnostic));
        }
        node(SyntaxKind::EdgeTypeSpecification, children)
    }

    fn parse_edge_type_pattern(&mut self, start: u32, diagnostic: &'static str) -> Vec<Event> {
        let mut children = self.parse_node_type_reference(start);
        children.extend(self.skip_trivia());

        match self.current_kind() {
            Some(TokenKind::Punctuation('-')) => {
                children.extend(node(SyntaxKind::EdgeDirection, vec![self.bump_event()]));
                self.parse_edge_type_filler(
                    start,
                    diagnostic,
                    &mut children,
                    &['['],
                    &[']', '-', '>'],
                );
            }
            Some(TokenKind::Punctuation('<')) => {
                children.extend(node(SyntaxKind::EdgeDirection, vec![self.bump_event()]));
                self.parse_edge_type_filler(
                    start,
                    diagnostic,
                    &mut children,
                    &['-', '['],
                    &[']', '-'],
                );
            }
            Some(TokenKind::Punctuation('~')) => {
                children.extend(node(SyntaxKind::EdgeDirection, vec![self.bump_event()]));
                self.parse_edge_type_filler(start, diagnostic, &mut children, &['['], &[']', '~']);
            }
            _ => self.emit_match_syntax(
                diagnostic,
                "edge type requires a directed or undirected arc",
                self.next_span_or(start),
            ),
        }

        children.extend(self.skip_trivia());
        children.extend(self.parse_node_type_reference(start));
        children
    }

    fn parse_endpoint_pair_phrase(&mut self, start: u32, diagnostic: &'static str) -> Vec<Event> {
        let mut children = Vec::new();
        if self.matches_contextual_identifier("CONNECTING") {
            children.push(self.bump_event());
        } else {
            self.emit_match_syntax(
                diagnostic,
                "edge type phrase requires CONNECTING",
                self.next_span_or(start),
            );
            return node(SyntaxKind::EndpointPair, children);
        }
        children.extend(self.skip_trivia());
        if self.matches_kind(TokenKind::Punctuation('(')) {
            children.push(self.bump_event());
        } else {
            self.emit_match_syntax(
                diagnostic,
                "edge endpoint pair requires an opening parenthesis",
                self.next_span_or(start),
            );
            return node(SyntaxKind::EndpointPair, children);
        }
        children.extend(self.skip_trivia());
        children.extend(self.parse_endpoint_alias(start, diagnostic));
        children.extend(self.skip_trivia());
        let direction = match self.current_kind() {
            Some(TokenKind::Identifier) if self.matches_contextual_identifier("TO") => {
                vec![self.bump_event()]
            }
            Some(TokenKind::Punctuation('-')) => {
                let mut connector = vec![self.bump_event()];
                if self.matches_kind(TokenKind::Punctuation('>')) {
                    connector.push(self.bump_event());
                } else {
                    self.emit_match_syntax(
                        diagnostic,
                        "edge endpoint connector requires a right arrow",
                        self.next_span_or(start),
                    );
                }
                connector
            }
            Some(TokenKind::Punctuation('<')) => {
                let mut connector = vec![self.bump_event()];
                if self.matches_kind(TokenKind::Punctuation('-')) {
                    connector.push(self.bump_event());
                } else {
                    self.emit_match_syntax(
                        diagnostic,
                        "edge endpoint connector requires a left arrow",
                        self.next_span_or(start),
                    );
                }
                connector
            }
            Some(TokenKind::Punctuation('~')) => vec![self.bump_event()],
            _ => {
                self.emit_match_syntax(
                    diagnostic,
                    "edge endpoint pair requires TO, an arrow, or a tilde",
                    self.next_span_or(start),
                );
                Vec::new()
            }
        };
        children.extend(node(SyntaxKind::EdgeDirection, direction));
        children.extend(self.skip_trivia());
        children.extend(self.parse_endpoint_alias(start, diagnostic));
        children.extend(self.skip_trivia());
        if self.matches_kind(TokenKind::Punctuation(')')) {
            children.push(self.bump_event());
        } else {
            self.emit_match_syntax(
                diagnostic,
                "edge endpoint pair is missing its closing parenthesis",
                self.next_span_or(start),
            );
        }
        node(SyntaxKind::EndpointPair, children)
    }

    fn parse_endpoint_alias(&mut self, start: u32, diagnostic: &'static str) -> Vec<Event> {
        if self.matches_kind(TokenKind::Identifier) {
            node(SyntaxKind::NodeTypeReference, vec![self.bump_event()])
        } else {
            self.emit_match_syntax(
                diagnostic,
                "edge endpoint requires a node type alias",
                self.next_span_or(start),
            );
            node(SyntaxKind::NodeTypeReference, Vec::new())
        }
    }

    fn parse_edge_type_filler(
        &mut self,
        start: u32,
        diagnostic: &'static str,
        children: &mut Vec<Event>,
        prefix: &[char],
        suffix: &[char],
    ) {
        for &punctuation in prefix {
            if self.matches_kind(TokenKind::Punctuation(punctuation)) {
                children.push(self.bump_event());
            } else {
                self.emit_match_syntax(
                    diagnostic,
                    format!("edge type arc requires `{punctuation}`"),
                    self.next_span_or(start),
                );
                return;
            }
        }
        children.extend(self.skip_trivia());
        if self.is_type_filler_start() {
            children.extend(self.parse_type_filler(start, diagnostic));
            children.extend(self.skip_trivia());
        } else {
            self.emit_match_syntax(
                diagnostic,
                "edge type arc requires an edge type filler",
                self.next_span_or(start),
            );
        }
        for &punctuation in suffix {
            if self.matches_kind(TokenKind::Punctuation(punctuation)) {
                children.push(self.bump_event());
            } else {
                self.emit_match_syntax(
                    diagnostic,
                    format!("edge type arc requires `{punctuation}`"),
                    self.next_span_or(start),
                );
                return;
            }
        }
    }

    fn parse_node_type_reference(&mut self, start: u32) -> Vec<Event> {
        let diagnostic = recovery_diagnostic("nested-graph-type")
            .expect("Gerbil grammar owns nested graph type recovery");
        let mut children = Vec::new();
        if self.matches_kind(TokenKind::Punctuation('(')) {
            children.push(self.bump_event());
        } else {
            self.emit_match_syntax(
                diagnostic,
                "edge endpoint requires `(`",
                self.next_span_or(start),
            );
            return node(SyntaxKind::NodeTypeReference, children);
        }
        children.extend(self.skip_trivia());
        if self.matches_kind(TokenKind::Identifier) && !self.is_type_filler_start() {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
        } else if self.is_type_filler_start() {
            children.extend(self.parse_type_filler(start, diagnostic));
            children.extend(self.skip_trivia());
        } else if self.matches_kind(TokenKind::Punctuation(')')) {
        } else {
            self.emit_match_syntax(
                diagnostic,
                "edge endpoint requires an alias or inline node type filler",
                self.next_span_or(start),
            );
        }
        if self.matches_kind(TokenKind::Punctuation(')')) {
            children.push(self.bump_event());
        } else {
            self.emit_match_syntax(
                diagnostic,
                "edge endpoint is missing `)`",
                self.next_span_or(start),
            );
        }
        node(SyntaxKind::NodeTypeReference, children)
    }

    fn parse_type_filler(&mut self, start: u32, diagnostic: &'static str) -> Vec<Event> {
        let mut children = Vec::new();
        if self.is_implies_start() {
            children.extend(node(
                SyntaxKind::KeyLabelSet,
                self.parse_implies(start, diagnostic),
            ));
        } else if self.is_label_set_start() {
            let phrase = self.parse_label_set_phrase(start, diagnostic);
            let trivia = self.skip_trivia();
            if self.is_implies_start() {
                let mut key_labels = phrase;
                key_labels.extend(trivia);
                key_labels.extend(self.parse_implies(start, diagnostic));
                children.extend(node(SyntaxKind::KeyLabelSet, key_labels));
            } else {
                children.extend(phrase);
                children.extend(trivia);
            }
        }
        children.extend(self.skip_trivia());
        if self.is_label_set_start() {
            children.extend(self.parse_label_set_phrase(start, diagnostic));
            children.extend(self.skip_trivia());
        }
        if self.matches_kind(TokenKind::Punctuation('{')) {
            children.extend(self.parse_property_type_list(start));
        }
        children
    }

    fn parse_label_set_phrase(&mut self, start: u32, diagnostic: &'static str) -> Vec<Event> {
        let single = self.matches_contextual_identifier("LABEL");
        let mut children = vec![self.bump_event()];
        children.extend(self.skip_trivia());
        if !self.matches_kind(TokenKind::Identifier) {
            self.emit_match_syntax(
                diagnostic,
                "label set phrase requires a label name",
                self.next_span_or(start),
            );
            return node(SyntaxKind::LabelSetPhrase, children);
        }
        children.push(self.bump_event());
        if !single {
            loop {
                let trivia = self.skip_trivia();
                if !self.matches_kind(TokenKind::Punctuation('&')) {
                    children.extend(trivia);
                    break;
                }
                children.extend(trivia);
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
                if self.matches_kind(TokenKind::Identifier) {
                    children.push(self.bump_event());
                } else {
                    self.emit_match_syntax(
                        diagnostic,
                        "label set `&` requires a following label name",
                        self.next_span_or(start),
                    );
                    break;
                }
            }
        }
        node(SyntaxKind::LabelSetPhrase, children)
    }

    fn parse_implies(&mut self, start: u32, diagnostic: &'static str) -> Vec<Event> {
        if self.matches_contextual_identifier("IMPLIES") {
            return vec![self.bump_event()];
        }
        let mut children = Vec::new();
        if self.matches_kind(TokenKind::Punctuation('=')) {
            children.push(self.bump_event());
            if self.matches_kind(TokenKind::Punctuation('>')) {
                children.push(self.bump_event());
            } else {
                self.emit_match_syntax(
                    diagnostic,
                    "key label set `=` requires adjacent `>`",
                    self.next_span_or(start),
                );
            }
        }
        children
    }

    fn is_type_filler_start(&self) -> bool {
        self.is_label_set_start()
            || self.is_implies_start()
            || self.matches_kind(TokenKind::Punctuation('{'))
    }

    fn is_label_set_start(&self) -> bool {
        self.matches_contextual_identifier("LABEL")
            || self.matches_contextual_identifier("LABELS")
            || self.matches_kind(TokenKind::Keyword(Keyword::Is))
            || self.matches_kind(TokenKind::Punctuation(':'))
    }

    fn is_implies_start(&self) -> bool {
        self.matches_contextual_identifier("IMPLIES")
            || self.matches_kind(TokenKind::Punctuation('='))
    }

    fn parse_property_type_list(&mut self, start: u32) -> Vec<Event> {
        let diagnostic = recovery_diagnostic("nested-graph-type")
            .expect("Gerbil grammar owns nested graph type recovery");
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
                    "property type requires a property name",
                    self.next_span_or(start),
                );
                break;
            }
            let mut property = vec![self.bump_event()];
            property.extend(self.skip_trivia());
            property.extend(self.parse_typed_marker());
            if self.is_value_type_start() {
                property.extend(self.parse_property_value_type(start, diagnostic));
            } else {
                self.emit_match_syntax(
                    diagnostic,
                    "property type requires an ISO GQL value type",
                    self.next_span_or(start),
                );
            }
            children.extend(node(SyntaxKind::PropertyType, property));
            children.extend(self.skip_trivia());
            if self.matches_kind(TokenKind::Punctuation(',')) {
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
                if self.matches_kind(TokenKind::Punctuation('}')) {
                    self.emit_match_syntax(
                        diagnostic,
                        "property type list does not admit a trailing comma",
                        self.next_span_or(start),
                    );
                }
                continue;
            }
            if !self.matches_kind(TokenKind::Punctuation('}')) {
                self.emit_match_syntax(
                    diagnostic,
                    "property types require `,` or `}`",
                    self.next_span_or(start),
                );
                break;
            }
        }
        node(SyntaxKind::PropertyTypeList, children)
    }

    pub(super) fn parse_typed_marker(&mut self) -> Vec<Event> {
        let mut children = Vec::new();
        if self.matches_word("TYPED") {
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
        } else if self.matches_kind(TokenKind::Punctuation(':')) {
            children.push(self.bump_event());
            if self.matches_kind(TokenKind::Punctuation(':')) {
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
            }
        }
        children
    }

    pub(in crate::parser) fn parse_property_value_type(
        &mut self,
        start: u32,
        diagnostic: &'static str,
    ) -> Vec<Event> {
        let mut children = node(
            SyntaxKind::ValueTypeAtom,
            self.parse_value_type_atom(start, diagnostic),
        );

        if self.next_significant_matches_word("LIST") || self.next_significant_matches_word("ARRAY")
        {
            children.extend(self.skip_trivia());
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
            children.extend(self.parse_optional_type_bound(start, diagnostic));
            if self.next_significant_matches_word("NOT") {
                children.extend(self.skip_trivia());
            }
            children.extend(self.parse_optional_not_null(start, diagnostic));
        }

        while self.next_significant_kind() == Some(TokenKind::Punctuation('|')) {
            children.extend(self.skip_trivia());
            children.push(self.bump_event());
            children.extend(self.skip_trivia());
            if !self.is_value_type_start() {
                self.emit_match_syntax(
                    diagnostic,
                    "closed dynamic union requires a value type after `|`",
                    self.next_span_or(start),
                );
                break;
            }
            children.extend(node(
                SyntaxKind::ValueTypeAtom,
                self.parse_value_type_atom(start, diagnostic),
            ));
        }

        node(SyntaxKind::PropertyValueType, children)
    }

    fn parse_value_type_atom(&mut self, start: u32, diagnostic: &'static str) -> Vec<Event> {
        let mut children = Vec::new();
        if self.matches_word("PROPERTY") || self.matches_word("GRAPH") {
            if self.matches_word("PROPERTY") {
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
            }
            if self.matches_word("GRAPH") {
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
            } else {
                self.emit_match_syntax(
                    diagnostic,
                    "PROPERTY in a reference value type must be followed by GRAPH",
                    self.next_span_or(start),
                );
            }
            if self.matches_kind(TokenKind::Punctuation('{')) {
                children.extend(self.parse_nested_graph_type_specification(start));
            } else {
                self.emit_match_syntax(
                    diagnostic,
                    "closed graph reference type requires a nested graph type specification",
                    self.next_span_or(start),
                );
            }
            if self.next_significant_matches_word("NOT") {
                children.extend(self.skip_trivia());
            }
            children.extend(self.parse_optional_not_null(start, diagnostic));
            return node(SyntaxKind::ReferenceValueType, children);
        }
        if self.matches_word("DIRECTED")
            || self.matches_word("UNDIRECTED")
            || (self.matches_word("EDGE") && self.reference_phrase_has_specification())
            || (self.matches_word("RELATIONSHIP") && self.reference_phrase_has_specification())
        {
            children.extend(self.parse_edge_type_specification(start));
            if self.next_significant_matches_word("NOT") {
                children.extend(self.skip_trivia());
            }
            children.extend(self.parse_optional_not_null(start, diagnostic));
            return node(SyntaxKind::ReferenceValueType, children);
        }
        if (self.matches_word("NODE") || self.matches_word("VERTEX"))
            && self.reference_phrase_has_specification()
        {
            children.extend(self.parse_node_type_specification(start));
            if self.next_significant_matches_word("NOT") {
                children.extend(self.skip_trivia());
            }
            children.extend(self.parse_optional_not_null(start, diagnostic));
            return node(SyntaxKind::ReferenceValueType, children);
        }
        if self.matches_word("NODE")
            || self.matches_word("VERTEX")
            || self.matches_word("EDGE")
            || self.matches_word("RELATIONSHIP")
        {
            children.push(self.bump_event());
            if self.next_significant_matches_word("NOT") {
                children.extend(self.skip_trivia());
            }
            children.extend(self.parse_optional_not_null(start, diagnostic));
            return node(SyntaxKind::ReferenceValueType, children);
        }
        if self.matches_word("TABLE") || self.matches_word("BINDING") {
            if self.matches_word("BINDING") {
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
            }
            if self.matches_word("TABLE") {
                children.push(self.bump_event());
            } else {
                self.emit_match_syntax(
                    diagnostic,
                    "BINDING in a value type must be followed by TABLE",
                    self.next_span_or(start),
                );
            }
            if self.next_significant_kind() == Some(TokenKind::Punctuation('{')) {
                children.extend(self.skip_trivia());
                children.extend(self.parse_field_type_list(start, diagnostic));
            } else {
                self.emit_match_syntax(
                    diagnostic,
                    "binding-table type requires a field-type specification",
                    self.next_span_or(start),
                );
            }
            if self.next_significant_matches_word("NOT") {
                children.extend(self.skip_trivia());
            }
            children.extend(self.parse_optional_not_null(start, diagnostic));
            return node(SyntaxKind::ReferenceValueType, children);
        }
        if self.matches_word("LIST") || self.matches_word("ARRAY") {
            children.push(self.bump_event());
            if self.next_significant_kind() == Some(TokenKind::Punctuation('<')) {
                children.extend(self.skip_trivia());
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
                children.extend(self.parse_property_value_type(start, diagnostic));
                children.extend(self.skip_trivia());
                if self.matches_kind(TokenKind::Punctuation('>')) {
                    children.push(self.bump_event());
                } else {
                    self.emit_match_syntax(
                        diagnostic,
                        "parameterized LIST or ARRAY type is missing `>`",
                        self.next_span_or(start),
                    );
                }
            }
            if self.next_significant_kind() == Some(TokenKind::Punctuation('[')) {
                children.extend(self.skip_trivia());
            }
            children.extend(self.parse_optional_type_bound(start, diagnostic));
            if self.next_significant_matches_word("NOT") {
                children.extend(self.skip_trivia());
            }
            children.extend(self.parse_optional_not_null(start, diagnostic));
            return children;
        }

        if self.matches_word("ANY") {
            children.push(self.bump_event());
            if self.next_significant_matches_word("RECORD") {
                children.extend(self.skip_trivia());
                children.push(self.bump_event());
                if self.next_significant_matches_word("NOT") {
                    children.extend(self.skip_trivia());
                }
                children.extend(self.parse_optional_not_null(start, diagnostic));
                return children;
            }
            if self.next_significant_matches_word("PROPERTY") {
                children.extend(self.skip_trivia());
                children.push(self.bump_event());
            }
            if self.next_significant_matches_word("GRAPH")
                || self.next_significant_matches_word("NODE")
                || self.next_significant_matches_word("VERTEX")
                || self.next_significant_matches_word("EDGE")
                || self.next_significant_matches_word("RELATIONSHIP")
            {
                children.extend(self.skip_trivia());
                children.push(self.bump_event());
                if self.next_significant_matches_word("NOT") {
                    children.extend(self.skip_trivia());
                }
                children.extend(self.parse_optional_not_null(start, diagnostic));
                return node(SyntaxKind::ReferenceValueType, children);
            }
            if self.next_significant_matches_word("VALUE") {
                children.extend(self.skip_trivia());
                children.push(self.bump_event());
            }
            if self.next_significant_kind() == Some(TokenKind::Punctuation('<')) {
                children.extend(self.skip_trivia());
                children.push(self.bump_event());
                children.extend(self.skip_trivia());
                children.extend(self.parse_property_value_type(start, diagnostic));
                children.extend(self.skip_trivia());
                if self.matches_kind(TokenKind::Punctuation('>')) {
                    children.push(self.bump_event());
                } else {
                    self.emit_match_syntax(
                        diagnostic,
                        "closed dynamic union is missing `>`",
                        self.next_span_or(start),
                    );
                }
            }
            if self.next_significant_matches_word("NOT") {
                children.extend(self.skip_trivia());
            }
            children.extend(self.parse_optional_not_null(start, diagnostic));
            return children;
        }

        if self.matches_word("RECORD") || self.matches_kind(TokenKind::Punctuation('{')) {
            if self.matches_word("RECORD") {
                children.push(self.bump_event());
            }
            if self.next_significant_kind() == Some(TokenKind::Punctuation('{')) {
                children.extend(self.skip_trivia());
                children.extend(self.parse_field_type_list(start, diagnostic));
            }
            if self.next_significant_matches_word("NOT") {
                children.extend(self.skip_trivia());
            }
            children.extend(self.parse_optional_not_null(start, diagnostic));
            return children;
        }

        if !self.is_predefined_type_start() {
            self.emit_match_syntax(
                diagnostic,
                "property type requires a recognized ISO GQL value type",
                self.next_span_or(start),
            );
            return children;
        }

        let head = self
            .current()
            .map(|token| token.text().to_ascii_uppercase());
        children.push(self.bump_event());
        match head.as_deref() {
            Some("SIGNED" | "UNSIGNED") if self.is_verbose_integer_type_start() => {
                children.extend(self.skip_trivia());
                let verbose_head = self
                    .current()
                    .map(|token| token.text().to_ascii_uppercase());
                children.push(self.bump_event());
                if matches!(verbose_head.as_deref(), Some("SMALL" | "BIG"))
                    && self.next_significant_matches_word("INTEGER")
                {
                    children.extend(self.skip_trivia());
                    children.push(self.bump_event());
                }
            }
            Some("DOUBLE") if self.next_significant_matches_word("PRECISION") => {
                children.extend(self.skip_trivia());
                children.push(self.bump_event());
            }
            Some("ZONED" | "LOCAL")
                if self.next_significant_matches_word("DATETIME")
                    || self.next_significant_matches_word("TIME") =>
            {
                children.extend(self.skip_trivia());
                children.push(self.bump_event());
            }
            Some("TIMESTAMP" | "TIME")
                if self.next_significant_matches_word("WITH")
                    || self.next_significant_matches_word("WITHOUT") =>
            {
                children.extend(self.skip_trivia());
                children.push(self.bump_event());
                if self.next_significant_matches_word("TIME") {
                    children.extend(self.skip_trivia());
                    children.push(self.bump_event());
                }
                if self.next_significant_matches_word("ZONE") {
                    children.extend(self.skip_trivia());
                    children.push(self.bump_event());
                }
            }
            Some("SMALL" | "BIG") if self.next_significant_matches_word("INTEGER") => {
                children.extend(self.skip_trivia());
                children.push(self.bump_event());
            }
            _ => {}
        }
        if self.next_significant_kind() == Some(TokenKind::Punctuation('(')) {
            children.extend(self.skip_trivia());
            children.extend(self.parse_type_parameter_list(start, diagnostic));
        }
        if self.next_significant_matches_word("NOT") {
            children.extend(self.skip_trivia());
        }
        children.extend(self.parse_optional_not_null(start, diagnostic));
        children
    }

    fn looks_like_unnamed_edge_type(&self) -> bool {
        let mut index = self.index;
        if self.tokens.get(index).map(|token| token.kind) != Some(TokenKind::Punctuation('(')) {
            return false;
        }
        index += 1;
        while self
            .tokens
            .get(index)
            .is_some_and(|token| matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))
        {
            index += 1;
        }
        if self.tokens.get(index).map(|token| token.kind) == Some(TokenKind::Identifier) {
            index += 1;
        }
        while self
            .tokens
            .get(index)
            .is_some_and(|token| matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))
        {
            index += 1;
        }
        if self.tokens.get(index).map(|token| token.kind) != Some(TokenKind::Punctuation(')')) {
            return false;
        }
        index += 1;
        while self
            .tokens
            .get(index)
            .is_some_and(|token| matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))
        {
            index += 1;
        }
        matches!(
            self.tokens.get(index).map(|token| token.kind),
            Some(TokenKind::Punctuation('-' | '<' | '~'))
        )
    }

    fn matches_contextual_identifier(&self, expected: &str) -> bool {
        self.matches_kind(TokenKind::Identifier)
            && self.tokens[self.index]
                .text()
                .eq_ignore_ascii_case(expected)
    }

    pub(in crate::parser) fn matches_word(&self, expected: &str) -> bool {
        self.current()
            .is_some_and(|token| token.text().eq_ignore_ascii_case(expected))
    }

    fn next_significant_kind(&self) -> Option<TokenKind> {
        self.tokens[self.index..]
            .iter()
            .find(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))
            .map(|token| token.kind)
    }

    fn next_significant_matches_word(&self, expected: &str) -> bool {
        self.tokens[self.index..]
            .iter()
            .find(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))
            .is_some_and(|token| token.text().eq_ignore_ascii_case(expected))
    }

    fn reference_phrase_has_specification(&self) -> bool {
        self.tokens[self.index + 1..]
            .iter()
            .find(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))
            .is_some_and(|token| {
                token.kind == TokenKind::Keyword(Keyword::Type)
                    || token.kind == TokenKind::Identifier
                    || matches!(token.kind, TokenKind::Punctuation('(' | '{' | ':'))
            })
    }

    fn is_verbose_integer_type_start(&self) -> bool {
        const NAMES: &[&str] = &[
            "INTEGER8",
            "INTEGER16",
            "INTEGER32",
            "INTEGER64",
            "INTEGER128",
            "INTEGER256",
            "SMALL",
            "INTEGER",
            "BIG",
        ];
        self.tokens[self.index..]
            .iter()
            .find(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))
            .is_some_and(|token| {
                NAMES
                    .iter()
                    .any(|name| token.text().eq_ignore_ascii_case(name))
            })
    }
}
