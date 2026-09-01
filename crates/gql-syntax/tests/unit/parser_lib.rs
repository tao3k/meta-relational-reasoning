//! Parser contracts split by grammar responsibility.

use crate::{SyntaxElementKind, SyntaxKind, TokenKind, parse};
use gql_source::Diagnostic;

fn contains_node_kind(node: &crate::SyntaxNode, expected: SyntaxKind) -> bool {
    node.kind() == expected
        || node
            .children()
            .into_iter()
            .any(|element| match element.kind {
                SyntaxElementKind::Node(child) => contains_node_kind(&child, expected),
                SyntaxElementKind::Token(_) => false,
            })
}

include!("parser_lib/source_and_rowan.rs");
include!("parser_lib/clauses.rs");
include!("parser_lib/expressions_and_patterns.rs");
include!("parser_lib/values_and_collections.rs");
