//! Lossless Rowan sink for parser-owned ParseArtifact V1 events.

use std::sync::Arc;

use rowan::{GreenNode, GreenNodeBuilder};

use super::{
    ParseArtifact, ParseArtifactStatus, ParseEvent, ParserKindCatalog, ParserKindCategory,
};

/// Stable numeric syntax kind assigned by parser grammar declaration order.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ParserSyntaxKind(u16);

impl ParserSyntaxKind {
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }
}

/// Rowan language marker for a parser-owned language-selected catalog.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ParserSyntax {}

impl rowan::Language for ParserSyntax {
    type Kind = ParserSyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        ParserSyntaxKind(raw.0)
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind.0)
    }
}

/// One lossless Rowan tree paired with its parser-owned kind catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParserCst {
    green: GreenNode,
    catalog: Arc<ParserKindCatalog>,
}

impl ParserCst {
    #[must_use]
    pub fn root(&self) -> rowan::SyntaxNode<ParserSyntax> {
        rowan::SyntaxNode::new_root(self.green.clone())
    }

    #[must_use]
    pub fn catalog(&self) -> &ParserKindCatalog {
        &self.catalog
    }

    /// Resolves a Rowan kind through the parser-published catalog.
    #[must_use]
    pub fn kind_name(&self, kind: ParserSyntaxKind) -> Option<&str> {
        self.catalog.kind_name(kind.raw())
    }
}

/// Fail-closed structural errors while sinking parser events into Rowan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParserCstError {
    RejectedArtifact,
    UnknownNodeKind { event: usize },
    UnknownTokenKind { event: usize },
    KindCategoryMismatch { event: usize },
    InvalidEventOrder { event: usize },
    InvalidOffset { event: usize },
    MissingRoot,
}

impl ParseArtifact {
    /// Validates and sinks the complete accepted event stream into Rowan.
    pub fn to_rowan_cst(&self) -> Result<ParserCst, ParserCstError> {
        if self.status != ParseArtifactStatus::Accepted {
            return Err(ParserCstError::RejectedArtifact);
        }
        let mut builder = GreenNodeBuilder::new();
        let mut nodes = Vec::new();
        let mut fields = Vec::new();
        let mut offset = 0_u32;
        let mut roots = 0_u32;
        for (index, event) in self.events.iter().enumerate() {
            match event {
                ParseEvent::StartNode { id, kind, start } => {
                    if nodes.is_empty() && roots != 0 {
                        return Err(ParserCstError::InvalidEventOrder { event: index });
                    }
                    if *start != offset {
                        return Err(ParserCstError::InvalidOffset { event: index });
                    }
                    let kind_id = self
                        .kind_catalog
                        .kind_id(kind)
                        .ok_or(ParserCstError::UnknownNodeKind { event: index })?;
                    if self.kind_catalog.kinds()[usize::from(kind_id)].category()
                        != ParserKindCategory::Node
                    {
                        return Err(ParserCstError::KindCategoryMismatch { event: index });
                    }
                    if nodes.is_empty() {
                        roots += 1;
                    }
                    builder.start_node(rowan::SyntaxKind(kind_id));
                    nodes.push((*id, kind.as_str()));
                }
                ParseEvent::FinishNode { id, kind, end } => {
                    if *end != offset
                        || fields
                            .last()
                            .is_some_and(|(_, owner_depth)| *owner_depth >= nodes.len())
                        || nodes.pop() != Some((*id, kind.as_str()))
                    {
                        return Err(ParserCstError::InvalidEventOrder { event: index });
                    }
                    builder.finish_node();
                }
                ParseEvent::StartField { field, start } => {
                    if nodes.is_empty() {
                        return Err(ParserCstError::InvalidEventOrder { event: index });
                    }
                    if *start != offset {
                        return Err(ParserCstError::InvalidOffset { event: index });
                    }
                    fields.push((field.as_str(), nodes.len()));
                }
                ParseEvent::FinishField { field, end } => {
                    if *end != offset
                        || nodes.is_empty()
                        || fields.pop() != Some((field.as_str(), nodes.len()))
                    {
                        return Err(ParserCstError::InvalidEventOrder { event: index });
                    }
                }
                ParseEvent::Token {
                    kind,
                    lexeme,
                    start,
                    end,
                    ..
                } => {
                    if nodes.is_empty() {
                        return Err(ParserCstError::InvalidEventOrder { event: index });
                    }
                    let length = u32::try_from(lexeme.len())
                        .map_err(|_| ParserCstError::InvalidOffset { event: index })?;
                    let expected_end = start
                        .checked_add(length)
                        .ok_or(ParserCstError::InvalidOffset { event: index })?;
                    if *start != offset || *end != expected_end {
                        return Err(ParserCstError::InvalidOffset { event: index });
                    }
                    let kind_id = self
                        .kind_catalog
                        .terminal_kind_id(kind)
                        .ok_or(ParserCstError::UnknownTokenKind { event: index })?;
                    builder.token(rowan::SyntaxKind(kind_id), lexeme);
                    offset = *end;
                }
            }
        }
        if !nodes.is_empty() || !fields.is_empty() || roots != 1 {
            return Err(ParserCstError::MissingRoot);
        }
        Ok(ParserCst {
            green: builder.finish(),
            catalog: Arc::clone(&self.kind_catalog),
        })
    }
}
