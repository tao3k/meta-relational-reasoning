#[cfg(feature = "agentic-ai-context-tokens")]
use crate::{
    AgenticAiContextComputationalIdentity, AgenticAiContextTokenBindingRequest,
    AgenticAiContextTokenizationError, AgenticAiContextTokenizationRequest,
    AgenticAiContextTokenizer,
};
use std::num::NonZeroUsize;

use crate::{
    AdmittedAgenticAiContext, AgenticAiContextAdmissionError, AgenticAiContextAdmissionRequest,
    AgenticAiContextContract, AgenticAiContextError, AgenticAiContextLimits,
    AgenticAiContextMaterializationRequest, AgenticAiContextRenderedElement, Binding, EntityId,
    EvidenceCompleteness, ExternalRevisionIdentity, Fact, FactId, FactProvenance, FactValidity,
    GenerationId, GraphPattern, MetaQueryIr, NodePattern, PathPattern, QueryId, QueryOperatorId,
    QueryResult, QueryTemplate, ReasoningBundle, ReasoningBundleDeclaration, RelationAuthority,
    RelationContext, RelationField, RelationId, RelationSchema, RevisionBinding, SemanticSnapshot,
    StateId, Value, ValueSchema, admit_agentic_ai_context, bind_query_to_catalog,
};

pub(super) fn snapshot(generation: GenerationId, revision: &str) -> SemanticSnapshot {
    SemanticSnapshot::admit(
        generation,
        vec![
            RevisionBinding::admit(
                ExternalRevisionIdentity::new("git", "context-source", revision).unwrap(),
                generation,
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

pub(super) fn source() -> (ReasoningBundle, SemanticSnapshot, QueryId, FactId) {
    source_with_text("admitted evidence")
}

pub(super) fn source_with_text(text: &str) -> (ReasoningBundle, SemanticSnapshot, QueryId, FactId) {
    let generation = GenerationId::from_canonical_bytes("context-generation").unwrap();
    let relation = RelationId::from_canonical_bytes("context-relation").unwrap();
    let source = EntityId::from_canonical_bytes("context-source").unwrap();
    let fact_id = FactId::from_canonical_bytes("context-fact").unwrap();
    let query_id = QueryId::from_canonical_bytes("context-query").unwrap();
    let query = MetaQueryIr::new(
        query_id,
        GraphPattern::new(
            QueryOperatorId::from_canonical_bytes("context-graph").unwrap(),
            vec![PathPattern::new(
                NodePattern::new(Binding::new("source").unwrap(), vec![]),
                vec![],
            )],
        )
        .unwrap(),
        vec![],
        QueryResult::finish(),
    )
    .unwrap();
    let fact = Fact::new(
        fact_id,
        relation,
        vec![Value::String(text.into())],
        RelationContext::new(
            generation,
            RelationAuthority::Entity(source),
            FactProvenance::Source(source),
            EvidenceCompleteness::Complete,
            FactValidity::Valid,
        )
        .unwrap(),
    );
    let bundle = ReasoningBundle::admit(ReasoningBundleDeclaration {
        relations: vec![
            RelationSchema::new(
                relation,
                "evidence",
                vec![RelationField::new("statement", ValueSchema::String, false).unwrap()],
                vec![],
            )
            .unwrap(),
        ],
        facts: vec![fact],
        query_templates: vec![QueryTemplate::new(query, vec![])],
        ..ReasoningBundleDeclaration::default()
    })
    .unwrap();
    (bundle, snapshot(generation, "one"), query_id, fact_id)
}

pub(super) fn contract() -> AgenticAiContextContract {
    AgenticAiContextContract {
        actor: EntityId::from_canonical_bytes("actor").unwrap(),
        task: StateId::from_canonical_bytes("task").unwrap(),
        policy_digest: [1; 32],
        required: vec![],
        temporal_receipts: vec![],
        require_complete: true,
    }
}

pub(super) fn limits() -> AgenticAiContextLimits {
    AgenticAiContextLimits {
        max_elements: NonZeroUsize::new(16).unwrap(),
        max_dependency_edges: NonZeroUsize::new(32).unwrap(),
        max_rendered_bytes: NonZeroUsize::new(64).unwrap(),
    }
}

pub(super) fn admit(
    bundle: &ReasoningBundle,
    snapshot: &SemanticSnapshot,
    query_id: QueryId,
    fact: FactId,
) -> AdmittedAgenticAiContext {
    let query = bind_query_to_catalog(bundle, query_id, snapshot).unwrap();
    admit_agentic_ai_context(
        bundle,
        snapshot,
        &query,
        AgenticAiContextAdmissionRequest {
            contract: contract(),
            roots: vec![fact],
            dependencies: vec![],
            limits: limits(),
        },
    )
    .unwrap()
}

pub(super) fn presentation_request(
    fact: FactId,
    bytes: &[u8],
) -> AgenticAiContextMaterializationRequest {
    AgenticAiContextMaterializationRequest {
        precedence: vec![fact],
        renderer_identity: "fixture-renderer-v1".into(),
        segments: vec![AgenticAiContextRenderedElement {
            element: fact,
            bytes: bytes.to_vec(),
        }],
    }
}

#[test]
fn context_materialization_is_source_bound_without_token_support() {
    let (bundle, source_snapshot, query, fact) = source();
    let admitted = admit(&bundle, &source_snapshot, query, fact);
    let presentation = admitted
        .materialize(
            &bundle,
            &source_snapshot,
            presentation_request(fact, &[1, 2]),
        )
        .unwrap();
    assert_eq!(presentation.source_bundle(), bundle.id());
    assert_eq!(presentation.materialization().bytes(), &[1, 2]);
    let (replacement, _, _, _) = source_with_text("changed evidence");
    let revised = snapshot(source_snapshot.generation(), "two");
    assert_eq!(
        presentation.check_source(&replacement, &source_snapshot),
        Err(AgenticAiContextAdmissionError::SourceBundleMismatch)
    );
    assert_eq!(
        admitted.materialize(&bundle, &revised, presentation_request(fact, &[1, 2])),
        Err(AgenticAiContextAdmissionError::QueryBindingMismatch)
    );
}

#[cfg(feature = "agentic-ai-context-tokens")]
fn token_request(tokens: &[u32]) -> AgenticAiContextTokenBindingRequest {
    AgenticAiContextTokenBindingRequest {
        identity: AgenticAiContextComputationalIdentity {
            model_weights: "fixture-model-v1".into(),
            tokenizer: "fixture-byte-tokenizer-v1".into(),
            renderer: "fixture-renderer-v1".into(),
            chat_template: "fixture-template-v1".into(),
            position_scheme: "absolute-v1".into(),
            attention_semantics: "causal-v1".into(),
            cache_format: "fixture-cache-v1".into(),
            adapter_digest: [0; 32],
            multimodal_digest: [0; 32],
            sharing_scope: [2; 32],
            block_tokens: NonZeroUsize::new(2).unwrap(),
        },
        tokens: tokens.to_vec(),
    }
}

#[cfg(feature = "agentic-ai-context-tokens")]
struct FixtureTokenizer {
    calls: std::cell::Cell<usize>,
    identity: &'static str,
    fail: bool,
}

#[cfg(feature = "agentic-ai-context-tokens")]
impl AgenticAiContextTokenizer for FixtureTokenizer {
    type Error = std::io::Error;

    fn identity(&self) -> &str {
        self.identity
    }

    fn tokenize(&self, presentation: &[u8]) -> Result<Vec<u32>, Self::Error> {
        self.calls.set(self.calls.get() + 1);
        if self.fail {
            return Err(std::io::Error::other("fixture tokenizer unavailable"));
        }
        Ok(presentation.iter().map(|byte| u32::from(*byte)).collect())
    }
}

#[cfg(feature = "agentic-ai-context-tokens")]
#[test]
fn context_tokenizer_adapter_encodes_the_exact_full_presentation() {
    let (bundle, snapshot, query, fact) = source();
    let presentation = admit(&bundle, &snapshot, query, fact)
        .materialize(
            &bundle,
            &snapshot,
            presentation_request(fact, &[1, 2, 3, 4]),
        )
        .unwrap();
    let adapter = FixtureTokenizer {
        calls: std::cell::Cell::new(0),
        identity: "fixture-byte-tokenizer-v1",
        fail: false,
    };
    let tokens = presentation
        .tokenize(
            &bundle,
            &snapshot,
            AgenticAiContextTokenizationRequest {
                identity: token_request(&[]).identity,
                tokenizer: &adapter,
            },
        )
        .unwrap();
    assert_eq!(adapter.calls.get(), 1);
    assert_eq!(tokens.layout().tokens(), &[1, 2, 3, 4]);
    assert_eq!(
        tokens.presentation().materialization().bytes(),
        &[1, 2, 3, 4]
    );
}

#[cfg(feature = "agentic-ai-context-tokens")]
#[test]
fn context_tokenizer_is_not_invoked_for_stale_source_or_invalid_configuration() {
    let (bundle, source_snapshot, query, fact) = source();
    let presentation = admit(&bundle, &source_snapshot, query, fact)
        .materialize(
            &bundle,
            &source_snapshot,
            presentation_request(fact, &[1, 2]),
        )
        .unwrap();
    let revised = snapshot(source_snapshot.generation(), "two");
    let adapter = FixtureTokenizer {
        calls: std::cell::Cell::new(0),
        identity: "fixture-byte-tokenizer-v1",
        fail: false,
    };
    assert!(matches!(
        presentation.clone().tokenize(
            &bundle,
            &revised,
            AgenticAiContextTokenizationRequest {
                identity: token_request(&[]).identity,
                tokenizer: &adapter,
            },
        ),
        Err(AgenticAiContextTokenizationError::Admission(
            AgenticAiContextAdmissionError::QueryBindingMismatch
        ))
    ));
    for field in ["tokenizer", "renderer", "model_weights"] {
        let mut identity = token_request(&[]).identity;
        match field {
            "tokenizer" => identity.tokenizer = "foreign-tokenizer".into(),
            "renderer" => identity.renderer = "foreign-renderer".into(),
            _ => identity.model_weights = String::new(),
        }
        assert!(matches!(
            presentation.clone().tokenize(
                &bundle,
                &source_snapshot,
                AgenticAiContextTokenizationRequest {
                    identity,
                    tokenizer: &adapter,
                },
            ),
            Err(AgenticAiContextTokenizationError::Admission(
                AgenticAiContextAdmissionError::Context(_)
            ))
        ));
    }
    assert_eq!(adapter.calls.get(), 0);
}

#[cfg(feature = "agentic-ai-context-tokens")]
#[test]
fn context_tokenizer_failure_preserves_the_upstream_typed_cause() {
    let (bundle, snapshot, query, fact) = source();
    let presentation = admit(&bundle, &snapshot, query, fact)
        .materialize(&bundle, &snapshot, presentation_request(fact, &[1, 2]))
        .unwrap();
    let adapter = FixtureTokenizer {
        calls: std::cell::Cell::new(0),
        identity: "fixture-byte-tokenizer-v1",
        fail: true,
    };
    let error = presentation
        .tokenize(
            &bundle,
            &snapshot,
            AgenticAiContextTokenizationRequest {
                identity: token_request(&[]).identity,
                tokenizer: &adapter,
            },
        )
        .unwrap_err();
    assert!(matches!(
        error,
        AgenticAiContextTokenizationError::Tokenizer(_)
    ));
    assert!(std::error::Error::source(&error).is_some());
    assert_eq!(adapter.calls.get(), 1);
}

#[cfg(feature = "agentic-ai-context-tokens")]
#[test]
fn context_presentation_pipeline_retains_source_and_exact_declared_layout() {
    let (bundle, snapshot, query, fact) = source();
    let admitted = admit(&bundle, &snapshot, query, fact);
    let presentation = admitted
        .materialize(
            &bundle,
            &snapshot,
            presentation_request(fact, &[1, 2, 3, 4]),
        )
        .unwrap();
    assert_eq!(presentation.source_bundle(), bundle.id());
    assert_eq!(presentation.materialization().bytes(), &[1, 2, 3, 4]);
    let tokens = presentation
        .bind_tokens(&bundle, &snapshot, token_request(&[1, 2, 3, 4]))
        .unwrap();
    assert_eq!(tokens.layout().tokens(), &[1, 2, 3, 4]);
    assert_eq!(tokens.presentation().source_bundle(), bundle.id());
    assert_eq!(tokens.presentation().manifest(), admitted.manifest());
    tokens.check_source(&bundle, &snapshot).unwrap();
    let eligibility = tokens
        .reuse_eligibility_from(&tokens, &bundle, &snapshot)
        .unwrap();
    assert_eq!(eligibility.stable_token_prefix(), 4);
    assert_eq!(eligibility.eligible_full_block_tokens(), 4);
}

#[cfg(feature = "agentic-ai-context-tokens")]
#[test]
fn context_presentation_stages_reject_source_replacement_and_stale_revision() {
    let (bundle, source_snapshot, query, fact) = source();
    let admitted = admit(&bundle, &source_snapshot, query, fact);
    let (replacement, _, _, _) = source_with_text("replacement evidence");
    let revised = snapshot(source_snapshot.generation(), "two");
    assert_eq!(
        admitted.materialize(
            &replacement,
            &source_snapshot,
            presentation_request(fact, &[1])
        ),
        Err(AgenticAiContextAdmissionError::SourceBundleMismatch)
    );
    assert_eq!(
        admitted.materialize(&bundle, &revised, presentation_request(fact, &[1])),
        Err(AgenticAiContextAdmissionError::QueryBindingMismatch)
    );
    let presentation = admitted
        .materialize(
            &bundle,
            &source_snapshot,
            presentation_request(fact, &[1, 2]),
        )
        .unwrap();
    assert_eq!(
        presentation.check_source(&replacement, &source_snapshot),
        Err(AgenticAiContextAdmissionError::SourceBundleMismatch)
    );
    assert_eq!(
        presentation
            .clone()
            .bind_tokens(&bundle, &revised, token_request(&[1, 2])),
        Err(AgenticAiContextAdmissionError::QueryBindingMismatch)
    );
    assert_eq!(
        presentation
            .clone()
            .bind_tokens(&replacement, &source_snapshot, token_request(&[1, 2])),
        Err(AgenticAiContextAdmissionError::SourceBundleMismatch)
    );
    let tokens = presentation
        .bind_tokens(&bundle, &source_snapshot, token_request(&[1, 2]))
        .unwrap();
    assert_eq!(
        tokens.reuse_eligibility_from(&tokens, &bundle, &revised),
        Err(AgenticAiContextAdmissionError::QueryBindingMismatch)
    );
    assert_eq!(
        tokens.reuse_eligibility_from(&tokens, &replacement, &source_snapshot),
        Err(AgenticAiContextAdmissionError::SourceBundleMismatch)
    );
}

#[cfg(feature = "agentic-ai-context-tokens")]
#[test]
fn context_reuse_checks_new_evidence_but_uses_old_tokens_as_historical_basis() {
    let (old_bundle, old_snapshot, query, fact) = source();
    let (new_bundle, _, _, _) = source_with_text("revised evidence");
    let new_snapshot = snapshot(old_snapshot.generation(), "two");
    let old = admit(&old_bundle, &old_snapshot, query, fact)
        .materialize(
            &old_bundle,
            &old_snapshot,
            presentation_request(fact, &[1, 2, 3, 4]),
        )
        .unwrap()
        .bind_tokens(&old_bundle, &old_snapshot, token_request(&[1, 2, 3, 4]))
        .unwrap();
    let new = admit(&new_bundle, &new_snapshot, query, fact)
        .materialize(
            &new_bundle,
            &new_snapshot,
            presentation_request(fact, &[1, 2, 3, 9]),
        )
        .unwrap()
        .bind_tokens(&new_bundle, &new_snapshot, token_request(&[1, 2, 3, 9]))
        .unwrap();
    assert_eq!(
        old.check_source(&new_bundle, &new_snapshot),
        Err(AgenticAiContextAdmissionError::SourceBundleMismatch)
    );
    let eligibility = new
        .reuse_eligibility_from(&old, &new_bundle, &new_snapshot)
        .unwrap();
    assert_eq!(eligibility.stable_token_prefix(), 3);
    assert_eq!(eligibility.eligible_full_block_tokens(), 2);
}

#[cfg(feature = "agentic-ai-context-tokens")]
#[test]
fn context_presentation_rejects_foreign_segments_and_renderer_identity_drift() {
    let (bundle, snapshot, query, fact) = source();
    let admitted = admit(&bundle, &snapshot, query, fact);
    let mut wrong = presentation_request(fact, &[1, 2]);
    wrong.segments[0].element = FactId::from_canonical_bytes("foreign").unwrap();
    assert_eq!(
        admitted.materialize(&bundle, &snapshot, wrong),
        Err(AgenticAiContextAdmissionError::Context(
            AgenticAiContextError::RenderedElementMismatch
        ))
    );
    let presentation = admitted
        .materialize(&bundle, &snapshot, presentation_request(fact, &[1, 2]))
        .unwrap();
    let mut wrong = token_request(&[1, 2]);
    wrong.identity.renderer = "another-renderer".into();
    assert_eq!(
        presentation.bind_tokens(&bundle, &snapshot, wrong),
        Err(AgenticAiContextAdmissionError::Context(
            AgenticAiContextError::ComputationalIdentityMismatch
        ))
    );
}

#[test]
fn context_admission_retains_exact_bundle_snapshot_and_query_binding() {
    let (bundle, snapshot, query_id, fact) = source();
    let query = bind_query_to_catalog(&bundle, query_id, &snapshot).unwrap();
    let context = admit_agentic_ai_context(
        &bundle,
        &snapshot,
        &query,
        AgenticAiContextAdmissionRequest {
            contract: contract(),
            roots: vec![fact],
            dependencies: vec![],
            limits: limits(),
        },
    )
    .unwrap();
    assert_eq!(context.source_bundle(), bundle.id());
    assert_eq!(context.state().snapshot(), &snapshot);
    assert_eq!(context.state().query().id(), query_id);
    assert_eq!(context.closure().elements(), &[fact]);
    context.check_source(&bundle, &snapshot).unwrap();
}

#[test]
fn context_admission_rejects_stale_query_binding_and_unknown_evidence() {
    let (bundle, source_snapshot, query_id, fact) = source();
    let query = bind_query_to_catalog(&bundle, query_id, &source_snapshot).unwrap();
    let changed = snapshot(source_snapshot.generation(), "two");
    assert_eq!(
        admit_agentic_ai_context(
            &bundle,
            &changed,
            &query,
            AgenticAiContextAdmissionRequest {
                contract: contract(),
                roots: vec![fact],
                dependencies: vec![],
                limits: limits()
            }
        ),
        Err(AgenticAiContextAdmissionError::QueryBindingMismatch)
    );
    let missing = FactId::from_canonical_bytes("missing").unwrap();
    assert_eq!(
        admit_agentic_ai_context(
            &bundle,
            &source_snapshot,
            &query,
            AgenticAiContextAdmissionRequest {
                contract: contract(),
                roots: vec![missing],
                dependencies: vec![],
                limits: limits()
            }
        ),
        Err(AgenticAiContextAdmissionError::Context(
            AgenticAiContextError::UnknownElement(missing)
        ))
    );
    assert_eq!(
        admit_agentic_ai_context(
            &bundle,
            &source_snapshot,
            &query,
            AgenticAiContextAdmissionRequest {
                contract: contract(),
                roots: vec![fact],
                dependencies: vec![(missing, vec![fact])],
                limits: limits()
            }
        ),
        Err(AgenticAiContextAdmissionError::Context(
            AgenticAiContextError::UnknownElement(missing)
        ))
    );
    let admitted = admit_agentic_ai_context(
        &bundle,
        &source_snapshot,
        &query,
        AgenticAiContextAdmissionRequest {
            contract: contract(),
            roots: vec![fact],
            dependencies: vec![],
            limits: limits(),
        },
    )
    .unwrap();
    assert_eq!(
        admitted.check_source(&bundle, &changed),
        Err(AgenticAiContextAdmissionError::QueryBindingMismatch)
    );
}
