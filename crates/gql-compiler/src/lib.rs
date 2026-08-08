#![forbid(unsafe_code)]

pub use gql_sema::{Analysis, analyze};
pub use gql_syntax::{Parse, parse};

use gql_ast::Statement;
use gql_catalog::GqlCatalog;

#[derive(Clone, Copy, Debug, Default)]
pub struct Compiler;

impl Compiler {
    #[must_use]
    pub fn parse(&self, name: &str, source: &str) -> Parse {
        parse(name, source)
    }

    #[must_use]
    pub fn analyze(&self, statement: &Statement, catalog: &dyn GqlCatalog) -> Analysis {
        analyze(statement, catalog)
    }
}
