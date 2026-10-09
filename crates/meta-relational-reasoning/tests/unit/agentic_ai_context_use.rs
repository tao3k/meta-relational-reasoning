use super::agentic_ai_context::{admit, snapshot, source, source_with_text};
use crate::{
    AgenticAiContextManifest, AgenticAiContextMaterializationRequest,
    AgenticAiContextRenderedElement, AgenticAiContextUseAuthority,
    AgenticAiContextUseDecision as Decision, AgenticAiContextUseError as Error,
    AgenticAiContextUseObservation, AgenticAiContextUsePurpose as Purpose, EntityId, StateId,
};
use std::cell::Cell;

struct Authority {
    calls: Cell<usize>,
    observation: Result<AgenticAiContextUseObservation, &'static str>,
}
impl AgenticAiContextUseAuthority for Authority {
    type Error = &'static str;
    fn observe(
        &self,
        _: &AgenticAiContextManifest,
        _: Purpose,
    ) -> Result<AgenticAiContextUseObservation, Self::Error> {
        self.calls.set(self.calls.get() + 1);
        self.observation.clone()
    }
}
fn authority(manifest: &AgenticAiContextManifest) -> Authority {
    Authority {
        calls: Cell::new(0),
        observation: Ok(AgenticAiContextUseObservation {
            manifest_digest: *manifest.digest(),
            purpose: Purpose::Display,
            current_contract: manifest.record().contract.clone(),
            decision: Decision::Allowed,
        }),
    }
}

#[test]
fn current_use_replays_all_finite_quint_inputs() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/agentic-ai-context/use.json"
    ))
    .unwrap();
    assert_eq!(fixture["schema"], "mrr.context-use-cases.v1");
    assert_eq!(
        fixture["checks"],
        serde_json::json!([
            "source",
            "query",
            "contract",
            "observation",
            "authorization",
            "expiry"
        ])
    );
    let (bundle, snap, query, fact) = source();
    let context = admit(&bundle, &snap, query, fact);
    let (replaced, _, _, _) = source_with_text("replacement with the same fact IDs");
    let stale = snapshot(snap.generation(), "two");
    let mut seen = std::collections::BTreeSet::new();
    for case in fixture["cases"].as_array().unwrap() {
        let valid: std::collections::BTreeSet<_> = case["valid"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_u64().unwrap())
            .collect();
        assert!(valid.iter().all(|x| *x < 6));
        assert!(seen.insert(valid.clone()), "duplicate finite input");
        let owner = authority(context.manifest());
        let mut observation = owner.observation.clone().unwrap();
        if !valid.contains(&2) {
            observation.current_contract.policy_digest = [9; 32];
        }
        if !valid.contains(&3) {
            observation.manifest_digest = [9; 32];
        }
        observation.decision = if !valid.contains(&5) {
            Decision::Expired
        } else if !valid.contains(&4) {
            Decision::Revoked
        } else {
            Decision::Allowed
        };
        let owner = Authority {
            observation: Ok(observation),
            ..owner
        };
        let result = context.check_current_use(
            if valid.contains(&0) {
                &bundle
            } else {
                &replaced
            },
            if valid.contains(&1) { &snap } else { &stale },
            Purpose::Display,
            &owner,
        );
        assert_eq!(
            result.is_ok(),
            case["accepted"].as_bool().unwrap(),
            "{valid:?}"
        );
        assert_eq!(result.is_ok(), valid.len() == 6);
        assert_eq!(
            owner.calls.get(),
            usize::from(valid.contains(&0) && valid.contains(&1))
        );
    }
    assert_eq!(
        seen.len(),
        64,
        "all finite predicate assignments must replay"
    );
}

#[test]
fn current_use_checks_every_observation_and_preserves_typed_errors() {
    let (bundle, snap, query, fact) = source();
    let context = admit(&bundle, &snap, query, fact);
    let mut owner = authority(context.manifest());
    assert_eq!(
        context.check_current_use(&bundle, &snap, Purpose::Display, &owner),
        Ok(())
    );
    for (decision, error) in [
        (Decision::Denied, Error::Denied),
        (Decision::Revoked, Error::Revoked),
        (Decision::Expired, Error::Expired),
    ] {
        owner.observation.as_mut().unwrap().decision = decision;
        assert_eq!(
            context.check_current_use(&bundle, &snap, Purpose::Display, &owner),
            Err(error)
        );
    }
    owner.observation = Err("authority unavailable");
    assert_eq!(
        context.check_current_use(&bundle, &snap, Purpose::Display, &owner),
        Err(Error::Authority("authority unavailable"))
    );
    assert_eq!(owner.calls.get(), 5);
}

#[test]
fn display_observation_and_identical_bytes_cannot_widen_scope_or_authorize_action() {
    let (bundle, snap, query, fact) = source();
    let context = admit(&bundle, &snap, query, fact);
    let presentation = context
        .materialize(
            &bundle,
            &snap,
            AgenticAiContextMaterializationRequest {
                precedence: vec![fact],
                renderer_identity: "fixture".into(),
                segments: vec![AgenticAiContextRenderedElement {
                    element: fact,
                    bytes: b"same bytes".to_vec(),
                }],
            },
        )
        .unwrap();
    let mut owner = authority(presentation.manifest());
    assert_eq!(
        presentation.check_current_use(&bundle, &snap, Purpose::Action, &owner),
        Err(Error::ObservationMismatch)
    );
    let original = owner.observation.clone().unwrap();
    let mut contracts = vec![original.current_contract.clone(); 6];
    contracts[0].actor = EntityId::from_canonical_bytes("another-actor").unwrap();
    contracts[1].task = StateId::from_canonical_bytes("another-task").unwrap();
    contracts[2].policy_digest = [2; 32];
    contracts[3].required.push(fact);
    contracts[4].temporal_receipts.push(fact);
    contracts[5].require_complete = false;
    for contract in contracts {
        owner.observation = Ok(AgenticAiContextUseObservation {
            current_contract: contract,
            ..original.clone()
        });
        assert_eq!(
            presentation.check_current_use(&bundle, &snap, Purpose::Display, &owner),
            Err(Error::ContractMismatch)
        );
    }
    owner.observation = Ok(AgenticAiContextUseObservation {
        decision: Decision::Expired,
        ..original
    });
    assert_eq!(
        presentation.check_current_use(&bundle, &snap, Purpose::Display, &owner),
        Err(Error::Expired)
    );

    #[cfg(feature = "agentic-ai-context-tokens")]
    {
        let tokens = presentation
            .bind_tokens(
                &bundle,
                &snap,
                crate::AgenticAiContextTokenBindingRequest {
                    identity: crate::AgenticAiContextComputationalIdentity {
                        model_weights: "fixture-model".into(),
                        tokenizer: "fixture-tokenizer".into(),
                        renderer: "fixture".into(),
                        chat_template: "fixture-template".into(),
                        position_scheme: "absolute".into(),
                        attention_semantics: "causal".into(),
                        cache_format: "fixture-cache".into(),
                        adapter_digest: [0; 32],
                        multimodal_digest: [0; 32],
                        sharing_scope: [2; 32],
                        block_tokens: std::num::NonZeroUsize::new(2).unwrap(),
                    },
                    tokens: vec![1, 2, 3, 4],
                },
            )
            .unwrap();
        assert_eq!(
            tokens
                .reuse_eligibility_from(&tokens, &bundle, &snap)
                .unwrap()
                .eligible_full_block_tokens(),
            4
        );
        assert_eq!(
            tokens.check_current_use(&bundle, &snap, Purpose::Display, &owner),
            Err(Error::Expired)
        );
        owner.observation.as_mut().unwrap().decision = Decision::Revoked;
        assert_eq!(
            tokens.check_current_use(&bundle, &snap, Purpose::Display, &owner),
            Err(Error::Revoked)
        );
        owner.observation.as_mut().unwrap().decision = Decision::Allowed;
        assert_eq!(
            tokens.check_current_use(&bundle, &snap, Purpose::Display, &owner),
            Ok(())
        );
        let revised = snapshot(snap.generation(), "later");
        assert!(matches!(
            tokens.check_current_use(&bundle, &revised, Purpose::Display, &owner),
            Err(Error::Source(_))
        ));
    }
}
