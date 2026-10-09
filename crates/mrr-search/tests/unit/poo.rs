use crate::*;
use mrr_identity::{FactId, GenerationId};
use std::num::NonZeroUsize;
fn stage(name: &str, role: PooSearchRole, input: &str, output: &str) -> PooSearchPlan {
    PooSearchPlan::Stage {
        name: name.into(),
        role,
        input_domain: input.into(),
        output_domain: output.into(),
    }
}
fn generation() -> GenerationId {
    GenerationId::from_canonical_bytes(b"poo-source-one").unwrap()
}
#[test]
fn actual_poo_chain_drives_mrr_inference() {
    let plan = PooSearchPlan::Chain {
        name: "chain".into(),
        children: vec![
            stage(
                "source",
                PooSearchRole::Acquisition,
                "workspace",
                "candidates",
            ),
            stage("rank", PooSearchRole::Refinement, "candidates", "ranked"),
        ],
    };
    let projection = compile_poo_search_plan("actual-poo", generation(), &plan).unwrap();
    assert!(!projection.dag_receipt().is_empty());
    let roles = projection
        .role_evidence()
        .expect("actual POO role object evidence");
    assert_eq!(roles.roots.len(), projection.factors().len());
    assert_eq!(roles.runtime_orders.len(), projection.factors().len());
    for (input, root) in &roles.roots {
        assert!(projection.factor_by_input(input).is_some());
        let actual = &roles
            .runtime_orders
            .iter()
            .find(|(factor, _)| factor == input)
            .unwrap()
            .1;
        assert_eq!(actual.first(), Some(root));
        assert!(actual.len() > 1, "actual inherited roles are retained");
        assert!(
            actual
                .iter()
                .all(|id| roles.graph.iter().any(|(name, _)| name == id))
        );
    }
    let source = projection.factor_by_name("source").unwrap();
    let rank = projection.factor_by_name("rank").unwrap();
    assert_eq!(
        projection.edges(),
        [SearchFactorEdge::new(source.id(), rank.id())]
    );
    let event = FactId::from_canonical_bytes(b"event").unwrap();
    let candidate = FactId::from_canonical_bytes(b"candidate").unwrap();
    let observation =
        SearchObservation::new(event, candidate, source.id(), generation(), 0, vec![]);
    let bound = NonZeroUsize::new(8).unwrap();
    let receipt = evaluate_poo_search_factors(
        &projection,
        std::slice::from_ref(&observation),
        SearchFrameworkLimits::new(bound, bound, bound, bound, bound),
    )
    .unwrap();
    let matched = evaluate_search_factors(
        generation(),
        projection.factors(),
        projection.edges(),
        &[observation],
        SearchFrameworkLimits::new(bound, bound, bound, bound, bound),
    )
    .unwrap();
    assert_eq!(receipt.influences(), matched.influences());
    assert_eq!(receipt.digest(), matched.digest());
    assert!(
        receipt
            .influences()
            .iter()
            .any(|row| row.factor() == rank.id())
    );
}
#[test]
fn actual_poo_rejects_domain_mismatch_and_duplicate_stage_names() {
    for children in [
        vec![
            stage("a", PooSearchRole::Acquisition, "workspace", "candidates"),
            stage("b", PooSearchRole::Refinement, "different", "ranked"),
        ],
        vec![
            stage("a", PooSearchRole::Acquisition, "workspace", "candidates"),
            stage("a", PooSearchRole::Refinement, "candidates", "ranked"),
        ],
    ] {
        let plan = PooSearchPlan::Chain {
            name: "chain".into(),
            children,
        };
        assert!(compile_poo_search_plan("actual-poo", generation(), &plan).is_err());
    }
}
#[test]
fn parallel_has_no_cross_branch_edge_and_merge_uses_both_actual_outputs() {
    let parallel = PooSearchPlan::Parallel {
        name: "branches".into(),
        children: vec![
            stage("a", PooSearchRole::Acquisition, "workspace", "lexical"),
            stage("b", PooSearchRole::Acquisition, "workspace", "structural"),
        ],
    };
    assert!(
        compile_poo_search_plan("parallel", generation(), &parallel)
            .unwrap()
            .edges()
            .is_empty()
    );
    let plan = PooSearchPlan::Merge {
        name: "joined".into(),
        parallel: Box::new(parallel),
        stage_name: "merge".into(),
        role: PooSearchRole::Refinement,
        output_domain: "candidates".into(),
    };
    let projected = compile_poo_search_plan("joined", generation(), &plan).unwrap();
    assert_eq!(projected.edges().len(), 2);
    let merge = projected.factor_by_name("merge").unwrap();
    assert!(projected.edges().iter().all(|edge| edge.to() == merge.id()));
}
#[test]
fn nested_compiled_branches_contract_to_exact_stage_dependencies() {
    let leaf = |name| stage(name, PooSearchRole::Acquisition, "workspace", "workspace");
    let pair = PooSearchPlan::Chain {
        name: "left-chain".into(),
        children: vec![leaf("a"), leaf("b")],
    };
    let parallel = PooSearchPlan::Parallel {
        name: "three-arms".into(),
        children: vec![pair, leaf("c"), leaf("d")],
    };
    let plan = PooSearchPlan::Chain {
        name: "outer-chain".into(),
        children: vec![
            leaf("start"),
            PooSearchPlan::Merge {
                name: "join".into(),
                parallel: Box::new(parallel),
                stage_name: "merge".into(),
                role: PooSearchRole::Refinement,
                output_domain: "workspace".into(),
            },
            leaf("end"),
        ],
    };
    let projection = compile_poo_search_plan("nested-dag", generation(), &plan).unwrap();
    let id = |name| projection.factor_by_name(name).unwrap().id();
    let expected = [
        ("start", "a"),
        ("start", "c"),
        ("start", "d"),
        ("a", "b"),
        ("b", "merge"),
        ("c", "merge"),
        ("d", "merge"),
        ("merge", "end"),
    ]
    .into_iter()
    .map(|(from, to)| SearchFactorEdge::new(id(from), id(to)))
    .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        projection
            .edges()
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>(),
        expected
    );
    assert_eq!(projection.factors().len(), 7);
    assert_eq!(projection.edges().len(), 8);
}
