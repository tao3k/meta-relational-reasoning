//! Fixed-field process protocol for the live closure resource.

use crate::{ClosureToolInput, run_closure_tool};

/// Execute a normalized closure command and return its line-oriented receipt.
pub fn run_closure_tool_cli(values: &[String]) -> Result<String, String> {
    if values.len() < 4 || !values.len().is_multiple_of(2) {
        return Err("usage: mrr-closure-tool SOURCE TARGET FROM TO [FROM TO ...]".into());
    }
    let input = ClosureToolInput {
        source: values[0].clone(),
        target: values[1].clone(),
        edges: values[2..]
            .chunks_exact(2)
            .map(|edge| (edge[0].clone(), edge[1].clone()))
            .collect(),
    };
    let receipt = run_closure_tool(&input).map_err(|error| error.to_string())?;
    Ok(format!(
        "schema=mrr.closure-tool-receipt.v1\nstatus=admitted\nreachable={}\ncandidate_count={}\nclosure_status={:?}",
        receipt.reachable,
        receipt.closure.receipt().derived_fact_ids().len(),
        receipt.closure.receipt().closure_status(),
    ))
}
