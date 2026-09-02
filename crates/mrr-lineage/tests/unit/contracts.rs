use crate::{
    Derivation, DerivationId, Fact, FactId, GenerationId, LineageEdge, LineageEdgeId,
    LineageEdgeKind, LineageError, LineageGraph, LineageGraphError, LineageNode, LineageNodeId,
    LineageNodeKind, QueryOperatorId, RuleId, TransitionId, WhyError, impact, why, why_one_witness,
};
use mrr_relation::{
    EvidenceCompleteness, FactProvenance, FactValidity, RelationAuthority, RelationContext,
    RelationId, Value,
};

macro_rules! id {
    ($kind:ident, $value:expr) => {
        $kind::from_canonical_bytes(format!("test:{}:{}", stringify!($kind), $value))
            .expect("test identity")
    };
}

fn derived_fact(
    id: FactId,
    derivation: DerivationId,
    rule: RuleId,
    generation: GenerationId,
) -> Fact {
    Fact::new(
        id,
        RelationId::from_canonical_bytes(b"relation:lineage-test").expect("relation id"),
        vec![Value::Integer(1)],
        RelationContext::new(
            generation,
            RelationAuthority::Rule(rule),
            FactProvenance::Derivation(derivation),
            EvidenceCompleteness::Complete,
            FactValidity::Valid,
        ),
    )
}
#[test]
fn lineage_rejects_self_support() {
    let fact_id = FactId::from_canonical_bytes(b"fact:self-support").expect("fact id");
    let generation =
        GenerationId::from_canonical_bytes(b"generation:lineage-test").expect("generation id");
    let derivation_id =
        DerivationId::from_canonical_bytes(b"derivation:self-support").expect("derivation id");
    let rule = RuleId::from_canonical_bytes(b"rule:lineage-test").expect("rule id");
    let output = derived_fact(fact_id, derivation_id, rule, generation);
    assert_eq!(
        Derivation::new(derivation_id, rule, generation, output, vec![fact_id],),
        Err(LineageError::SelfSupport)
    );
}

#[test]
fn derivation_requires_exact_context_and_unique_support() {
    let output_id = id!(FactId, "output");
    let support = id!(FactId, "support");
    let derivation = id!(DerivationId, "derivation");
    let rule = id!(RuleId, "rule");
    let generation = id!(GenerationId, "generation");
    let output = derived_fact(output_id, derivation, rule, generation);

    assert_eq!(
        Derivation::new(
            derivation,
            rule,
            generation,
            output.clone(),
            vec![support, support],
        ),
        Err(LineageError::DuplicateSupport(support))
    );
    assert_eq!(
        Derivation::new(
            derivation,
            rule,
            id!(GenerationId, "other"),
            output,
            vec![support],
        ),
        Err(LineageError::GenerationMismatch {
            expected: id!(GenerationId, "other"),
            actual: generation,
        })
    );
}

#[test]
fn admits_every_unified_lineage_node_and_edge_kind() {
    let nodes = vec![
        LineageNode::new(
            id!(LineageNodeId, 1),
            LineageNodeKind::Result(id!(FactId, 1)),
        ),
        LineageNode::new(
            id!(LineageNodeId, 2),
            LineageNodeKind::Projection(id!(QueryOperatorId, 2)),
        ),
        LineageNode::new(
            id!(LineageNodeId, 3),
            LineageNodeKind::QueryOperator(id!(QueryOperatorId, 3)),
        ),
        LineageNode::new(
            id!(LineageNodeId, 4),
            LineageNodeKind::DerivedFact(id!(FactId, 4)),
        ),
        LineageNode::new(
            id!(LineageNodeId, 5),
            LineageNodeKind::RuleApplication(id!(DerivationId, 5)),
        ),
        LineageNode::new(
            id!(LineageNodeId, 6),
            LineageNodeKind::Transition(id!(TransitionId, 6)),
        ),
        LineageNode::new(
            id!(LineageNodeId, 7),
            LineageNodeKind::SourceFact(id!(FactId, 7)),
        ),
    ];
    let edges = vec![
        LineageEdge::new(
            id!(LineageEdgeId, 1),
            id!(LineageNodeId, 1),
            id!(LineageNodeId, 2),
            LineageEdgeKind::SelectedBy,
        ),
        LineageEdge::new(
            id!(LineageEdgeId, 2),
            id!(LineageNodeId, 2),
            id!(LineageNodeId, 3),
            LineageEdgeKind::DependsOn,
        ),
        LineageEdge::new(
            id!(LineageEdgeId, 3),
            id!(LineageNodeId, 3),
            id!(LineageNodeId, 4),
            LineageEdgeKind::DerivedFrom,
        ),
        LineageEdge::new(
            id!(LineageEdgeId, 4),
            id!(LineageNodeId, 4),
            id!(LineageNodeId, 5),
            LineageEdgeKind::ProducedBy,
        ),
        LineageEdge::new(
            id!(LineageEdgeId, 5),
            id!(LineageNodeId, 5),
            id!(LineageNodeId, 6),
            LineageEdgeKind::InvalidatedBy,
        ),
        LineageEdge::new(
            id!(LineageEdgeId, 6),
            id!(LineageNodeId, 6),
            id!(LineageNodeId, 7),
            LineageEdgeKind::DependsOn,
        ),
    ];

    let graph = LineageGraph::admit(nodes, edges).expect("finite acyclic lineage graph");
    assert_eq!(graph.nodes().len(), 7);
    assert_eq!(graph.edges().len(), 6);
}

#[test]
fn lineage_graph_fails_closed_on_missing_endpoints_and_cycles() {
    let left = LineageNode::new(
        id!(LineageNodeId, "left"),
        LineageNodeKind::SourceFact(id!(FactId, "left")),
    );
    let right = LineageNode::new(
        id!(LineageNodeId, "right"),
        LineageNodeKind::DerivedFact(id!(FactId, "right")),
    );
    let missing = id!(LineageNodeId, "missing");
    assert_eq!(
        LineageGraph::admit(
            vec![left],
            vec![LineageEdge::new(
                id!(LineageEdgeId, "missing"),
                left.id(),
                missing,
                LineageEdgeKind::DependsOn,
            )],
        ),
        Err(LineageGraphError::MissingEndpoint(missing))
    );

    assert_eq!(
        LineageGraph::admit(
            vec![left, right],
            vec![
                LineageEdge::new(
                    id!(LineageEdgeId, 1),
                    left.id(),
                    right.id(),
                    LineageEdgeKind::DependsOn,
                ),
                LineageEdge::new(
                    id!(LineageEdgeId, 2),
                    right.id(),
                    left.id(),
                    LineageEdgeKind::DependsOn,
                ),
            ],
        ),
        Err(LineageGraphError::Cycle)
    );
}

#[test]
fn why_preserves_all_derivations_unless_one_witness_is_explicit() {
    let result = LineageNode::new(
        id!(LineageNodeId, "result"),
        LineageNodeKind::Result(id!(FactId, "result")),
    );
    let path_a = LineageNode::new(
        id!(LineageNodeId, "path-a"),
        LineageNodeKind::RuleApplication(id!(DerivationId, "path-a")),
    );
    let path_b = LineageNode::new(
        id!(LineageNodeId, "path-b"),
        LineageNodeKind::RuleApplication(id!(DerivationId, "path-b")),
    );
    let source_a = LineageNode::new(
        id!(LineageNodeId, "source-a"),
        LineageNodeKind::SourceFact(id!(FactId, "source-a")),
    );
    let source_b = LineageNode::new(
        id!(LineageNodeId, "source-b"),
        LineageNodeKind::SourceFact(id!(FactId, "source-b")),
    );
    let graph = LineageGraph::admit(
        vec![result, path_a, path_b, source_a, source_b],
        vec![
            LineageEdge::new(
                id!(LineageEdgeId, "result-a"),
                result.id(),
                path_a.id(),
                LineageEdgeKind::ProducedBy,
            ),
            LineageEdge::new(
                id!(LineageEdgeId, "result-b"),
                result.id(),
                path_b.id(),
                LineageEdgeKind::ProducedBy,
            ),
            LineageEdge::new(
                id!(LineageEdgeId, "path-a-source"),
                path_a.id(),
                source_a.id(),
                LineageEdgeKind::DerivedFrom,
            ),
            LineageEdge::new(
                id!(LineageEdgeId, "path-b-source"),
                path_b.id(),
                source_b.id(),
                LineageEdgeKind::DerivedFrom,
            ),
        ],
    )
    .expect("two explanation paths");

    let all = why(&graph, result.id()).expect("all witnesses");
    assert_eq!(all.root(), result.id());
    assert_eq!(all.graph().nodes().len(), 5);
    assert_eq!(all.graph().edges().len(), 4);

    let one = why_one_witness(&graph, result.id()).expect("explicit one witness");
    assert_eq!(one.graph().nodes().len(), 3);
    assert_eq!(one.graph().edges().len(), 2);

    let missing = id!(LineageNodeId, "unknown");
    assert_eq!(why(&graph, missing), Err(WhyError::UnknownResult(missing)));
}

#[test]
fn impact_classifies_direct_transitive_and_unaffected_nodes() {
    let changed_fact = id!(FactId, "changed");
    let source = LineageNode::new(
        id!(LineageNodeId, "source"),
        LineageNodeKind::SourceFact(changed_fact),
    );
    let application = LineageNode::new(
        id!(LineageNodeId, "application"),
        LineageNodeKind::RuleApplication(id!(DerivationId, "application")),
    );
    let derived = LineageNode::new(
        id!(LineageNodeId, "derived"),
        LineageNodeKind::DerivedFact(id!(FactId, "derived")),
    );
    let result = LineageNode::new(
        id!(LineageNodeId, "result"),
        LineageNodeKind::Result(id!(FactId, "result")),
    );
    let unaffected = LineageNode::new(
        id!(LineageNodeId, "unaffected"),
        LineageNodeKind::SourceFact(id!(FactId, "unaffected")),
    );
    let graph = LineageGraph::admit(
        vec![source, application, derived, result, unaffected],
        vec![
            LineageEdge::new(
                id!(LineageEdgeId, 1),
                application.id(),
                source.id(),
                LineageEdgeKind::DependsOn,
            ),
            LineageEdge::new(
                id!(LineageEdgeId, 2),
                derived.id(),
                application.id(),
                LineageEdgeKind::ProducedBy,
            ),
            LineageEdge::new(
                id!(LineageEdgeId, 3),
                result.id(),
                derived.id(),
                LineageEdgeKind::DerivedFrom,
            ),
        ],
    )
    .expect("impact fixture");

    let impact = impact(&graph, changed_fact).expect("known source fact");
    assert_eq!(impact.source(), source.id());
    assert_eq!(impact.directly_invalidated(), &[application.id()]);
    let transitive: std::collections::BTreeSet<_> =
        impact.transitively_invalidated().iter().copied().collect();
    assert_eq!(
        transitive,
        [derived.id(), result.id()].into_iter().collect()
    );
    assert_eq!(impact.unaffected(), &[unaffected.id()]);
}
