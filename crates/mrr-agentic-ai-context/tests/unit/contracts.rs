#[cfg(feature = "token-layout")]
use crate::{
    AgenticAiContextComputationalIdentity, AgenticAiContextReuseEligibility,
    AgenticAiContextTokenLayout,
};
use std::num::NonZeroUsize;

use mrr_identity::{EntityId, FactId, GenerationId, QueryId, RelationId, StateId};
use mrr_relation::{
    EvidenceCompleteness, Fact, FactProvenance, FactValidity, RelationAuthority, RelationContext,
    Value,
};
use mrr_revision::{ExternalRevisionIdentity, RevisionBinding, SemanticSnapshot};

use crate::{
    AgenticAiContextComposition, AgenticAiContextContract, AgenticAiContextElement,
    AgenticAiContextError, AgenticAiContextLimits, AgenticAiContextMaterialization,
    AgenticAiContextQuery, AgenticAiContextRenderedElement, AgenticAiContextRevision,
    AgenticAiContextState, AgenticAiContextStateInput,
};

pub(super) fn fact_id(name: &str) -> FactId {
    FactId::from_canonical_bytes(name).unwrap()
}
fn generation() -> GenerationId {
    GenerationId::from_canonical_bytes("context-generation").unwrap()
}

pub(super) fn snapshot() -> SemanticSnapshot {
    SemanticSnapshot::admit(
        generation(),
        vec![
            RevisionBinding::admit(
                ExternalRevisionIdentity::new("git", "context", "revision-one").unwrap(),
                generation(),
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

pub(super) fn element(name: &str, dependencies: &[&str]) -> AgenticAiContextElement {
    let source = EntityId::from_canonical_bytes("context-source").unwrap();
    let context = RelationContext::new(
        generation(),
        RelationAuthority::Entity(source),
        FactProvenance::Source(source),
        EvidenceCompleteness::Complete,
        FactValidity::Valid,
    )
    .unwrap();
    AgenticAiContextElement::new(
        Fact::new(
            fact_id(name),
            RelationId::from_canonical_bytes("context-relation").unwrap(),
            vec![Value::String(name.into())],
            context,
        ),
        dependencies.iter().map(|name| fact_id(name)).collect(),
    )
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

pub(super) fn state(
    roots: &[&str],
    elements: Vec<AgenticAiContextElement>,
) -> AgenticAiContextState {
    AgenticAiContextState::validate(AgenticAiContextStateInput {
        snapshot: snapshot(),
        query: AgenticAiContextQuery::new(
            QueryId::from_canonical_bytes("query").unwrap(),
            roots.iter().map(|name| fact_id(name)).collect(),
        ),
        contract: contract(),
        elements,
        limits: limits(),
    })
    .unwrap()
}

fn render(
    state: &AgenticAiContextState,
    precedence: &[&str],
    bytes: &[&[u8]],
) -> AgenticAiContextMaterialization {
    let order = AgenticAiContextComposition::from_precedence(
        state,
        precedence.iter().map(|name| fact_id(name)).collect(),
    )
    .unwrap();
    let segments = order
        .parent_first()
        .zip(bytes)
        .map(|(element, bytes)| AgenticAiContextRenderedElement {
            element,
            bytes: bytes.to_vec(),
        })
        .collect();
    AgenticAiContextMaterialization::new(state, order, "segments-v1".into(), segments).unwrap()
}

#[cfg(feature = "token-layout")]
fn identity() -> AgenticAiContextComputationalIdentity {
    AgenticAiContextComputationalIdentity {
        model_weights: "weights-v1".into(),
        tokenizer: "tokenizer-v1".into(),
        renderer: "segments-v1".into(),
        chat_template: "template-v1".into(),
        position_scheme: "absolute-v1".into(),
        attention_semantics: "causal-full-v1".into(),
        cache_format: "kv-v1".into(),
        adapter_digest: [0; 32],
        multimodal_digest: [0; 32],
        sharing_scope: [1; 32],
        block_tokens: NonZeroUsize::new(2).unwrap(),
    }
}

#[test]
fn required_closure_includes_contract_and_typed_temporal_refs_and_terminates_cycles() {
    let mut scoped = contract();
    scoped.required = vec![fact_id("required")];
    scoped.temporal_receipts = vec![fact_id("temporal")];
    let input = AgenticAiContextState::validate(AgenticAiContextStateInput {
        snapshot: snapshot(),
        query: AgenticAiContextQuery::new(
            QueryId::from_canonical_bytes("query").unwrap(),
            vec![fact_id("a")],
        ),
        contract: scoped,
        elements: vec![
            element("a", &["b"]),
            element("b", &["a"]),
            element("required", &[]),
            element("temporal", &[]),
            element("unused", &[]),
        ],
        limits: limits(),
    })
    .unwrap();
    let closure = input.required_closure();
    assert_eq!(closure.elements().len(), 4);
    assert!(!closure.elements().contains(&fact_id("unused")));
    assert_eq!(closure.coverage(), EvidenceCompleteness::Complete);
}

#[test]
fn stale_generation_unknown_dependencies_and_duplicates_are_rejected() {
    let original = element("a", &[]);
    let stale_context = RelationContext::new(
        GenerationId::from_canonical_bytes("stale").unwrap(),
        original.fact().context().authority(),
        original.fact().context().provenance(),
        EvidenceCompleteness::Complete,
        FactValidity::Valid,
    )
    .unwrap();
    let stale = AgenticAiContextElement::new(
        Fact::new(
            fact_id("a"),
            original.fact().relation(),
            original.fact().values().to_vec(),
            stale_context,
        ),
        vec![],
    );
    let query = AgenticAiContextQuery::new(
        QueryId::from_canonical_bytes("query").unwrap(),
        vec![fact_id("a")],
    );
    assert!(matches!(
        AgenticAiContextState::validate(AgenticAiContextStateInput {
            snapshot: snapshot(),
            query: query.clone(),
            contract: contract(),
            elements: vec![stale],
            limits: limits()
        }),
        Err(AgenticAiContextError::GenerationMismatch { .. })
    ));
    assert_eq!(
        AgenticAiContextState::validate(AgenticAiContextStateInput {
            snapshot: snapshot(),
            query: query.clone(),
            contract: contract(),
            elements: vec![element("a", &["missing"])],
            limits: limits()
        }),
        Err(AgenticAiContextError::UnknownElement(fact_id("missing")))
    );
    assert_eq!(
        AgenticAiContextState::validate(AgenticAiContextStateInput {
            snapshot: snapshot(),
            query,
            contract: contract(),
            elements: vec![original.clone(), original],
            limits: limits()
        }),
        Err(AgenticAiContextError::DuplicateElement(fact_id("a")))
    );
}

#[test]
fn invalidated_or_incomplete_required_evidence_is_not_materialized() {
    let original = element("a", &[]);
    for (completeness, validity, expected) in [
        (
            EvidenceCompleteness::Partial,
            FactValidity::Valid,
            AgenticAiContextError::IncompleteEvidence(fact_id("a")),
        ),
        (
            EvidenceCompleteness::Complete,
            FactValidity::InvalidatedBy(fact_id("revision")),
            AgenticAiContextError::InvalidatedElement(fact_id("a")),
        ),
    ] {
        let context = RelationContext::new(
            generation(),
            original.fact().context().authority(),
            original.fact().context().provenance(),
            completeness,
            validity,
        )
        .unwrap();
        let changed = AgenticAiContextElement::new(
            Fact::new(
                fact_id("a"),
                original.fact().relation(),
                original.fact().values().to_vec(),
                context,
            ),
            vec![],
        );
        let query = AgenticAiContextQuery::new(
            QueryId::from_canonical_bytes("query").unwrap(),
            vec![fact_id("a")],
        );
        assert_eq!(
            AgenticAiContextState::validate(AgenticAiContextStateInput {
                snapshot: snapshot(),
                query,
                contract: contract(),
                elements: vec![changed],
                limits: limits()
            }),
            Err(expected)
        );
    }
}

#[test]
fn composition_requires_the_exact_required_closure_and_rendering_order() {
    let input = state(&["a"], vec![element("root", &[]), element("a", &["root"])]);
    for invalid in [
        vec![fact_id("a")],
        vec![fact_id("a"), fact_id("a")],
        vec![fact_id("a"), fact_id("other")],
    ] {
        assert_eq!(
            AgenticAiContextComposition::from_precedence(&input, invalid),
            Err(AgenticAiContextError::InvalidPrecedence)
        );
    }
    let composition =
        AgenticAiContextComposition::from_precedence(&input, vec![fact_id("a"), fact_id("root")])
            .unwrap();
    let wrong = vec![
        AgenticAiContextRenderedElement {
            element: fact_id("a"),
            bytes: vec![2],
        },
        AgenticAiContextRenderedElement {
            element: fact_id("root"),
            bytes: vec![1],
        },
    ];
    assert_eq!(
        AgenticAiContextMaterialization::new(&input, composition, "segments-v1".into(), wrong),
        Err(AgenticAiContextError::RenderedElementMismatch)
    );
}

#[test]
fn shared_c4_fixture_replays_byte_append_and_structural_recomposition() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/agentic-ai-context/materialization.json"
    ))
    .unwrap();
    assert_eq!(
        fixture["schema"],
        "mrr.agentic-ai-context.materialization-fixture.v1"
    );
    let base = vec![element("root", &[]), element("a", &["root"])];
    let old = state(&["a"], base.clone());
    let leaf = state(
        &["child"],
        [base.clone(), vec![element("child", &["a"])]].concat(),
    );
    let recomposed = state(
        &["child"],
        [
            base,
            vec![element("b", &["root"]), element("child", &["a", "b"])],
        ]
        .concat(),
    );
    let replay = |state: &AgenticAiContextState, key: &str| {
        let order: Vec<_> = fixture[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|name| name.as_str().unwrap())
            .collect();
        let bytes: Vec<Vec<u8>> = order
            .iter()
            .rev()
            .map(|name| {
                fixture["segments"][*name]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|byte| u8::try_from(byte.as_u64().unwrap()).unwrap())
                    .collect()
            })
            .collect();
        render(
            state,
            &order,
            &bytes.iter().map(Vec::as_slice).collect::<Vec<_>>(),
        )
    };
    let old_view = replay(&old, "old_precedence");
    let leaf_view = replay(&leaf, "leaf_precedence");
    let recomposed_view = replay(&recomposed, "recomposed_precedence");
    for (view, key) in [
        (&old_view, "old_bytes"),
        (&leaf_view, "leaf_bytes"),
        (&recomposed_view, "recomposed_bytes"),
    ] {
        assert_eq!(serde_json::to_value(view.bytes()).unwrap(), fixture[key]);
    }
    assert!(leaf_view.is_leaf_append_of(&old_view, fact_id("child")));
    assert!(!recomposed_view.bytes().starts_with(old_view.bytes()));
    assert_eq!(leaf_view.spans()[2].start, old_view.bytes().len());
}

#[test]
fn changed_old_payload_breaks_leaf_append_despite_matching_order() {
    let old = state(&["a"], vec![element("a", &[])]);
    let new = state(
        &["child"],
        vec![element("a", &[]), element("child", &["a"])],
    );
    let old_view = render(&old, &["a"], &[&[1]]);
    let new_view = render(&new, &["child", "a"], &[&[9], &[2]]);
    assert!(!new_view.is_leaf_append_of(&old_view, fact_id("child")));
}

#[test]
fn revision_follows_declared_transitive_dependencies_but_keeps_unaffected_evidence() {
    let mut scoped = contract();
    scoped.required = vec![fact_id("independent")];
    let make = |a| {
        AgenticAiContextState::validate(AgenticAiContextStateInput {
            snapshot: snapshot(),
            query: AgenticAiContextQuery::new(
                QueryId::from_canonical_bytes("query").unwrap(),
                vec![fact_id("c")],
            ),
            contract: scoped.clone(),
            elements: vec![
                a,
                element("b", &["a"]),
                element("c", &["b"]),
                element("independent", &[]),
            ],
            limits: limits(),
        })
        .unwrap()
    };
    let original = element("a", &[]);
    let changed = AgenticAiContextElement::new(
        Fact::new(
            fact_id("a"),
            original.fact().relation(),
            vec![Value::String("changed".into())],
            *original.fact().context(),
        ),
        vec![],
    );
    let old = make(original);
    let new = make(changed);
    let revision = AgenticAiContextRevision::between(&old, &new);
    assert_eq!(revision.changed(), &[fact_id("a")]);
    for name in ["a", "b", "c"] {
        assert!(revision.invalidated().contains(&fact_id(name)));
    }
    assert_eq!(revision.reusable().elements(), &[fact_id("independent")]);
}

#[cfg(feature = "token-layout")]
#[test]
fn actual_tokens_detect_cross_boundary_merge_and_round_down_full_blocks() {
    let old = state(&["a"], vec![element("a", &[])]);
    let new = state(
        &["child"],
        vec![element("a", &[]), element("child", &["a"])],
    );
    let old_view = render(&old, &["a"], &[b"a"]);
    let new_view = render(&new, &["child", "a"], &[b"a", b"b"]);
    assert!(new_view.is_leaf_append_of(&old_view, fact_id("child")));
    let old_layout = AgenticAiContextTokenLayout::new(&old_view, identity(), vec![97]).unwrap();
    let new_layout = AgenticAiContextTokenLayout::new(&new_view, identity(), vec![256]).unwrap();
    assert_eq!(
        AgenticAiContextReuseEligibility::between(&old_layout, &new_layout)
            .unwrap()
            .stable_token_prefix(),
        0
    );
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/agentic-ai-context/materialization.json"
    ))
    .unwrap();
    let tokens = |key: &str| {
        fixture["token_revision"][key]
            .as_array()
            .unwrap()
            .iter()
            .map(|token| u32::try_from(token.as_u64().unwrap()).unwrap())
            .collect()
    };
    let old_layout =
        AgenticAiContextTokenLayout::new(&old_view, identity(), tokens("old")).unwrap();
    let new_layout =
        AgenticAiContextTokenLayout::new(&new_view, identity(), tokens("new")).unwrap();
    let eligibility = AgenticAiContextReuseEligibility::between(&old_layout, &new_layout).unwrap();
    assert_eq!(
        eligibility.stable_token_prefix(),
        fixture["token_revision"]["stable_prefix"]
            .as_array()
            .unwrap()
            .len()
    );
    assert_eq!(
        eligibility.eligible_full_block_tokens() as u64,
        fixture["token_revision"]["eligible_full_block_tokens"]
            .as_u64()
            .unwrap()
    );
}

#[cfg(feature = "token-layout")]
#[test]
fn computation_and_contract_drift_deny_even_identical_tokens() {
    let input = state(&["a"], vec![element("a", &[])]);
    let view = render(&input, &["a"], &[&[1]]);
    let old = AgenticAiContextTokenLayout::new(&view, identity(), vec![1, 2, 3]).unwrap();
    let mut changed = identity();
    changed.model_weights = "weights-v2".into();
    let new = AgenticAiContextTokenLayout::new(&view, changed, vec![1, 2, 3]).unwrap();
    assert_eq!(
        AgenticAiContextReuseEligibility::between(&old, &new),
        Err(AgenticAiContextError::ComputationalIdentityMismatch)
    );
    let mut changed_contract = contract();
    changed_contract.policy_digest = [9; 32];
    let changed_state = AgenticAiContextState::validate(AgenticAiContextStateInput {
        snapshot: snapshot(),
        query: input.query().clone(),
        contract: changed_contract,
        elements: vec![element("a", &[])],
        limits: limits(),
    })
    .unwrap();
    let new_view = render(&changed_state, &["a"], &[&[1]]);
    let new = AgenticAiContextTokenLayout::new(&new_view, identity(), vec![1, 2, 3]).unwrap();
    assert_eq!(
        AgenticAiContextReuseEligibility::between(&old, &new),
        Err(AgenticAiContextError::ComputationalIdentityMismatch)
    );
    assert!(
        AgenticAiContextRevision::between(&input, &changed_state)
            .reusable()
            .elements()
            .is_empty()
    );
}

#[test]
fn budgets_fail_before_context_or_byte_output_is_published() {
    let mut bounded = limits();
    bounded.max_elements = NonZeroUsize::new(1).unwrap();
    let query = AgenticAiContextQuery::new(
        QueryId::from_canonical_bytes("query").unwrap(),
        vec![fact_id("a")],
    );
    assert_eq!(
        AgenticAiContextState::validate(AgenticAiContextStateInput {
            snapshot: snapshot(),
            query: query.clone(),
            contract: contract(),
            elements: vec![element("a", &[]), element("b", &[])],
            limits: bounded
        }),
        Err(AgenticAiContextError::ElementBudget)
    );
    let mut bounded = limits();
    bounded.max_rendered_bytes = NonZeroUsize::new(1).unwrap();
    let input = AgenticAiContextState::validate(AgenticAiContextStateInput {
        snapshot: snapshot(),
        query,
        contract: contract(),
        elements: vec![element("a", &[])],
        limits: bounded,
    })
    .unwrap();
    let composition =
        AgenticAiContextComposition::from_precedence(&input, vec![fact_id("a")]).unwrap();
    assert_eq!(
        AgenticAiContextMaterialization::new(
            &input,
            composition,
            "segments-v1".into(),
            vec![AgenticAiContextRenderedElement {
                element: fact_id("a"),
                bytes: vec![1, 2]
            }]
        ),
        Err(AgenticAiContextError::RenderedByteBudget)
    );
}
