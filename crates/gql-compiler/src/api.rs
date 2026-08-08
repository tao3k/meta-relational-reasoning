//! Compiler facade implementation for parse/lower/analyze/compile steps.

pub use gql_sema::{Analysis, analyze};
pub use gql_syntax::{Parse, parse};

use gql_ast::{lower_from_syntax, Statement};
use gql_catalog::GqlCatalog;

#[derive(Clone, Copy, Debug, Default)]
/// Orchestrator type for GQL compilation.
pub struct Compiler;

#[derive(Clone, Debug, Eq, PartialEq)]
/// Full compile output including parse and analysis artifacts.
pub struct Compilation {
    /// Original parse result.
    pub parse: Parse,
    /// Lowered syntax statement.
    pub statement: Statement,
    /// Analysis result and diagnostics.
    pub analysis: Analysis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Parse/lower output including statement and diagnostics.
pub struct ParserOutput {
    /// Lowered statement.
    pub statement: Statement,
    /// Diagnostic output from parsing/lowering.
    pub diagnostics: Vec<gql_source::Diagnostic>,
}

impl Compiler {
    /// Parse source text into a syntax parse result.
    #[must_use]
    pub fn parse(&self, name: &str, source: &str) -> Parse {
        parse(name, source)
    }

    /// Analyze a lowered statement against a catalog.
    #[must_use]
    pub fn analyze(&self, statement: &Statement, catalog: &dyn GqlCatalog) -> Analysis {
        analyze(statement, catalog)
    }

    /// Lower a source string to parser output.
    #[must_use]
    pub fn lower(&self, name: &str, source: &str) -> ParserOutput {
        let parse = self.parse(name, source);
        let lowered = lower_from_syntax(&parse);
        ParserOutput {
            statement: lowered.statement,
            diagnostics: lowered.diagnostics,
        }
    }

    /// Compile a query into parse, diagnostics, and analysis state.
    #[must_use]
    pub fn compile(&self, name: &str, source: &str, catalog: &dyn GqlCatalog) -> Compilation {
        let parse = self.parse(name, source);
        let parser_output = {
            let lowered = lower_from_syntax(&parse);
            ParserOutput {
                statement: lowered.statement,
                diagnostics: lowered.diagnostics,
            }
        };

        let mut analysis = analyze(&parser_output.statement, catalog);
        let mut merged_diagnostics = parse
            .diagnostics
            .clone()
            .into_iter()
            .chain(parser_output.diagnostics)
            .collect::<Vec<_>>();
        merged_diagnostics.append(&mut analysis.diagnostics);
        analysis.diagnostics = merged_diagnostics;
        if !analysis.diagnostics.is_empty() {
            analysis.ir = None;
        }

        Compilation {
            parse,
            statement: parser_output.statement,
            analysis,
        }
    }
}
