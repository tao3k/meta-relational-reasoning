use crate::Compiler;
use crate::catalog::{Catalog, CatalogName};
use crate::syntax::TokenKind;

fn empty_catalog() -> Catalog {
    Catalog::new(
        CatalogName("trivia-contract".into()),
        Vec::new(),
        Vec::new(),
    )
}

#[test]
fn line_and_block_comments_are_lossless_and_semantically_transparent() {
    let source = "// leading\nMATCH (n) /* inline */ RETURN n -- trailing";
    let result = Compiler.compile("comments.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse diagnostics: {:?}",
        result.parse.diagnostics
    );
    let comments = result
        .parse
        .tree
        .tokens()
        .iter()
        .filter(|token| token.kind == TokenKind::Comment)
        .map(|token| token.text())
        .collect::<Vec<_>>();
    assert_eq!(
        comments,
        ["// leading", "/* inline */", "-- trailing"],
        "comments must remain distinct lossless CST tokens in source order"
    );
    assert!(
        result.analysis.diagnostics.is_empty(),
        "semantic diagnostics: {:?}",
        result.analysis.diagnostics
    );

    let plain = Compiler.compile("comments-plain.gql", "MATCH (n) RETURN n", &empty_catalog());
    assert_eq!(
        result.analysis.ir, plain.analysis.ir,
        "trivia must not affect canonical IR"
    );
}

#[test]
fn unterminated_block_comment_has_one_typed_terminal_and_no_ir() {
    let source = "MATCH (n) RETURN n /* unterminated";
    let result = Compiler.compile("unterminated-comment.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(result.parse.diagnostics.len(), 1);
    let diagnostic = &result.parse.diagnostics[0];
    assert_eq!(diagnostic.code, "GQL-SYNTAX-UNTERMINATED-BLOCK-COMMENT");
    assert_eq!(
        &source[diagnostic.span.start as usize..diagnostic.span.end as usize],
        "/* unterminated"
    );
    assert!(result.analysis.ir.is_none());
}
