//! Bounded consumer qualification; no physical backend or real serving cache.
pub mod support;
use meta_relational_reasoning::{
    AgenticAiContextAdmissionError, AgenticAiContextError, AgenticAiContextMaterializationRequest,
    AgenticAiContextQuerySelectionRestoreRequest, AgenticAiContextRenderedElement,
    AgenticAiContextRevisionRequest, QueryResultAdmissionError,
    compare_agentic_ai_context_revision, restore_agentic_ai_context_query_selection,
    select_agentic_ai_context_from_query,
};
use serde::Serialize;
use std::{env, time::Instant};

#[derive(Serialize)]
struct ScaleReceipt {
    schema: &'static str,
    facts: usize,
    rows: usize,
    distinct_roots: usize,
    fanout: usize,
    iterations: usize,
    selected: usize,
    result_bytes: usize,
    source_ms: f64,
    selection_ms: f64,
    export_ms: f64,
    restore_ms: f64,
    revision_ms: f64,
    semantic_reusable: usize,
    rendered_bytes: usize,
    stable_token_prefix: usize,
    eligible_full_block_tokens: usize,
    budget_rejections: usize,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args()
        .skip(1)
        .map(|arg| arg.parse::<usize>())
        .collect::<Result<_, _>>()?;
    let [facts, rows, roots, fanout, iterations] = args.as_slice() else {
        return Err("expected facts rows distinct_roots fanout iterations".into());
    };
    let (facts, rows, roots, fanout, iterations) = (*facts, *rows, *roots, *fanout, *iterations);
    if facts < 2
        || rows < 2
        || roots == 0
        || roots > facts
        || fanout < 2
        || fanout >= facts
        || iterations == 0
    {
        return Err("invalid bounded scale dimensions".into());
    }
    let start = Instant::now();
    let source = support::source(&(1..=facts).map(|id| (id, 7)).collect::<Vec<_>>());
    let source_ms = start.elapsed().as_secs_f64() * 1000.0;
    println!("SCALE-OK: source admitted ({facts} facts)");
    let dependencies = vec![(1, (2..=fanout + 1).collect::<Vec<_>>())];
    let req = support::request(facts, rows + 1, &dependencies);
    let values: Vec<_> = (0..rows).map(|index| (index % roots + 1, 7)).collect();
    let candidate = support::candidate(&source, &values);
    let select = |request| {
        select_agentic_ai_context_from_query(
            &source.bundle,
            &source.snapshot,
            &source.query,
            &candidate,
            request,
        )
    };
    let mut selection_ms = 0.0;
    let mut export_ms = 0.0;
    let mut restore_ms = 0.0;
    let mut revision_ms = 0.0;
    let mut selected_count = 0;
    let mut result_bytes = 0;
    let mut reusable = 0;
    let mut rendered_bytes = 0;
    let mut token_prefix = 0;
    let mut eligible_tokens = 0;
    for iteration in 0..iterations {
        let start = Instant::now();
        let selected = select(req.clone())?;
        selection_ms += start.elapsed().as_secs_f64() * 1000.0;
        println!("SCALE-OK: iteration {iteration} selection");
        selected_count = selected.context().closure().elements().len();
        let start = Instant::now();
        let record = selected.export_record(
            &source.query,
            &candidate,
            req.result_limits,
            support::nz((rows + 1) * 1024 + 4096),
        )?;
        export_ms += start.elapsed().as_secs_f64() * 1000.0;
        result_bytes = record.result_transport.len();
        println!("SCALE-OK: iteration {iteration} export ({result_bytes} bytes)");
        let start = Instant::now();
        let restored = restore_agentic_ai_context_query_selection(
            &source.bundle,
            &source.snapshot,
            AgenticAiContextQuerySelectionRestoreRequest {
                record: &record,
                expected_digest: *selected.digest(),
                limits: req.limits,
                result_limits: req.result_limits,
                max_result_bytes: support::nz((rows + 1) * 1024 + 4096),
            },
        )?;
        restore_ms += start.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(selected, restored);
        println!("SCALE-OK: iteration {iteration} restore");
        let start = Instant::now();
        let revision = compare_agentic_ai_context_revision(AgenticAiContextRevisionRequest {
            old: selected.context(),
            new: restored.context(),
            old_bundle: &source.bundle,
            new_bundle: &source.bundle,
            old_snapshot: &source.snapshot,
            new_snapshot: &source.snapshot,
        })?;
        revision_ms += start.elapsed().as_secs_f64() * 1000.0;
        reusable = revision.revision().reusable().elements().len();
        assert_eq!(reusable, selected_count);
        // Identical semantic admission and deterministic renderer yield actual byte equality.
        let order = selected.context().closure().elements().to_vec();
        let materialize = |context: &meta_relational_reasoning::AdmittedAgenticAiContext| {
            context.materialize(
                &source.bundle,
                &source.snapshot,
                AgenticAiContextMaterializationRequest {
                    precedence: order.clone(),
                    renderer_identity: "scale-byte-renderer.v1".into(),
                    segments: order
                        .iter()
                        .rev()
                        .map(|id| AgenticAiContextRenderedElement {
                            element: *id,
                            bytes: vec![65],
                        })
                        .collect(),
                },
            )
        };
        let old = materialize(selected.context())?;
        let new = materialize(restored.context())?;
        rendered_bytes = new.materialization().bytes().len();
        assert_eq!(old.materialization().bytes(), new.materialization().bytes());
        // Full-prompt byte tokenizer, with actual declared tokens, not byte-prefix inference.
        let identity = meta_relational_reasoning::AgenticAiContextComputationalIdentity {
            model_weights: "scale-no-model".into(),
            tokenizer: "fixture-byte-tokenizer.v1".into(),
            renderer: "scale-byte-renderer.v1".into(),
            chat_template: "raw.v1".into(),
            position_scheme: "sequential.v1".into(),
            attention_semantics: "causal.v1".into(),
            cache_format: "blocks.v1".into(),
            adapter_digest: [0; 32],
            multimodal_digest: [0; 32],
            sharing_scope: [1; 32],
            block_tokens: support::nz(16),
        };
        let old = old.tokenize(
            &source.bundle,
            &source.snapshot,
            meta_relational_reasoning::AgenticAiContextTokenizationRequest {
                identity: identity.clone(),
                tokenizer: &support::ByteTokenizer,
            },
        )?;
        let new = new.tokenize(
            &source.bundle,
            &source.snapshot,
            meta_relational_reasoning::AgenticAiContextTokenizationRequest {
                identity,
                tokenizer: &support::ByteTokenizer,
            },
        )?;
        let eligibility = new.reuse_eligibility_from(&old, &source.bundle, &source.snapshot)?;
        token_prefix = eligibility.stable_token_prefix();
        eligible_tokens = eligibility.eligible_full_block_tokens();
        assert_eq!(token_prefix, rendered_bytes);
        println!(
            "SCALE-OK: iteration {iteration} revision/render/token ({reusable} reusable facts)"
        );
    }
    let mut row_budget = req.clone();
    row_budget.result_limits = meta_relational_reasoning::QueryResultLimits::new(
        support::nz(rows - 1),
        support::nz(rows - 1),
    );
    assert!(matches!(
        select(row_budget),
        Err(AgenticAiContextAdmissionError::QueryResult(
            QueryResultAdmissionError::RowLimitExceeded { .. }
        ))
    ));
    let mut source_budget = req.clone();
    source_budget.limits.max_elements = support::nz(facts - 1);
    assert!(matches!(
        select(source_budget),
        Err(AgenticAiContextAdmissionError::Context(
            AgenticAiContextError::ElementBudget
        ))
    ));
    let mut edge_budget = req.clone();
    edge_budget.limits.max_dependency_edges = support::nz(fanout - 1);
    assert!(matches!(
        select(edge_budget),
        Err(AgenticAiContextAdmissionError::Context(
            AgenticAiContextError::DependencyBudget
        ))
    ));
    let selected = select(req.clone())?;
    assert!(matches!(
        selected.export_record(&source.query, &candidate, req.result_limits, support::nz(1)),
        Err(AgenticAiContextAdmissionError::SelectionTransport(
            meta_relational_reasoning::QueryResultTransportError::TooLarge { .. }
        ))
    ));
    let mut byte_budget = req.clone();
    byte_budget.limits.max_rendered_bytes = support::nz(1);
    let bounded = select(byte_budget)?;
    let order = bounded.context().closure().elements().to_vec();
    assert!(matches!(
        bounded.context().materialize(
            &source.bundle,
            &source.snapshot,
            AgenticAiContextMaterializationRequest {
                precedence: order.clone(),
                renderer_identity: "scale-byte-renderer.v1".into(),
                segments: order
                    .iter()
                    .rev()
                    .map(|id| AgenticAiContextRenderedElement {
                        element: *id,
                        bytes: vec![65]
                    })
                    .collect()
            }
        ),
        Err(AgenticAiContextAdmissionError::Context(
            AgenticAiContextError::RenderedByteBudget
        ))
    ));
    println!("SCALE-OK: row/source/dependency/transport/render ceilings reject overflow");

    let receipt = ScaleReceipt {
        schema: "mrr.agentic-ai-context.scale-receipt.v1",
        facts,
        rows,
        distinct_roots: roots,
        fanout,
        iterations,
        selected: selected_count,
        result_bytes,
        source_ms,
        selection_ms: selection_ms / iterations as f64,
        export_ms: export_ms / iterations as f64,
        restore_ms: restore_ms / iterations as f64,
        revision_ms: revision_ms / iterations as f64,
        semantic_reusable: reusable,
        rendered_bytes,
        stable_token_prefix: token_prefix,
        eligible_full_block_tokens: eligible_tokens,
        budget_rejections: 5,
    };
    let encoded = serde_json::to_string(&receipt)?;
    if let Some(path) = env::var_os("MRR_CONTEXT_SCALE_RECEIPT") {
        std::fs::write(path, &encoded)?;
    }
    println!("{encoded}");
    Ok(())
}
