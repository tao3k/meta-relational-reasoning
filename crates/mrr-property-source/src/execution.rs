//! Source compilation delegates physical dispatch to the MRR contract port.
use crate::BoundPropertySourceQuery;
use meta_relational_reasoning::{
    AdmittedPropertyExecution, PropertyExecutionError, PropertyQueryBackend, QueryResultLimits,
};

impl BoundPropertySourceQuery {
    /// Dispatch the original bound query; its parser receipt remains on this source value.
    pub async fn execute_with<B: PropertyQueryBackend>(
        &self,
        backend: &B,
        result_limits: QueryResultLimits,
    ) -> Result<AdmittedPropertyExecution<B::PhysicalEvidence>, PropertyExecutionError<B::Error>>
    {
        self.query().execute_with(backend, result_limits).await
    }
}
