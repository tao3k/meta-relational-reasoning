//! Deterministic consumer fixture, shared by runnable examples and integration checks.
use std::num::NonZeroUsize;

use meta_relational_reasoning::{
    AgenticAiContextContract, AgenticAiContextLimits, AgenticAiContextQuerySelectionRequest,
    Binding, CandidateQueryResult, CatalogBoundQuery, Direction, EntityId, EvidenceCompleteness,
    Expression, ExternalRevisionIdentity, Fact, FactId, FactProvenance, FactValidity, GenerationId,
    GraphPattern, MetaQueryIr, NodePattern, PathPattern, PathSegment, Projection, QueryId,
    QueryOperatorId, QueryResult, QueryResultBinding, QueryResultLimits, QueryResultValue,
    QueryTemplate, ReasoningBundle, ReasoningBundleDeclaration, RelationAuthority, RelationContext,
    RelationField, RelationId, RelationPattern, RelationSchema, RevisionBinding, SemanticSnapshot,
    SetQuantifier, StateId, Value, ValueSchema, bind_query_to_catalog,
};
use serde::Deserialize;

pub const FIXTURE: &str = include_str!("../../../../fixtures/agentic-ai-context/selection.json");

#[derive(Deserialize)]
pub struct SelectionFixture {
    pub schema: String,
    pub facts: Vec<(usize, usize)>,
    pub current_binding: Vec<usize>,
    pub cases: Vec<SelectionCase>,
    pub workflow: Workflow,
}
#[derive(Deserialize)]
pub struct SelectionCase {
    pub name: String,
    pub binding: Vec<usize>,
    pub rows: Vec<(usize, usize)>,
    pub max_rows: usize,
    pub max_facts: usize,
    pub accepted: bool,
    pub roots: Vec<usize>,
}
#[derive(Deserialize)]
pub struct Workflow {
    pub old_rows: Vec<(usize, usize)>,
    pub new_rows: Vec<(usize, usize)>,
    pub dependencies: Vec<(usize, Vec<usize>)>,
    pub old_precedence: Vec<usize>,
    pub new_precedence: Vec<usize>,
    pub old_bytes: Vec<u8>,
    pub new_bytes: Vec<u8>,
    pub stable_prefix: usize,
    pub block_tokens: usize,
    pub eligible_tokens: usize,
}

pub fn fact_id(index: usize) -> FactId {
    FactId::from_canonical_bytes(format!("context-workflow-fact-{index}")).unwrap()
}
pub fn relation_id(index: usize) -> RelationId {
    RelationId::from_canonical_bytes(format!("context-workflow-relation-{index}")).unwrap()
}
pub fn nz(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}

pub struct Source {
    pub bundle: ReasoningBundle,
    pub snapshot: SemanticSnapshot,
    pub query: CatalogBoundQuery,
}

pub fn schema(index: usize) -> RelationSchema {
    RelationSchema::new(
        relation_id(index),
        "depends",
        vec![
            RelationField::new("subject", ValueSchema::Entity, false).unwrap(),
            RelationField::new("object", ValueSchema::Entity, false).unwrap(),
        ],
        vec![],
    )
    .unwrap()
}

pub fn snapshot(generation: GenerationId, revision: &str) -> SemanticSnapshot {
    SemanticSnapshot::admit(
        generation,
        vec![
            RevisionBinding::admit(
                ExternalRevisionIdentity::new("fixture", "context-workflow", revision).unwrap(),
                generation,
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

pub fn source(facts: &[(usize, usize)]) -> Source {
    let generation = GenerationId::from_canonical_bytes("context-workflow-generation").unwrap();
    let entity = EntityId::from_canonical_bytes("context-workflow-entity").unwrap();
    let query_id = QueryId::from_canonical_bytes("context-workflow-query").unwrap();
    let binding = |s| Binding::new(s).unwrap();
    let query = MetaQueryIr::new(
        query_id,
        GraphPattern::new(
            QueryOperatorId::from_canonical_bytes("context-workflow-graph").unwrap(),
            vec![PathPattern::new(
                NodePattern::new(binding("source"), vec![]),
                vec![PathSegment::new(
                    RelationPattern::new(
                        Some(binding("edge")),
                        vec![relation_id(7)],
                        Direction::Outgoing,
                        1,
                        Some(1),
                    )
                    .unwrap(),
                    NodePattern::new(binding("target"), vec![]),
                )],
            )],
        )
        .unwrap(),
        vec![],
        QueryResult::returning(SetQuantifier::All).with_projections(vec![Projection::new(
            QueryOperatorId::from_canonical_bytes("context-workflow-projection").unwrap(),
            Expression::Binding(binding("edge")),
            binding("evidence"),
        )]),
    )
    .unwrap();
    let bundle = ReasoningBundle::admit(ReasoningBundleDeclaration {
        relations: vec![schema(7)],
        facts: facts
            .iter()
            .map(|(id, relation)| {
                Fact::new(
                    fact_id(*id),
                    relation_id(*relation),
                    vec![Value::Entity(entity), Value::Entity(entity)],
                    RelationContext::new(
                        generation,
                        RelationAuthority::Entity(entity),
                        FactProvenance::Source(entity),
                        EvidenceCompleteness::Complete,
                        FactValidity::Valid,
                    )
                    .unwrap(),
                )
            })
            .collect(),
        query_templates: vec![QueryTemplate::new(query, vec![])],
        ..ReasoningBundleDeclaration::default()
    })
    .unwrap();
    let snapshot = snapshot(generation, "one");
    let query = bind_query_to_catalog(&bundle, query_id, &snapshot).unwrap();
    Source {
        bundle,
        snapshot,
        query,
    }
}

pub fn candidate(source: &Source, rows: &[(usize, usize)]) -> CandidateQueryResult {
    CandidateQueryResult::new(
        QueryResultBinding::for_query(&source.query),
        vec![Binding::new("evidence").unwrap()],
        rows.iter()
            .map(|(id, relation)| {
                vec![QueryResultValue::relation(
                    fact_id(*id),
                    relation_id(*relation),
                )]
            })
            .collect(),
    )
}

pub fn request(
    elements: usize,
    rows: usize,
    dependencies: &[(usize, Vec<usize>)],
) -> AgenticAiContextQuerySelectionRequest {
    AgenticAiContextQuerySelectionRequest {
        column: Binding::new("evidence").unwrap(),
        contract: AgenticAiContextContract {
            actor: EntityId::from_canonical_bytes("context-workflow-actor").unwrap(),
            task: StateId::from_canonical_bytes("context-workflow-task").unwrap(),
            policy_digest: [1; 32],
            required: vec![],
            temporal_receipts: vec![],
            require_complete: true,
        },
        dependencies: dependencies
            .iter()
            .map(|(id, parents)| {
                (
                    fact_id(*id),
                    parents.iter().map(|id| fact_id(*id)).collect(),
                )
            })
            .collect(),
        limits: AgenticAiContextLimits {
            max_elements: nz(elements),
            max_dependency_edges: nz(elements.saturating_mul(4)),
            max_rendered_bytes: nz(elements.saturating_mul(8)),
        },
        result_limits: QueryResultLimits::new(nz(rows), nz(rows)),
    }
}

use meta_relational_reasoning::{
    AdmittedAgenticAiContextQuerySelection, AgenticAiContextComposedMaterializationRequest,
    AgenticAiContextComposer, AgenticAiContextCompositionGraph,
    AgenticAiContextCompositionGraphInput, AgenticAiContextCompositionNode,
    AgenticAiContextCompositionProducer, AgenticAiContextCompositionRequest,
    AgenticAiContextComputationalIdentity, AgenticAiContextExactSelectionRestoreRequest,
    AgenticAiContextRenderedElement, AgenticAiContextRevisionRequest,
    AgenticAiContextTokenizationRequest, AgenticAiContextTokenizer,
    SourceBoundAgenticAiContextTokens, compare_agentic_ai_context_revision,
    restore_agentic_ai_context_query_selection_exact, select_agentic_ai_context_from_query_exact,
};
use serde::Serialize;

/// Consumer adapter returning the shared precedence fixture checked by Lean POO.
/// This is not a new C4 algorithm or an authenticated producer implementation.
struct FixtureComposer {
    identity: AgenticAiContextCompositionProducer,
    precedence: Vec<FactId>,
}
impl AgenticAiContextComposer for FixtureComposer {
    type Error = std::io::Error;
    fn identity(&self) -> &AgenticAiContextCompositionProducer {
        &self.identity
    }
    fn compose(
        &self,
        _graph: &AgenticAiContextCompositionGraph,
    ) -> Result<Vec<FactId>, Self::Error> {
        Ok(self.precedence.clone())
    }
}
pub struct ByteTokenizer;
impl AgenticAiContextTokenizer for ByteTokenizer {
    type Error = std::io::Error;
    fn identity(&self) -> &str {
        "fixture-byte-tokenizer.v1"
    }
    fn tokenize(&self, bytes: &[u8]) -> Result<Vec<u32>, Self::Error> {
        Ok(bytes.iter().map(|byte| u32::from(*byte)).collect())
    }
}

pub fn present(
    source: &Source,
    selected: &AdmittedAgenticAiContextQuerySelection,
    order: &[usize],
    block: usize,
) -> Result<SourceBoundAgenticAiContextTokens, Box<dyn std::error::Error>> {
    let identity = AgenticAiContextCompositionProducer {
        owner: "shared-LeanPoo-fixture".into(),
        algorithm: "checked-C4-fixture".into(),
        implementation_version: "cb4623980056fb761f34601181bb7d8485af1dda".into(),
    };
    let producer = FixtureComposer {
        identity: identity.clone(),
        precedence: order.iter().map(|id| fact_id(*id)).collect(),
    };
    let graph = AgenticAiContextCompositionGraphInput {
        root: order.first().map(|id| fact_id(*id)),
        nodes: order
            .iter()
            .enumerate()
            .map(|(index, id)| AgenticAiContextCompositionNode {
                element: fact_id(*id),
                parent_orders: order
                    .get(index + 1)
                    .map(|parent| vec![vec![fact_id(*parent)]])
                    .unwrap_or_default(),
                suffix: false,
            })
            .collect(),
    };
    let receipt = selected.context().compose(
        &source.bundle,
        &source.snapshot,
        AgenticAiContextCompositionRequest {
            graph,
            expected_producer: identity,
            composer: &producer,
        },
    )?;
    let materialized = selected.context().materialize_composed(
        &source.bundle,
        &source.snapshot,
        AgenticAiContextComposedMaterializationRequest {
            composition: &receipt,
            renderer_identity: "fixture-segment-renderer.v1".into(),
            segments: order
                .iter()
                .rev()
                .map(|id| AgenticAiContextRenderedElement {
                    element: fact_id(*id),
                    bytes: vec![u8::try_from(64 + id).unwrap()],
                })
                .collect(),
        },
    )?;
    Ok(materialized.tokenize(
        &source.bundle,
        &source.snapshot,
        AgenticAiContextTokenizationRequest {
            identity: AgenticAiContextComputationalIdentity {
                model_weights: "fixture-no-model".into(),
                tokenizer: "fixture-byte-tokenizer.v1".into(),
                renderer: "fixture-segment-renderer.v1".into(),
                chat_template: "fixture-raw.v1".into(),
                position_scheme: "fixture-sequential.v1".into(),
                attention_semantics: "fixture-causal.v1".into(),
                cache_format: "fixture-blocks.v1".into(),
                adapter_digest: [0; 32],
                multimodal_digest: [0; 32],
                sharing_scope: [1; 32],
                block_tokens: nz(block),
            },
            tokenizer: &ByteTokenizer,
        },
    )?)
}

#[derive(Serialize)]
pub struct WorkflowReceipt {
    pub schema: &'static str,
    pub old_selected: usize,
    pub new_selected: usize,
    pub restored: bool,
    pub semantic_reusable: usize,
    pub stable_token_prefix: usize,
    pub eligible_full_block_tokens: usize,
    pub old_bytes: Vec<u8>,
    pub new_bytes: Vec<u8>,
}

pub fn workflow() -> Result<WorkflowReceipt, Box<dyn std::error::Error>> {
    let fixture: SelectionFixture = serde_json::from_str(FIXTURE)?;
    let source = source(&fixture.facts);
    let run = |rows: &[(usize, usize)]| {
        select_agentic_ai_context_from_query_exact(
            &source.bundle,
            &source.snapshot,
            &source.query,
            &candidate(&source, rows),
            request(8, 8, &fixture.workflow.dependencies),
        )
    };
    let old_exact = run(&fixture.workflow.old_rows)?;
    println!("WORKFLOW-OK: query admission and old required closure");
    let new_exact = run(&fixture.workflow.new_rows)?;
    let record = new_exact.export_record(request(8, 8, &[]).result_limits, nz(64 * 1024))?;
    let restored_exact = restore_agentic_ai_context_query_selection_exact(
        &source.bundle,
        &source.snapshot,
        AgenticAiContextExactSelectionRestoreRequest {
            record: &record,
            expected: &new_exact,
            limits: request(8, 8, &[]).limits,
            result_limits: request(8, 8, &[]).result_limits,
            max_result_bytes: nz(64 * 1024),
        },
    )?;
    assert_eq!(restored_exact, new_exact);
    let old = old_exact.selection();
    let new = new_exact.selection();
    let restored = restored_exact.selection();
    println!("WORKFLOW-OK: exact trusted source/query/result reference restored");
    let old_tokens = present(
        &source,
        old,
        &fixture.workflow.old_precedence,
        fixture.workflow.block_tokens,
    )?;
    let new_tokens = present(
        &source,
        restored,
        &fixture.workflow.new_precedence,
        fixture.workflow.block_tokens,
    )?;
    let old_bytes = old_tokens.presentation().materialization().bytes().to_vec();
    let new_bytes = new_tokens.presentation().materialization().bytes().to_vec();
    assert_eq!(old_bytes, fixture.workflow.old_bytes);
    assert_eq!(new_bytes, fixture.workflow.new_bytes);
    println!("WORKFLOW-OK: composition, parent-first rendering and full-prompt tokens");
    let revision = compare_agentic_ai_context_revision(AgenticAiContextRevisionRequest {
        old: old.context(),
        new: restored.context(),
        old_bundle: &source.bundle,
        new_bundle: &source.bundle,
        old_snapshot: &source.snapshot,
        new_snapshot: &source.snapshot,
    })?;
    // Changed root selection is a global semantic basis change in the existing contract.
    assert!(revision.revision().reusable().elements().is_empty());
    let eligibility =
        new_tokens.reuse_eligibility_from(&old_tokens, &source.bundle, &source.snapshot)?;
    assert_eq!(
        eligibility.stable_token_prefix(),
        fixture.workflow.stable_prefix
    );
    assert_eq!(
        eligibility.eligible_full_block_tokens(),
        fixture.workflow.eligible_tokens
    );
    println!("WORKFLOW-OK: conservative semantic revision and independent token eligibility");
    Ok(WorkflowReceipt {
        schema: "mrr.agentic-ai-context.workflow-receipt.v1",
        old_selected: old.context().closure().elements().len(),
        new_selected: new.context().closure().elements().len(),
        restored: true,
        semantic_reusable: revision.revision().reusable().elements().len(),
        stable_token_prefix: eligibility.stable_token_prefix(),
        eligible_full_block_tokens: eligibility.eligible_full_block_tokens(),
        old_bytes,
        new_bytes,
    })
}
