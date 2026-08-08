#![forbid(unsafe_code)]

use gql_ast::{QueryClause, Statement};
use gql_catalog::{GqlCatalog, RelationName};
use gql_ir::{QueryBlock, RelationScan};
use gql_source::Diagnostic;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Analysis {
    pub ir: Option<QueryBlock>,
    pub diagnostics: Vec<Diagnostic>,
}

#[must_use]
pub fn analyze(statement: &Statement, catalog: &dyn GqlCatalog) -> Analysis {
    let Statement::Query(query) = statement else {
        return Analysis {
            ir: None,
            diagnostics: vec![Diagnostic::error(
                "GQL-SEMA-NOT-YET-LOWERED",
                "catalog and data statements are not lowered by this foundation release",
                gql_source::Span::default(),
            )],
        };
    };
    let mut block = QueryBlock::default();
    let mut diagnostics = Vec::new();
    for clause in &query.clauses {
        if let QueryClause::Match { relation } = clause {
            let name = RelationName(relation.text.clone());
            if catalog.relation(&name).is_some() {
                block.scans.push(RelationScan {
                    relation: name,
                    bindings: Vec::new(),
                });
            } else {
                diagnostics.push(Diagnostic::error(
                    "GQL-SEMA-UNKNOWN-RELATION",
                    format!("unknown relation `{}`", relation.text),
                    relation.span,
                ));
            }
        }
    }
    Analysis {
        ir: diagnostics.is_empty().then_some(block),
        diagnostics,
    }
}
