use super::contracts::{element, fact_id, state};
use crate::{
    AgenticAiContextCompositionGraph as Graph, AgenticAiContextCompositionGraphInput as Input,
    AgenticAiContextCompositionNode as Node, AgenticAiContextCompositionProducer,
    AgenticAiContextError as Error,
};
fn node(name: &str, parents: &[&str]) -> Node {
    Node {
        element: fact_id(name),
        parent_orders: vec![parents.iter().map(|p| fact_id(p)).collect()],
        suffix: false,
    }
}
fn graph() -> Input {
    Input {
        root: Some(fact_id("child")),
        nodes: vec![node("child", &["a", "b"]), node("a", &[]), node("b", &[])],
    }
}
#[test]
fn composition_canonicalizes_nodes_and_preserves_order_and_suffix() {
    let state = state(
        &["child"],
        vec![
            element("child", &["a", "b"]),
            element("a", &[]),
            element("b", &[]),
        ],
    );
    let mut input = graph();
    input.nodes[1].suffix = true;
    let checked = Graph::validate(&state, input.clone()).unwrap();
    input.nodes.reverse();
    assert_eq!(Graph::validate(&state, input).unwrap(), checked);
    assert!(
        checked
            .input()
            .nodes
            .iter()
            .find(|n| n.element == fact_id("a"))
            .unwrap()
            .suffix
    );
    checked
        .check_result(&state, vec![fact_id("child"), fact_id("a"), fact_id("b")])
        .unwrap();
    for order in [["child", "b", "a"], ["a", "child", "b"]] {
        assert_eq!(
            checked.check_result(&state, order.map(fact_id).to_vec()),
            Err(Error::CompositionOrderViolation)
        );
    }
    assert_eq!(
        checked.check_result(&state, vec![fact_id("child")]),
        Err(Error::InvalidPrecedence)
    );
}
#[test]
fn composition_rejects_cycles_unknown_disconnected_and_duplicate_nodes() {
    let state = state(
        &["child"],
        vec![
            element("child", &["a", "b"]),
            element("a", &[]),
            element("b", &[]),
        ],
    );
    let mut cycle = graph();
    cycle.nodes[1].parent_orders = vec![vec![fact_id("child")]];
    assert_eq!(Graph::validate(&state, cycle), Err(Error::CompositionCycle));
    let mut unknown = graph();
    unknown.nodes[0].parent_orders[0].push(fact_id("missing"));
    assert_eq!(
        Graph::validate(&state, unknown),
        Err(Error::UnknownElement(fact_id("missing")))
    );
    let mut disconnected = graph();
    disconnected.nodes[0].parent_orders = vec![vec![fact_id("a")]];
    assert_eq!(
        Graph::validate(&state, disconnected),
        Err(Error::CompositionGraphMismatch)
    );
    let mut duplicate = graph();
    duplicate.nodes[2] = duplicate.nodes[1].clone();
    assert_eq!(
        Graph::validate(&state, duplicate),
        Err(Error::CompositionGraphMismatch)
    );
    let mut budget = graph();
    budget.nodes[0].parent_orders = vec![vec![]; 33];
    assert_eq!(
        Graph::validate(&state, budget),
        Err(Error::CompositionBudget)
    );
}
#[test]
fn semantic_dependency_cycles_do_not_become_inheritance_cycles() {
    let state = state(&["a"], vec![element("a", &["a"])]);
    let checked = Graph::validate(
        &state,
        Input {
            root: Some(fact_id("a")),
            nodes: vec![node("a", &[])],
        },
    )
    .unwrap();
    checked.check_result(&state, vec![fact_id("a")]).unwrap();
    let empty = super::contracts::state(&[], vec![]);
    Graph::validate(
        &empty,
        Input {
            root: None,
            nodes: vec![],
        },
    )
    .unwrap()
    .check_result(&empty, vec![])
    .unwrap();
    assert_eq!(
        Graph::validate(
            &empty,
            Input {
                root: Some(fact_id("a")),
                nodes: vec![]
            }
        ),
        Err(Error::CompositionGraphMismatch)
    );
}
#[test]
fn producer_declarations_reject_empty_whitespace_and_unbounded_versions() {
    for label in [String::new(), " v1".into(), "v1 ".into(), "x".repeat(257)] {
        let producer = AgenticAiContextCompositionProducer {
            owner: "owner".into(),
            algorithm: "C4".into(),
            implementation_version: label,
        };
        assert_eq!(producer.validate(), Err(Error::InvalidCompositionProducer));
    }
}
