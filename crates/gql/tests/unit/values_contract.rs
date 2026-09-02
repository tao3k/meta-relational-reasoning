use crate::Compiler;
use crate::ast::{Expression as AstExpression, QueryClause, Statement};
use crate::catalog::{Catalog, CatalogName};
use crate::ir::Expression as IrExpression;
use crate::syntax::TokenKind;

fn empty_catalog() -> Catalog {
    Catalog::new(
        CatalogName("values-contract".into()),
        Vec::new(),
        Vec::new(),
    )
}

#[test]
fn radix_integers_preserve_spelling_and_share_one_canonical_value() {
    let source = "MATCH (n) RETURN 0x2a, 0o52, 0b10_1010";
    let result = Compiler.compile("radix-integers.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert!(
        result.parse.diagnostics.is_empty(),
        "parse diagnostics: {:?}",
        result.parse.diagnostics
    );
    let numbers = result
        .parse
        .tree
        .tokens()
        .iter()
        .filter(|token| token.kind == TokenKind::Number)
        .map(|token| token.text())
        .collect::<Vec<_>>();
    assert_eq!(numbers, ["0x2a", "0o52", "0b10_1010"]);

    let Some(Statement::Query(query)) = &result.statement else {
        panic!("radix integer source must remain a query");
    };
    let Some(QueryClause::Return { projections, .. }) = query.clauses.get(1) else {
        panic!("RETURN clause exists");
    };
    assert_eq!(projections.len(), 3);
    for (projection, spelling) in projections.iter().zip(numbers) {
        let AstExpression::Integer(value, span) = &projection.expression else {
            panic!("radix literal must lower to an integer");
        };
        assert_eq!(*value, 42);
        assert_eq!(&source[span.start as usize..span.end as usize], spelling);
    }

    assert!(
        result.analysis.diagnostics.is_empty(),
        "semantic diagnostics: {:?}",
        result.analysis.diagnostics
    );
    let ir = result.analysis.ir.expect("canonical radix integer IR");
    assert_eq!(
        ir.projection
            .iter()
            .map(|projection| &projection.expression)
            .collect::<Vec<_>>(),
        [&IrExpression::Integer(42); 3]
    );
}

#[test]
fn invalid_radix_integer_has_one_typed_terminal_and_no_ir() {
    let source = "MATCH (n) RETURN 0b102";
    let result = Compiler.compile("invalid-radix-integer.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(result.parse.diagnostics.len(), 1);
    let diagnostic = &result.parse.diagnostics[0];
    assert_eq!(diagnostic.code, "GQL-SYNTAX-INVALID-NUMERIC-LITERAL");
    assert_eq!(
        &source[diagnostic.span.start as usize..diagnostic.span.end as usize],
        "0b102"
    );
    assert!(result.analysis.ir.is_none());
}

#[test]
fn out_of_range_integer_has_one_typed_terminal_and_no_ir() {
    let source = "MATCH (n) RETURN 0x8000000000000000";
    let result = Compiler.compile("out-of-range-integer.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(result.parse.diagnostics.len(), 1);
    let diagnostic = &result.parse.diagnostics[0];
    assert_eq!(diagnostic.code, "GQL-SYNTAX-NUMERIC-LITERAL-OUT-OF-RANGE");
    assert_eq!(
        &source[diagnostic.span.start as usize..diagnostic.span.end as usize],
        "0x8000000000000000"
    );
    assert!(result.analysis.ir.is_none());
}

#[test]
fn out_of_range_decimal_integer_has_one_typed_terminal_and_no_ir() {
    let source = "MATCH (n) RETURN 9223372036854775808";
    let result = Compiler.compile("out-of-range-decimal-integer.gql", source, &empty_catalog());

    assert_eq!(result.parse.tree.rowan_root().text().to_string(), source);
    assert_eq!(result.parse.diagnostics.len(), 1);
    let diagnostic = &result.parse.diagnostics[0];
    assert_eq!(diagnostic.code, "GQL-SYNTAX-NUMERIC-LITERAL-OUT-OF-RANGE");
    assert_eq!(
        &source[diagnostic.span.start as usize..diagnostic.span.end as usize],
        "9223372036854775808"
    );
    assert!(result.analysis.ir.is_none());
}
