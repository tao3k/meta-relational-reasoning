#![forbid(unsafe_code)]

pub use gql_core::*;

#[cfg(feature = "ascent")]
pub mod reasoning {
    pub use gql_ascent::AscentTransitiveClosure;
}

#[cfg(test)]
mod tests {
    #[test]
    fn iso_parse_surface_is_feature_invariant() {
        let source = "MATCH (a)-[:CALLS]->(b) RETURN a, b";
        let parsed = crate::syntax::parse("feature-invariance.gql", source);
        assert!(parsed.diagnostics.is_empty());
        assert_eq!(parsed.tree.source().text(), source);
    }

    #[test]
    fn ascent_is_not_a_parser_keyword() {
        let parsed = crate::syntax::parse("purity.gql", "RETURN ascent");
        assert!(parsed.diagnostics.is_empty());
        assert!(
            parsed
                .tree
                .tokens()
                .iter()
                .any(|token| token.kind == crate::syntax::TokenKind::Identifier)
        );
    }
}
