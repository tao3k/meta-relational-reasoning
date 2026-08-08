//! Conversion from crate-local syntax nodes to a `rowan::GreenNode`.

use gql_source::SourceText;

use crate::syntax::{SyntaxElementKind, SyntaxNode, Token};

/// Builds a `rowan` root node for downstream syntax consumers.
pub fn build_rowan_root(root: &SyntaxNode, source: &SourceText) -> rowan::GreenNode {
    let mut builder = rowan::GreenNodeBuilder::new();
    build_node(root, &mut builder, source);
    builder.finish()
}

fn build_node(node: &SyntaxNode, builder: &mut rowan::GreenNodeBuilder, source: &SourceText) {
    builder.start_node(node.kind().to_rowan());
    for child in node.children() {
        match &child.kind {
            SyntaxElementKind::Node(inner) => {
                build_node(inner, builder, source);
            }
            SyntaxElementKind::Token(token) => {
                builder.token(token.syntax_kind().to_rowan(), token_text(token, source));
            }
        }
    }
    builder.finish_node();
}

fn token_text<'a>(token: &Token, source: &'a SourceText) -> &'a str {
    source.slice(token.span).unwrap_or("")
}
