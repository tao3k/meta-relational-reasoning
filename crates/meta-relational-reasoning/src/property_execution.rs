//! MRR dispatches a bound query; an external backend owns physical execution.

use std::{error::Error, fmt, future::Future};

use crate::{
    CandidateQueryResult, CatalogBoundQuery, QueryResultAdmissionError,
    QueryResultAdmissionReceipt, QueryResultLimits, admit_query_result_candidate,
};

/// Backend-owned physical evidence is retained without becoming semantic authority.
pub struct PropertyExecutionCandidate<E> {
    pub candidate: CandidateQueryResult,
    pub physical_evidence: E,
}

/// Implemented by MRR Data's backend adapter, without importing MRR Data into MRR.
/// Backend configuration, storage protocols and execution limits belong to the
/// implementer. The exact admitted query is the only semantic execution input.
pub trait PropertyQueryBackend {
    type PhysicalEvidence: Send;
    type Error: Error + Send + Sync + 'static;

    fn execute<'a>(
        &'a self,
        query: &'a CatalogBoundQuery,
    ) -> impl Future<
        Output = Result<PropertyExecutionCandidate<Self::PhysicalEvidence>, Self::Error>,
    > + Send
    + 'a;
}

/// A backend failure never becomes an empty admitted result.
#[derive(Debug)]
pub enum PropertyExecutionError<E> {
    Backend(E),
    Admission(QueryResultAdmissionError),
}

impl<E: Error> fmt::Display for PropertyExecutionError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(error) => write!(f, "physical property execution: {error}"),
            Self::Admission(error) => write!(f, "{error}"),
        }
    }
}
impl<E: Error + 'static> Error for PropertyExecutionError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Backend(error) => Some(error),
            Self::Admission(error) => Some(error),
        }
    }
}

/// Only MRR's checked dispatch path constructs an admitted execution.
pub struct AdmittedPropertyExecution<E> {
    candidate: CandidateQueryResult,
    receipt: QueryResultAdmissionReceipt,
    physical_evidence: E,
}

impl<E> AdmittedPropertyExecution<E> {
    pub const fn candidate(&self) -> &CandidateQueryResult {
        &self.candidate
    }
    pub const fn receipt(&self) -> &QueryResultAdmissionReceipt {
        &self.receipt
    }
    pub const fn physical_evidence(&self) -> &E {
        &self.physical_evidence
    }
}

impl CatalogBoundQuery {
    /// Dispatch to the backend and admit its original candidate against this
    /// exact query. Parsing, identity allocation and result admission stay in MRR.
    pub async fn execute_with<B: PropertyQueryBackend>(
        &self,
        backend: &B,
        result_limits: QueryResultLimits,
    ) -> Result<AdmittedPropertyExecution<B::PhysicalEvidence>, PropertyExecutionError<B::Error>>
    {
        let output = backend
            .execute(self)
            .await
            .map_err(PropertyExecutionError::Backend)?;
        let receipt = admit_query_result_candidate(self, &output.candidate, result_limits)
            .map_err(PropertyExecutionError::Admission)?;
        Ok(AdmittedPropertyExecution {
            candidate: output.candidate,
            receipt,
            physical_evidence: output.physical_evidence,
        })
    }
}
