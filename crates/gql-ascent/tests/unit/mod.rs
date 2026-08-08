use crate::AscentTransitiveClosure;
use gql_reasoning::DerivationLimits;
use gql_reasoning::DerivedRelationProvider;
use gql_reasoning::RelationName;

#[test]
fn derives_transitive_relation_with_snapshot_witness() {
    let provider = AscentTransitiveClosure::new("CALLS", "DEPENDS_TRANSITIVELY", "dependency.v1");
    let calls = RelationName("CALLS".into());
    let facts = vec![
        gql_reasoning::Fact {
            predicate: calls.clone(),
            values: vec![
                gql_types::Value::String("A".into()),
                gql_types::Value::String("B".into()),
            ],
        },
        gql_reasoning::Fact {
            predicate: calls,
            values: vec![
                gql_types::Value::String("B".into()),
                gql_types::Value::String("C".into()),
            ],
        },
    ];
    let result = provider
        .derive(gql_reasoning::DerivationRequest {
            predicate: &RelationName("DEPENDS_TRANSITIVELY".into()),
            bindings: &[None, None],
            facts: &facts,
            snapshot: "snapshot-1",
            limits: DerivationLimits::default(),
        })
        .expect("registered predicate derives");
    assert!(result.tuples.iter().any(|tuple| tuple.values
        == vec![
            gql_types::Value::String("A".into()),
            gql_types::Value::String("C".into())
        ]));
    assert!(
        result
            .tuples
            .iter()
            .all(|tuple| tuple.witness.snapshot == "snapshot-1")
    );
}

#[test]
fn derives_transitive_relation_with_truncated_output_when_limit_hits() {
    let provider = AscentTransitiveClosure::new("CALLS", "DEPENDS_TRANSITIVELY", "dependency.v1");
    let calls = RelationName("CALLS".into());
    let facts = vec![
        gql_reasoning::Fact {
            predicate: calls.clone(),
            values: vec![
                gql_types::Value::String("A".into()),
                gql_types::Value::String("B".into()),
            ],
        },
        gql_reasoning::Fact {
            predicate: calls.clone(),
            values: vec![
                gql_types::Value::String("B".into()),
                gql_types::Value::String("C".into()),
            ],
        },
        gql_reasoning::Fact {
            predicate: calls,
            values: vec![
                gql_types::Value::String("A".into()),
                gql_types::Value::String("D".into()),
            ],
        },
    ];

    let result = provider
        .derive(gql_reasoning::DerivationRequest {
            predicate: &RelationName("DEPENDS_TRANSITIVELY".into()),
            bindings: &[None, None],
            facts: &facts,
            snapshot: "snapshot-1",
            limits: DerivationLimits {
                max_results: 1,
                ..DerivationLimits::default()
            },
        })
        .expect("registered predicate derives");

    assert_eq!(result.tuples.len(), 1);
    assert!(matches!(
        result.closure,
        gql_reasoning::ClosureStatus::OutputTruncated
    ));
}

#[test]
fn derives_budget_exhaustion_when_request_exceeds_budget() {
    let provider = AscentTransitiveClosure::new("CALLS", "DEPENDS_TRANSITIVELY", "dependency.v1");
    let calls = RelationName("CALLS".into());
    let facts = vec![gql_reasoning::Fact {
        predicate: calls,
        values: vec![
            gql_types::Value::String("A".into()),
            gql_types::Value::String("B".into()),
        ],
    }];

    let result = provider
        .derive(gql_reasoning::DerivationRequest {
            predicate: &RelationName("DEPENDS_TRANSITIVELY".into()),
            bindings: &[None, None],
            facts: &facts,
            snapshot: "snapshot-1",
            limits: DerivationLimits {
                max_rule_firings: 0,
                ..DerivationLimits::default()
            },
        })
        .expect("registered predicate derives");

    assert!(matches!(
        result.closure,
        gql_reasoning::ClosureStatus::BudgetExhausted
    ));
}
