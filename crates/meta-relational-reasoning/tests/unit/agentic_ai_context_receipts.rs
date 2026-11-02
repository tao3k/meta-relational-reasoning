use super::agentic_ai_context::{admit, contract, limits, snapshot, source, source_with_text};
use crate::{
    AgenticAiContextAdmissionError, AgenticAiContextAdmissionRequest,
    AgenticAiContextComposedMaterializationRequest, AgenticAiContextComposer,
    AgenticAiContextCompositionError, AgenticAiContextCompositionGraph,
    AgenticAiContextCompositionGraphInput, AgenticAiContextCompositionNode,
    AgenticAiContextCompositionProducer, AgenticAiContextCompositionRequest, AgenticAiContextError,
    AgenticAiContextRenderedElement, AgenticAiContextRevisionRequest, Binding, FactId,
    GraphPattern, MetaQueryIr, NodePattern, PathPattern, QueryOperatorId, QueryResult,
    QueryTemplate, ReasoningBundle, admit_agentic_ai_context, bind_query_to_catalog,
    compare_agentic_ai_context_revision,
};
use std::{cell::Cell, num::NonZeroUsize};
fn producer() -> AgenticAiContextCompositionProducer {
    AgenticAiContextCompositionProducer {
        owner: "fixture".into(),
        algorithm: "single-node".into(),
        implementation_version: "v1".into(),
    }
}
fn graph(fact: FactId) -> AgenticAiContextCompositionGraphInput {
    AgenticAiContextCompositionGraphInput {
        root: Some(fact),
        nodes: vec![AgenticAiContextCompositionNode {
            element: fact,
            parent_orders: vec![],
            suffix: false,
        }],
    }
}
struct Composer {
    identity: AgenticAiContextCompositionProducer,
    calls: Cell<usize>,
    fail: bool,
}
impl AgenticAiContextComposer for Composer {
    type Error = std::io::Error;
    fn identity(&self) -> &AgenticAiContextCompositionProducer {
        &self.identity
    }
    fn compose(
        &self,
        graph: &AgenticAiContextCompositionGraph,
    ) -> Result<Vec<FactId>, Self::Error> {
        self.calls.set(self.calls.get() + 1);
        if self.fail {
            return Err(std::io::Error::other("fixture failure"));
        }
        Ok(graph.input().nodes.iter().map(|n| n.element).collect())
    }
}
fn composer() -> Composer {
    Composer {
        identity: producer(),
        calls: Cell::new(0),
        fail: false,
    }
}
#[test]
fn composed_materialization_retains_exact_graph_producer_and_manifest() {
    let (bundle, snap, q, fact) = source();
    let context = admit(&bundle, &snap, q, fact);
    let owner = composer();
    let receipt = context
        .compose(
            &bundle,
            &snap,
            AgenticAiContextCompositionRequest {
                graph: graph(fact),
                expected_producer: producer(),
                composer: &owner,
            },
        )
        .unwrap();
    assert_eq!(receipt.manifest_digest(), context.manifest().digest());
    assert_eq!(receipt.producer(), &producer());
    assert_eq!(receipt.graph().input(), &graph(fact));
    assert_eq!(owner.calls.get(), 1);
    let materialize = |c: &crate::AdmittedAgenticAiContext| {
        c.materialize_composed(
            &bundle,
            &snap,
            AgenticAiContextComposedMaterializationRequest {
                composition: &receipt,
                renderer_identity: "renderer".into(),
                segments: vec![AgenticAiContextRenderedElement {
                    element: fact,
                    bytes: vec![1, 2],
                }],
            },
        )
    };
    let presentation = materialize(&context).unwrap();
    assert_eq!(presentation.composition_receipt(), Some(&receipt));
    let request = AgenticAiContextAdmissionRequest {
        contract: contract(),
        roots: vec![fact],
        dependencies: vec![(fact, vec![fact])],
        limits: limits(),
    };
    let bound = bind_query_to_catalog(&bundle, q, &snap).unwrap();
    let foreign = admit_agentic_ai_context(&bundle, &snap, &bound, request.clone()).unwrap();
    assert_eq!(foreign.closure(), context.closure());
    assert_eq!(
        materialize(&foreign),
        Err(AgenticAiContextAdmissionError::CompositionMismatch)
    );
    let mut other_producer = producer();
    other_producer.implementation_version = "v2".into();
    let other = Composer {
        identity: other_producer.clone(),
        ..composer()
    };
    let versioned = context
        .compose(
            &bundle,
            &snap,
            AgenticAiContextCompositionRequest {
                graph: graph(fact),
                expected_producer: other_producer,
                composer: &other,
            },
        )
        .unwrap();
    assert_ne!(receipt.digest(), versioned.digest());
    #[cfg(feature = "agentic-ai-context-tokens")]
    {
        let tokens = presentation
            .bind_tokens(
                &bundle,
                &snap,
                crate::AgenticAiContextTokenBindingRequest {
                    identity: crate::AgenticAiContextComputationalIdentity {
                        model_weights: "weights".into(),
                        tokenizer: "t".into(),
                        renderer: "renderer".into(),
                        chat_template: "template".into(),
                        position_scheme: "positions".into(),
                        attention_semantics: "causal".into(),
                        cache_format: "cache".into(),
                        adapter_digest: [0; 32],
                        multimodal_digest: [0; 32],
                        sharing_scope: [0; 32],
                        block_tokens: NonZeroUsize::new(1).unwrap(),
                    },
                    tokens: vec![1, 2],
                },
            )
            .unwrap();
        assert_eq!(tokens.presentation().composition_receipt(), Some(&receipt));
    }
}
#[test]
fn source_version_and_graph_failures_prevent_composer_invocation() {
    let (bundle, snap, q, fact) = source();
    let context = admit(&bundle, &snap, q, fact);
    let owner = composer();
    let stale = snapshot(snap.generation(), "two");
    assert!(matches!(
        context.compose(
            &bundle,
            &stale,
            AgenticAiContextCompositionRequest {
                graph: graph(fact),
                expected_producer: producer(),
                composer: &owner
            }
        ),
        Err(AgenticAiContextCompositionError::Admission(_))
    ));
    let mut version = producer();
    version.implementation_version = "v2".into();
    assert!(matches!(
        context.compose(
            &bundle,
            &snap,
            AgenticAiContextCompositionRequest {
                graph: graph(fact),
                expected_producer: version,
                composer: &owner
            }
        ),
        Err(AgenticAiContextCompositionError::Admission(
            AgenticAiContextAdmissionError::CompositionProducerMismatch
        ))
    ));
    let mut bad = graph(fact);
    bad.nodes[0].parent_orders = vec![vec![fact]];
    assert!(matches!(
        context.compose(
            &bundle,
            &snap,
            AgenticAiContextCompositionRequest {
                graph: bad,
                expected_producer: producer(),
                composer: &owner
            }
        ),
        Err(AgenticAiContextCompositionError::Admission(
            AgenticAiContextAdmissionError::Context(AgenticAiContextError::CompositionCycle)
        ))
    ));
    let mut budget = graph(fact);
    budget.nodes[0].parent_orders = vec![vec![]; 33];
    assert!(
        context
            .compose(
                &bundle,
                &snap,
                AgenticAiContextCompositionRequest {
                    graph: budget,
                    expected_producer: producer(),
                    composer: &owner
                }
            )
            .is_err()
    );
    assert_eq!(owner.calls.get(), 0);
    let failing = Composer {
        fail: true,
        ..composer()
    };
    let error = context
        .compose(
            &bundle,
            &snap,
            AgenticAiContextCompositionRequest {
                graph: graph(fact),
                expected_producer: producer(),
                composer: &failing,
            },
        )
        .unwrap_err();
    assert!(
        std::error::Error::source(&error)
            .unwrap()
            .is::<std::io::Error>()
    );
}
#[test]
fn revision_binds_pair_and_preserves_unchanged_semantics_across_budget_changes() {
    let (bundle, snap, q, fact) = source();
    let old = admit(&bundle, &snap, q, fact);
    let mut ceiling = limits();
    ceiling.max_rendered_bytes = NonZeroUsize::new(128).unwrap();
    let bound = bind_query_to_catalog(&bundle, q, &snap).unwrap();
    let new = admit_agentic_ai_context(
        &bundle,
        &snap,
        &bound,
        AgenticAiContextAdmissionRequest {
            contract: contract(),
            roots: vec![fact],
            dependencies: vec![],
            limits: ceiling,
        },
    )
    .unwrap();
    let receipt = compare_agentic_ai_context_revision(AgenticAiContextRevisionRequest {
        old: &old,
        new: &new,
        old_bundle: &bundle,
        old_snapshot: &snap,
        new_bundle: &bundle,
        new_snapshot: &snap,
    })
    .unwrap();
    assert_ne!(receipt.from_manifest(), receipt.to_manifest());
    assert!(!receipt.query_binding_changed());
    assert_eq!(receipt.revision().reusable().elements(), &[fact]);
    receipt.check_contexts(&old, &new).unwrap();
    assert_eq!(
        receipt.check_contexts(&new, &old),
        Err(AgenticAiContextAdmissionError::RevisionMismatch)
    );
    let stale = snapshot(snap.generation(), "two");
    assert!(
        compare_agentic_ai_context_revision(AgenticAiContextRevisionRequest {
            old: &old,
            new: &new,
            old_bundle: &bundle,
            old_snapshot: &stale,
            new_bundle: &bundle,
            new_snapshot: &snap
        })
        .is_err()
    );
}
#[test]
fn actual_query_binding_drift_invalidates_identical_selected_facts() {
    let (bundle, snap, q, fact) = source();
    let old = admit(&bundle, &snap, q, fact);
    let mut declaration = bundle.declaration().clone();
    let query = MetaQueryIr::new(
        q,
        GraphPattern::new(
            QueryOperatorId::from_canonical_bytes("revised-graph").unwrap(),
            vec![PathPattern::new(
                NodePattern::new(Binding::new("different").unwrap(), vec![]),
                vec![],
            )],
        )
        .unwrap(),
        vec![],
        QueryResult::finish(),
    )
    .unwrap();
    declaration.query_templates = vec![QueryTemplate::new(query, vec![])];
    let replacement = ReasoningBundle::admit(declaration).unwrap();
    let new = admit(&replacement, &snap, q, fact);
    assert_eq!(old.state(), new.state());
    let receipt = compare_agentic_ai_context_revision(AgenticAiContextRevisionRequest {
        old: &old,
        new: &new,
        old_bundle: &bundle,
        old_snapshot: &snap,
        new_bundle: &replacement,
        new_snapshot: &snap,
    })
    .unwrap();
    assert!(receipt.query_binding_changed());
    assert_eq!(receipt.revision().invalidated(), &[fact]);
    assert!(receipt.revision().reusable().elements().is_empty());
    let (changed, _, _, _) = source_with_text("new payload");
    let next = admit(&changed, &snap, q, fact);
    let payload = compare_agentic_ai_context_revision(AgenticAiContextRevisionRequest {
        old: &old,
        new: &next,
        old_bundle: &bundle,
        old_snapshot: &snap,
        new_bundle: &changed,
        new_snapshot: &snap,
    })
    .unwrap();
    assert!(!payload.query_binding_changed());
    assert_eq!(payload.revision().changed(), &[fact]);
}

#[test]
fn revision_uses_both_dependency_graphs_and_preserves_independent_facts() {
    let (base, snap, q, leaf) = source();
    let parent = FactId::from_canonical_bytes("dependent-parent").unwrap();
    let root = FactId::from_canonical_bytes("dependent-root").unwrap();
    let independent = FactId::from_canonical_bytes("independent").unwrap();
    let make_bundle = |changed: bool| {
        let mut declaration = base.declaration().clone();
        let original = &base.facts()[0];
        declaration.facts = [leaf, parent, root, independent]
            .into_iter()
            .map(|id| {
                crate::Fact::new(
                    id,
                    original.relation(),
                    vec![crate::Value::String(if changed && id == leaf {
                        "changed".into()
                    } else {
                        format!("{id:?}")
                    })],
                    *original.context(),
                )
            })
            .collect();
        ReasoningBundle::admit(declaration).unwrap()
    };
    let old_bundle = make_bundle(false);
    let new_bundle = make_bundle(true);
    let build = |bundle: &ReasoningBundle, detach: bool| {
        let bound = bind_query_to_catalog(bundle, q, &snap).unwrap();
        admit_agentic_ai_context(
            bundle,
            &snap,
            &bound,
            AgenticAiContextAdmissionRequest {
                contract: contract(),
                roots: vec![leaf, parent, root, independent],
                dependencies: vec![
                    (parent, if detach { vec![] } else { vec![leaf] }),
                    (root, vec![parent]),
                ],
                limits: limits(),
            },
        )
        .unwrap()
    };
    let old = build(&old_bundle, false);
    let new = build(&new_bundle, true);
    let receipt = compare_agentic_ai_context_revision(AgenticAiContextRevisionRequest {
        old: &old,
        new: &new,
        old_bundle: &old_bundle,
        old_snapshot: &snap,
        new_bundle: &new_bundle,
        new_snapshot: &snap,
    })
    .unwrap();
    assert!(!receipt.query_binding_changed());
    assert!(receipt.revision().changed().contains(&leaf));
    assert!(receipt.revision().changed().contains(&parent));
    assert!(receipt.revision().invalidated().contains(&root));
    assert_eq!(receipt.revision().reusable().elements(), &[independent]);
}

#[test]
fn composition_receipt_digest_binds_graph_metadata_and_rejects_wrong_segments() {
    let (bundle, snap, q, fact) = source();
    let context = admit(&bundle, &snap, q, fact);
    let owner = composer();
    let plain = context
        .compose(
            &bundle,
            &snap,
            AgenticAiContextCompositionRequest {
                graph: graph(fact),
                expected_producer: producer(),
                composer: &owner,
            },
        )
        .unwrap();
    let mut suffix = graph(fact);
    suffix.nodes[0].suffix = true;
    let with_suffix = context
        .compose(
            &bundle,
            &snap,
            AgenticAiContextCompositionRequest {
                graph: suffix,
                expected_producer: producer(),
                composer: &owner,
            },
        )
        .unwrap();
    assert_eq!(plain.composition(), with_suffix.composition());
    assert_ne!(plain.digest(), with_suffix.digest());
    assert!(matches!(
        context.materialize_composed(
            &bundle,
            &snap,
            AgenticAiContextComposedMaterializationRequest {
                composition: &plain,
                renderer_identity: "renderer".into(),
                segments: vec![],
            }
        ),
        Err(AgenticAiContextAdmissionError::Context(
            AgenticAiContextError::RenderedElementMismatch
        ))
    ));
}
