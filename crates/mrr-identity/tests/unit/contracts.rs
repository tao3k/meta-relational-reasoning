use std::str::FromStr;

use serde::Deserialize;

use crate::{
    ActionId, DerivationId, EntityId, FactId, GenerationId, IdentityDomain, LineageEdgeId,
    LineageNodeId, QueryId, QueryOperatorId, ReasoningBundleId, RelationId, RevisionId, RuleId,
    RulePackId, StateId, TransitionId,
};

const CANONICAL_INPUT: &[u8] = b"depends_on";

#[derive(Debug, Deserialize)]
struct GoldenIdentities {
    entity: EntityId,
    relation: RelationId,
    fact: FactId,
    rule: RuleId,
    derivation: DerivationId,
    query: QueryId,
    query_operator: QueryOperatorId,
    state: StateId,
    transition: TransitionId,
    action: ActionId,
    generation: GenerationId,
    revision: RevisionId,
    rule_pack: RulePackId,
    lineage_node: LineageNodeId,
    lineage_edge: LineageEdgeId,
    reasoning_bundle: ReasoningBundleId,
}

#[derive(Debug, Deserialize)]
struct GoldenFixture {
    schema: String,
    canonical_input_utf8: String,
    identities: GoldenIdentities,
}

#[test]
fn canonical_semantic_input_derives_stable_domain_separated_identities() {
    let relation = RelationId::from_canonical_bytes(CANONICAL_INPUT).expect("canonical input");
    assert_eq!(
        relation,
        RelationId::from_canonical_bytes(CANONICAL_INPUT).expect("same canonical input")
    );

    let encoded = [
        EntityId::from_canonical_bytes(CANONICAL_INPUT)
            .expect("entity")
            .to_string(),
        relation.to_string(),
        FactId::from_canonical_bytes(CANONICAL_INPUT)
            .expect("fact")
            .to_string(),
        RuleId::from_canonical_bytes(CANONICAL_INPUT)
            .expect("rule")
            .to_string(),
        DerivationId::from_canonical_bytes(CANONICAL_INPUT)
            .expect("derivation")
            .to_string(),
        QueryId::from_canonical_bytes(CANONICAL_INPUT)
            .expect("query")
            .to_string(),
        QueryOperatorId::from_canonical_bytes(CANONICAL_INPUT)
            .expect("query operator")
            .to_string(),
        StateId::from_canonical_bytes(CANONICAL_INPUT)
            .expect("state")
            .to_string(),
        TransitionId::from_canonical_bytes(CANONICAL_INPUT)
            .expect("transition")
            .to_string(),
        ActionId::from_canonical_bytes(CANONICAL_INPUT)
            .expect("action")
            .to_string(),
        GenerationId::from_canonical_bytes(CANONICAL_INPUT)
            .expect("generation")
            .to_string(),
        RevisionId::from_canonical_bytes(CANONICAL_INPUT)
            .expect("revision")
            .to_string(),
        RulePackId::from_canonical_bytes(CANONICAL_INPUT)
            .expect("rule pack")
            .to_string(),
        LineageNodeId::from_canonical_bytes(CANONICAL_INPUT)
            .expect("lineage node")
            .to_string(),
        LineageEdgeId::from_canonical_bytes(CANONICAL_INPUT)
            .expect("lineage edge")
            .to_string(),
        ReasoningBundleId::from_canonical_bytes(CANONICAL_INPUT)
            .expect("reasoning bundle")
            .to_string(),
    ];
    let unique = encoded.iter().collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique.len(), encoded.len());
}

#[test]
fn identity_domains_are_explicit_and_complete() {
    assert_eq!(EntityId::DOMAIN, IdentityDomain::Entity);
    assert_eq!(RelationId::DOMAIN, IdentityDomain::Relation);
    assert_eq!(FactId::DOMAIN, IdentityDomain::Fact);
    assert_eq!(RuleId::DOMAIN, IdentityDomain::Rule);
    assert_eq!(DerivationId::DOMAIN, IdentityDomain::Derivation);
    assert_eq!(QueryId::DOMAIN, IdentityDomain::Query);
    assert_eq!(QueryOperatorId::DOMAIN, IdentityDomain::QueryOperator);
    assert_eq!(StateId::DOMAIN, IdentityDomain::State);
    assert_eq!(TransitionId::DOMAIN, IdentityDomain::Transition);
    assert_eq!(ActionId::DOMAIN, IdentityDomain::Action);
    assert_eq!(GenerationId::DOMAIN, IdentityDomain::Generation);
    assert_eq!(RevisionId::DOMAIN, IdentityDomain::Revision);
    assert_eq!(RulePackId::DOMAIN, IdentityDomain::RulePack);
    assert_eq!(LineageNodeId::DOMAIN, IdentityDomain::LineageNode);
    assert_eq!(LineageEdgeId::DOMAIN, IdentityDomain::LineageEdge);
    assert_eq!(ReasoningBundleId::DOMAIN, IdentityDomain::ReasoningBundle);
}

#[test]
fn text_and_json_round_trip_preserve_the_typed_domain() {
    let identity = RelationId::from_canonical_bytes(CANONICAL_INPUT).expect("relation");
    let encoded = identity.to_string();
    assert_eq!(RelationId::from_str(&encoded), Ok(identity));

    let json = serde_json::to_string(&identity).expect("serialize identity");
    assert_eq!(json, format!("\"{encoded}\""));
    assert_eq!(
        serde_json::from_str::<RelationId>(&json).expect("deserialize identity"),
        identity
    );
    assert!(EntityId::from_str(&encoded).is_err());
}

#[test]
fn empty_or_noncanonical_identity_inputs_fail_closed() {
    assert!(RelationId::from_canonical_bytes([]).is_err());
    assert!(RelationId::from_str("relation:1234").is_err());
    assert!(RelationId::from_str("mrr:relation:v1:1234").is_err());
    assert!(
        RelationId::from_str(
            "mrr:relation:v2:fb20e2c5b1cd7b4a575a0f1fdce0e00c444fab69698ed8c9da105654e0ccd4f7"
        )
        .is_err()
    );
    assert!(
        RelationId::from_str(
            "mrr:relation:v1:FB20E2C5B1CD7B4A575A0F1FDCE0E00C444FAB69698ED8C9DA105654E0CCD4F7"
        )
        .is_err()
    );
    assert!(
        RelationId::from_str(
            "mrr:relation:v1:fb20e2c5b1cd7b4a575a0f1fdce0e00c444fab69698ed8c9da105654e0ccd4f7:extra"
        )
        .is_err()
    );
}

#[test]
fn version_one_golden_fixture_freezes_every_identity_domain() {
    let fixture: GoldenFixture =
        serde_json::from_str(include_str!("../../../../fixtures/identity/v1.json"))
            .expect("valid golden fixture");
    assert_eq!(fixture.schema, "mrr.identity.golden.v1");
    assert_eq!(fixture.canonical_input_utf8.as_bytes(), CANONICAL_INPUT);

    assert_eq!(
        fixture.identities.entity,
        EntityId::from_canonical_bytes(CANONICAL_INPUT).expect("entity")
    );
    assert_eq!(
        fixture.identities.relation,
        RelationId::from_canonical_bytes(CANONICAL_INPUT).expect("relation")
    );
    assert_eq!(
        fixture.identities.fact,
        FactId::from_canonical_bytes(CANONICAL_INPUT).expect("fact")
    );
    assert_eq!(
        fixture.identities.rule,
        RuleId::from_canonical_bytes(CANONICAL_INPUT).expect("rule")
    );
    assert_eq!(
        fixture.identities.derivation,
        DerivationId::from_canonical_bytes(CANONICAL_INPUT).expect("derivation")
    );
    assert_eq!(
        fixture.identities.query,
        QueryId::from_canonical_bytes(CANONICAL_INPUT).expect("query")
    );
    assert_eq!(
        fixture.identities.query_operator,
        QueryOperatorId::from_canonical_bytes(CANONICAL_INPUT).expect("query operator")
    );
    assert_eq!(
        fixture.identities.state,
        StateId::from_canonical_bytes(CANONICAL_INPUT).expect("state")
    );
    assert_eq!(
        fixture.identities.transition,
        TransitionId::from_canonical_bytes(CANONICAL_INPUT).expect("transition")
    );
    assert_eq!(
        fixture.identities.action,
        ActionId::from_canonical_bytes(CANONICAL_INPUT).expect("action")
    );
    assert_eq!(
        fixture.identities.generation,
        GenerationId::from_canonical_bytes(CANONICAL_INPUT).expect("generation")
    );
    assert_eq!(
        fixture.identities.revision,
        RevisionId::from_canonical_bytes(CANONICAL_INPUT).expect("revision")
    );
    assert_eq!(
        fixture.identities.rule_pack,
        RulePackId::from_canonical_bytes(CANONICAL_INPUT).expect("rule pack")
    );
    assert_eq!(
        fixture.identities.lineage_node,
        LineageNodeId::from_canonical_bytes(CANONICAL_INPUT).expect("lineage node")
    );
    assert_eq!(
        fixture.identities.lineage_edge,
        LineageEdgeId::from_canonical_bytes(CANONICAL_INPUT).expect("lineage edge")
    );
    assert_eq!(
        fixture.identities.reasoning_bundle,
        ReasoningBundleId::from_canonical_bytes(CANONICAL_INPUT).expect("reasoning bundle")
    );
}
