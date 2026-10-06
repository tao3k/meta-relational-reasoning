//! Source compilation delegates physical dispatch to the MRR contract port.
use crate::{BoundPropertySourceQuery, ParserOwnedCompilationReceipt};
use meta_relational_reasoning::{
    AdmittedPropertyExecution, CandidateQueryResult, CatalogBoundQuery, PropertyExecutionError,
    PropertyQueryBackend, QueryResultAdmissionReceipt, QueryResultLimits,
    QueryResultTransportError, export_query_result_transport,
};
use std::num::NonZeroUsize;

/// Original parser source, semantic query and backend evidence retained together.
/// Construction requires the checked dispatch path. This does not authenticate
/// a provider or authorize an external effect.
pub struct AdmittedPropertySourceExecution<E> {
    source: BoundPropertySourceQuery,
    execution: AdmittedPropertyExecution<E>,
    limits: QueryResultLimits,
}

impl<E> AdmittedPropertySourceExecution<E> {
    /// Original parser receipt; never reconstructed from result bytes.
    pub const fn compilation(&self) -> &ParserOwnedCompilationReceipt {
        self.source.compilation()
    }
    /// Original query with its admitted catalog, generation and snapshot binding.
    pub const fn query(&self) -> &CatalogBoundQuery {
        self.source.query()
    }
    /// Checked lower-level dispatch for physical adapter projections.
    pub const fn execution(&self) -> &AdmittedPropertyExecution<E> {
        &self.execution
    }
    /// Original admitted candidate rows.
    pub const fn candidate(&self) -> &CandidateQueryResult {
        self.execution.candidate()
    }
    /// Original MRR result admission receipt.
    pub const fn receipt(&self) -> &QueryResultAdmissionReceipt {
        self.execution.receipt()
    }
    /// Backend-owned root and engine evidence, retained without relabeling.
    pub const fn physical_evidence(&self) -> &E {
        self.execution.physical_evidence()
    }
    /// Export bounded Scheme v1 rows using the retained query and original limits.
    /// # Errors
    /// Rejects transport overflow or failed receiver-independent result admission.
    pub fn export_result_transport(
        &self,
        max_bytes: NonZeroUsize,
    ) -> Result<Vec<u8>, QueryResultTransportError> {
        export_query_result_transport(self.query(), self.candidate(), self.limits, max_bytes)
    }
}

impl BoundPropertySourceQuery {
    /// Dispatch the original bound query; its parser receipt remains on this source value.
    pub async fn execute_with<B: PropertyQueryBackend>(
        &self,
        backend: &B,
        result_limits: QueryResultLimits,
    ) -> Result<
        AdmittedPropertySourceExecution<B::PhysicalEvidence>,
        PropertyExecutionError<B::Error>,
    > {
        let execution = self.query().execute_with(backend, result_limits).await?;
        Ok(AdmittedPropertySourceExecution {
            source: self.clone(),
            execution,
            limits: result_limits,
        })
    }
}
