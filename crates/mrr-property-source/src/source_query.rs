//! Source-bound property query authority for external physical executors.
//!
//! The caller supplies catalogs and a semantic snapshot. Physical storage,
//! transfer, and query execution remain outside MRR.

use std::fmt;

use mrr_frontends::{
    ParserLanguage, ParserOwnedCompilation, ParserOwnedCompilationReceipt, QueryFrontend,
};

use meta_relational_reasoning::{
    BundleError, CandidateQueryResult, CatalogBoundQuery, EntityCatalog, MetaQueryIr,
    QueryCatalogBindingError, QueryResultAdmissionError, QueryResultAdmissionReceipt,
    QueryResultLimits, QueryTemplate, ReasoningBundle, ReasoningBundleDeclaration, RelationCatalog,
    SemanticSnapshot, admit_query_result_candidate, bind_query_to_catalog,
};

/// A parser-owned source compiled before any physical snapshot is read.
#[derive(Debug)]
pub struct CompiledPropertySourceQuery(ParserOwnedCompilation);

/// One original source bound to the caller's admitted catalogs and generation.
#[derive(Debug)]
pub struct BoundPropertySourceQuery {
    compilation: ParserOwnedCompilationReceipt,
    query: CatalogBoundQuery,
}

/// Rejection at the MRR-owned source, catalog, or candidate boundary.
#[derive(Debug)]
pub enum PropertySourceQueryError {
    Parser(String),
    SourceDigestMismatch,
    Bundle(BundleError),
    Binding(QueryCatalogBindingError),
    Admission(QueryResultAdmissionError),
}

impl fmt::Display for PropertySourceQueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parser(error) => write!(formatter, "parser-owned GQL compilation: {error}"),
            Self::SourceDigestMismatch => write!(formatter, "original GQL source digest mismatch"),
            Self::Bundle(error) => write!(formatter, "MRR query bundle admission: {error}"),
            Self::Binding(error) => write!(formatter, "MRR catalog binding: {error}"),
            Self::Admission(error) => write!(formatter, "MRR result admission: {error}"),
        }
    }
}

impl std::error::Error for PropertySourceQueryError {}

/// Compile the exact original GQL source and reject source drift before I/O.
///
/// # Errors
/// Rejects parser diagnostics or a mismatched caller-owned source digest.
pub fn compile_property_source_query(
    source_name: &str,
    source_text: &str,
    expected_source_digest: &str,
) -> Result<CompiledPropertySourceQuery, PropertySourceQueryError> {
    let compilation = QueryFrontend::new(ParserLanguage::Gql)
        .compile_with_receipt(source_name, source_text)
        .map_err(|error| PropertySourceQueryError::Parser(format!("{error:?}")))?;
    if compilation.receipt.source_digest != expected_source_digest {
        return Err(PropertySourceQueryError::SourceDigestMismatch);
    }
    Ok(CompiledPropertySourceQuery(compilation))
}

impl CompiledPropertySourceQuery {
    /// Parsed source query, before binding to a semantic snapshot.
    #[must_use]
    pub const fn query(&self) -> &MetaQueryIr {
        &self.0.query
    }

    /// Parser-owned source and grammar receipt.
    #[must_use]
    pub const fn compilation(&self) -> &ParserOwnedCompilationReceipt {
        &self.0.receipt
    }

    /// Bind the query to the exact semantic snapshot and admitted catalogs.
    ///
    /// # Errors
    /// Rejects invalid bundle declarations or mismatched catalog identities.
    pub fn bind(
        self,
        relation_catalog: &RelationCatalog,
        entity_catalog: &EntityCatalog,
        semantic_snapshot: &SemanticSnapshot,
    ) -> Result<BoundPropertySourceQuery, PropertySourceQueryError> {
        let ParserOwnedCompilation { query, receipt } = self.0;
        let query_id = query.id();
        let bundle = ReasoningBundle::admit(ReasoningBundleDeclaration {
            entities: entity_catalog.entities().to_vec(),
            relations: relation_catalog.relations().to_vec(),
            query_templates: vec![QueryTemplate::new(query, vec![])],
            ..ReasoningBundleDeclaration::default()
        })
        .map_err(PropertySourceQueryError::Bundle)?;
        let query = bind_query_to_catalog(&bundle, query_id, semantic_snapshot)
            .map_err(PropertySourceQueryError::Binding)?;
        Ok(BoundPropertySourceQuery {
            compilation: receipt,
            query,
        })
    }
}

impl BoundPropertySourceQuery {
    /// The exact MRR query a caller may submit to its physical library.
    #[must_use]
    pub const fn query(&self) -> &CatalogBoundQuery {
        &self.query
    }

    /// Parser-owned evidence for the original source and grammar.
    #[must_use]
    pub const fn compilation(&self) -> &ParserOwnedCompilationReceipt {
        &self.compilation
    }

    /// Admit the backend's candidate against this exact bound query.
    ///
    /// # Errors
    /// Rejects identity, shape, type, or resource-limit mismatches.
    pub fn admit(
        &self,
        candidate: &CandidateQueryResult,
        limits: QueryResultLimits,
    ) -> Result<QueryResultAdmissionReceipt, PropertySourceQueryError> {
        admit_query_result_candidate(&self.query, candidate, limits)
            .map_err(PropertySourceQueryError::Admission)
    }
}
