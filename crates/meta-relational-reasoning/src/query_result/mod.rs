//! Typed query-result admission and bounded Scheme transport.
mod api;
mod scheme;

pub use api::{
    CandidateQueryResult, QueryResultAdmissionError, QueryResultAdmissionReceipt,
    QueryResultBinding, QueryResultLimits, QueryResultTransportError, QueryResultValue,
    QueryResultValueKind, VerifiedQueryResultTransport, admit_query_result_candidate,
    export_query_result_transport, verify_query_result_transport,
};
