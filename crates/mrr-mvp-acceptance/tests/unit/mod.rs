use core::num::NonZeroUsize;

use meta_relational_reasoning::{
    Action, ActionId, Atom, Binding, CandidateIdentities, ClosureStatus, DeductionLimits,
    DeductionPlan, DerivationId, Direction, Effect, EntityId, EvidenceCompleteness, Expression,
    ExternalRevisionIdentity, Fact, FactId, FactProvenance, FactValidity, GenerationId,
    GraphPattern, GroundAtom, InitialState, IntentBindingStatus, IntentBundleBinding,
    IntentSemanticModel, Invariant, LineageEdge, LineageEdgeId, LineageEdgeKind, LineageGraph,
    LineageNode, LineageNodeId, LineageNodeKind, MetaQueryIr, MrrEngine, NodePattern, PathPattern,
    PathSegment, Precondition, Projection, QueryId, QueryOperatorId, QueryTemplate,
    ReasoningBundle, ReasoningBundleDeclaration, RelationAuthority, RelationCardinality,
    RelationContext, RelationField, RelationId, RelationPattern, RelationSchema, RevisionBinding,
    Rule, RuleId, RulePack, RulePackId, SafetyLimits, SafetyStatus, StatePredicate, StateSchema,
    StateSnapshot, Term, TransitionSystem, Value, ValueType, Variable, WhyNotLimits, WhyNotStatus,
};

macro_rules! id {
    ($kind:ident, $domain:expr, $value:expr) => {
        $kind::from_canonical_bytes(format!(
            "acceptance:{}:{}:{}",
            stringify!($kind),
            $domain,
            $value
        ))
        .expect("acceptance identity")
    };
}

struct Domain<'a> {
    name: &'a str,
    intent: &'a str,
    left: &'a str,
    middle: &'a str,
    right: &'a str,
    missing: &'a str,
}

fn variable(name: &str) -> Variable {
    Variable::new(name).expect("variable")
}

fn atom(relation: RelationId, variables: &[&str]) -> Atom {
    Atom {
        relation,
        terms: variables
            .iter()
            .map(|name| Term::Variable(variable(name)))
            .collect(),
    }
}

fn ground(relation: RelationId, value: &str) -> Atom {
    Atom {
        relation,
        terms: vec![Term::Value(Value::String(value.into()))],
    }
}

fn schema(relation: RelationId, name: &str, arity: usize) -> RelationSchema {
    RelationSchema::new(
        relation,
        name,
        (0..arity)
            .map(|index| RelationField::new(format!("field_{index}"), ValueType::String).unwrap())
            .collect(),
        RelationCardinality::ManyToMany,
    )
    .unwrap()
}

fn fact(
    domain: &str,
    index: usize,
    relation: RelationId,
    values: &[&str],
    generation: GenerationId,
) -> Fact {
    let authority = id!(EntityId, domain, index);
    Fact::new(
        id!(FactId, domain, index),
        relation,
        values
            .iter()
            .map(|value| Value::String((*value).into()))
            .collect(),
        RelationContext::new(
            generation,
            RelationAuthority::Entity(authority),
            FactProvenance::Source(authority),
            EvidenceCompleteness::Complete,
            FactValidity::Valid,
        ),
    )
}

fn query(domain: &str, relation: RelationId) -> MetaQueryIr {
    let left = Binding::new("left").unwrap();
    let right = Binding::new("right").unwrap();
    let graph = GraphPattern::new(
        id!(QueryOperatorId, domain, "graph"),
        vec![PathPattern::new(
            NodePattern::new(left.clone(), vec![]),
            vec![PathSegment::new(
                RelationPattern::new(None, vec![relation], Direction::Outgoing, 1, Some(1))
                    .unwrap(),
                NodePattern::new(right.clone(), vec![]),
            )],
        )],
    )
    .unwrap();
    MetaQueryIr::new(
        id!(QueryId, domain, "query"),
        graph,
        vec![],
        vec![Projection::new(
            id!(QueryOperatorId, domain, "projection"),
            Expression::Binding(right.clone()),
            right,
        )],
        vec![],
        vec![],
        None,
    )
    .unwrap()
}

fn execute(domain: Domain<'_>) {
    let edge = id!(RelationId, domain.name, "edge");
    let reachable = id!(RelationId, domain.name, "reachable");
    let permitted = id!(RelationId, domain.name, "permitted");
    let prerequisite = id!(RelationId, domain.name, "prerequisite");
    let generation = id!(GenerationId, domain.name, 1);
    let base = id!(RuleId, domain.name, "base");
    let transitive = id!(RuleId, domain.name, "transitive");
    let rule_pack = id!(RulePackId, domain.name, "closure");
    let missing_rule = Rule::new(
        id!(RuleId, domain.name, "missing"),
        atom(permitted, &["x"]),
        vec![atom(prerequisite, &["x"])],
    )
    .unwrap();
    let closure_rules = vec![
        Rule::new(
            base,
            atom(reachable, &["x", "y"]),
            vec![atom(edge, &["x", "y"])],
        )
        .unwrap(),
        Rule::new(
            transitive,
            atom(reachable, &["x", "z"]),
            vec![atom(reachable, &["x", "y"]), atom(edge, &["y", "z"])],
        )
        .unwrap(),
    ];
    let bundle = ReasoningBundle::admit(ReasoningBundleDeclaration {
        relations: vec![
            schema(edge, "edge", 2),
            schema(reachable, "reachable", 2),
            schema(permitted, "permitted", 1),
            schema(prerequisite, "prerequisite", 1),
        ],
        facts: vec![
            fact(
                domain.name,
                1,
                edge,
                &[domain.left, domain.middle],
                generation,
            ),
            fact(
                domain.name,
                2,
                edge,
                &[domain.middle, domain.right],
                generation,
            ),
        ],
        query_templates: vec![QueryTemplate::new(query(domain.name, edge), vec![])],
        rule_packs: vec![
            RulePack::new(rule_pack, closure_rules),
            RulePack::new(id!(RulePackId, domain.name, "why-not"), vec![missing_rule]),
        ],
        ..ReasoningBundleDeclaration::default()
    })
    .expect("domain bundle");
    let engine = MrrEngine::builder()
        .with_bundle(bundle)
        .build()
        .expect("domain engine");
    assert_eq!(
        engine
            .query(id!(QueryId, domain.name, "query"))
            .unwrap()
            .referenced_relations(),
        vec![edge]
    );

    let closure = engine
        .derive(
            DeductionPlan::transitive_closure(edge, reachable, rule_pack, base, transitive),
            generation,
            DeductionLimits::new(
                NonZeroUsize::new(8).unwrap(),
                NonZeroUsize::new(16).unwrap(),
                NonZeroUsize::new(16).unwrap(),
            ),
        )
        .expect("bounded closure");
    assert_eq!(closure.status(), ClosureStatus::Complete);
    assert!(closure.candidates().iter().any(|candidate| {
        candidate.values()
            == &[
                Value::String(domain.left.into()),
                Value::String(domain.right.into()),
            ]
    }));
    let identities: Vec<_> = closure
        .candidates()
        .iter()
        .enumerate()
        .map(|(index, _)| {
            CandidateIdentities::new(
                id!(FactId, domain.name, 100 + index),
                id!(DerivationId, domain.name, 200 + index),
            )
        })
        .collect();
    let materialized = engine
        .materialize(
            &closure,
            id!(GenerationId, domain.name, 0),
            generation,
            &identities,
        )
        .expect("atomic materialization");
    assert_eq!(materialized.derivations().len(), closure.candidates().len());

    let derivation = &materialized.derivations()[0];
    let result_node = id!(LineageNodeId, domain.name, "result");
    let application_node = id!(LineageNodeId, domain.name, "application");
    let source_nodes: Vec<_> = derivation
        .support()
        .iter()
        .enumerate()
        .map(|(index, _)| id!(LineageNodeId, domain.name, 10 + index))
        .collect();
    let mut nodes = vec![
        LineageNode::new(
            result_node,
            LineageNodeKind::Result(derivation.output().id()),
        ),
        LineageNode::new(
            application_node,
            LineageNodeKind::RuleApplication(derivation.id()),
        ),
    ];
    nodes.extend(
        derivation
            .support()
            .iter()
            .zip(&source_nodes)
            .map(|(fact, node)| LineageNode::new(*node, LineageNodeKind::SourceFact(*fact))),
    );
    let mut edges = vec![LineageEdge::new(
        id!(LineageEdgeId, domain.name, 0),
        result_node,
        application_node,
        LineageEdgeKind::ProducedBy,
    )];
    edges.extend(source_nodes.iter().enumerate().map(|(index, node)| {
        LineageEdge::new(
            id!(LineageEdgeId, domain.name, index + 1),
            application_node,
            *node,
            LineageEdgeKind::DerivedFrom,
        )
    }));
    let lineage = LineageGraph::admit(nodes, edges).expect("lineage");
    assert_eq!(engine.why(&lineage, result_node).unwrap().graph(), &lineage);
    let impact = engine
        .impact(&lineage, derivation.support()[0])
        .expect("impact");
    assert_eq!(impact.directly_invalidated(), &[application_node]);
    assert!(impact.transitively_invalidated().contains(&result_node));

    let missing = engine
        .why_not(
            &ground(permitted, domain.missing),
            generation,
            WhyNotLimits::new(
                NonZeroUsize::new(8).unwrap(),
                NonZeroUsize::new(16).unwrap(),
                NonZeroUsize::new(8).unwrap(),
            ),
        )
        .unwrap();
    assert_eq!(
        missing.status(),
        &WhyNotStatus::MissingPremises {
            alternatives: vec![vec![GroundAtom::new(
                prerequisite,
                vec![Value::String(domain.missing.into())],
            )]],
        }
    );

    let active = id!(FactId, domain.name, "active-state");
    let invalid = id!(FactId, domain.name, "invalid-state");
    let system = TransitionSystem::admit(
        StateSchema::new(vec![active, invalid]).unwrap(),
        InitialState::new(StateSnapshot::from_facts(vec![active]).unwrap()),
        vec![Action::new(
            id!(ActionId, domain.name, "invalid-transition"),
            Precondition::all(vec![StatePredicate::Present(active)]),
            Effect::new(vec![invalid], vec![]).unwrap(),
        )],
        vec![Invariant::forbidden_all("domain-safety", vec![active, invalid]).unwrap()],
    )
    .unwrap();
    assert_eq!(
        engine
            .check_safety(
                &system,
                SafetyLimits::new(NonZeroUsize::new(8).unwrap(), NonZeroUsize::new(8).unwrap(),),
            )
            .unwrap()
            .status(),
        SafetyStatus::Unsafe
    );

    let revision = RevisionBinding::admit(
        ExternalRevisionIdentity::new(domain.name, "logical-change", "content-1").unwrap(),
        generation,
    )
    .unwrap();
    assert_eq!(revision.generation(), generation);
    let intent = IntentSemanticModel::project_org(domain.intent).expect("domain intent");
    let intent_binding = IntentBundleBinding::select(&intent, engine.bundle().id());
    assert_eq!(intent_binding.status(&intent), IntentBindingStatus::Current);
}

#[test]
fn software_lifecycle_uses_the_shared_kernel() {
    execute(Domain {
        name: "software",
        intent: include_str!("../../../../fixtures/software/runtime-lifecycle.org"),
        left: "RuntimeAdmission",
        middle: "worker_threads",
        right: "GenerationIdentity",
        missing: "log_level",
    });
}

#[test]
fn knowledge_provenance_uses_the_shared_kernel() {
    execute(Domain {
        name: "knowledge",
        intent: include_str!("../../../../fixtures/knowledge/claim-evidence.org"),
        left: "EvidenceA",
        middle: "ClaimX",
        right: "AcceptedClaimX",
        missing: "ClaimY",
    });
}

#[test]
fn workflow_policy_uses_the_shared_kernel() {
    execute(Domain {
        name: "workflow",
        intent: include_str!("../../../../fixtures/workflow/policy-transition.org"),
        left: "PolicyA",
        middle: "TransitionT",
        right: "ApprovalX",
        missing: "TransitionWithoutApproval",
    });
}
