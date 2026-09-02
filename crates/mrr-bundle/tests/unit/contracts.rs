use crate::{
    BundleError, Fact, InverseGoal, QueryTemplate, ReasoningBundle, ReasoningBundleDeclaration,
    RelationError, RelationSchema, RulePack, ValidationProfile,
};
use mrr_identity::{
    EntityId, FactId, GenerationId, QueryId, QueryOperatorId, RelationId, RuleId, RulePackId,
};
use mrr_logic::{Atom, Rule, Term};
use mrr_query::{
    Binding, Direction, Expression, GraphPattern, MetaQueryIr, NodePattern, PathPattern,
    PathSegment, Projection, RelationPattern,
};
use mrr_relation::{
    EvidenceCompleteness, FactProvenance, FactValidity, RelationAuthority, RelationCardinality,
    RelationContext, RelationField, Value, ValueType,
};

fn relation(label: &str) -> RelationId {
    RelationId::from_canonical_bytes(label.as_bytes()).expect("fixture relation identity")
}

fn schema(relation: RelationId, name: &str, arity: usize) -> RelationSchema {
    RelationSchema::new(
        relation,
        name,
        (0..arity)
            .map(|index| {
                RelationField::new(format!("value-{index}"), ValueType::Integer).expect("field")
            })
            .collect(),
        RelationCardinality::ManyToMany,
    )
    .expect("schema")
}

fn source_context(generation: GenerationId, label: &[u8]) -> RelationContext {
    let authority = EntityId::from_canonical_bytes(label).expect("authority");
    RelationContext::new(
        generation,
        RelationAuthority::Entity(authority),
        FactProvenance::Source(authority),
        EvidenceCompleteness::Complete,
        FactValidity::Valid,
    )
}

fn query(query_id: QueryId, relation: RelationId) -> MetaQueryIr {
    let start = NodePattern::new(Binding::new("left").expect("binding"), Vec::new());
    let end = NodePattern::new(Binding::new("right").expect("binding"), Vec::new());
    let edge = RelationPattern::new(None, vec![relation], Direction::Outgoing, 1, Some(1))
        .expect("relation pattern");
    let graph = GraphPattern::new(
        QueryOperatorId::from_canonical_bytes(b"bundle:graph-operator").expect("operator"),
        vec![PathPattern::new(start, vec![PathSegment::new(edge, end)])],
    )
    .expect("graph");
    MetaQueryIr::new(
        query_id,
        graph,
        Vec::new(),
        vec![Projection::new(
            QueryOperatorId::from_canonical_bytes(b"bundle:projection-operator").expect("operator"),
            Expression::Binding(Binding::new("right").expect("binding")),
            Binding::new("result").expect("binding"),
        )],
        Vec::new(),
        Vec::new(),
        None,
    )
    .expect("query")
}

fn declaration(
    relations: Vec<RelationSchema>,
    facts: Vec<Fact>,
    rules: Vec<Rule>,
) -> ReasoningBundleDeclaration {
    let rule_packs = if rules.is_empty() {
        Vec::new()
    } else {
        vec![RulePack::new(
            RulePackId::from_canonical_bytes(b"bundle:test-pack").expect("rule pack"),
            rules,
        )]
    };
    ReasoningBundleDeclaration {
        relations,
        facts,
        rule_packs,
        ..ReasoningBundleDeclaration::default()
    }
}

#[test]
fn bundle_admission_is_the_single_cross_contract_boundary() {
    let relation = relation("relation:bundle-valid");
    let generation =
        GenerationId::from_canonical_bytes(b"generation:bundle-valid").expect("generation id");
    let input = declaration(
        vec![schema(relation, "edge", 2)],
        vec![Fact::new(
            FactId::from_canonical_bytes(b"fact:bundle-valid").expect("fact id"),
            relation,
            vec![Value::Integer(1)],
            source_context(generation, b"authority:bundle-valid"),
        )],
        Vec::new(),
    );
    assert_eq!(
        ReasoningBundle::admit(input),
        Err(BundleError::InvalidFact(RelationError::ArityMismatch {
            expected: 2,
            actual: 1,
        }))
    );
}

#[test]
fn unknown_rule_and_query_relations_fail_closed() {
    let known = relation("relation:known");
    let unknown = relation("relation:unknown");
    let rule = Rule::new(
        RuleId::from_canonical_bytes(b"rule:unknown-relation").expect("rule id"),
        Atom {
            relation: unknown,
            terms: vec![Term::Value(Value::Integer(1))],
        },
        vec![Atom {
            relation: known,
            terms: vec![Term::Value(Value::Integer(1))],
        }],
    )
    .expect("rule");
    assert_eq!(
        ReasoningBundle::admit(declaration(
            vec![schema(known, "known", 1)],
            Vec::new(),
            vec![rule]
        )),
        Err(BundleError::UnknownRuleRelation(unknown))
    );

    let query_id = QueryId::from_canonical_bytes(b"query:unknown-relation").expect("query");
    let mut input = declaration(vec![schema(known, "known", 1)], Vec::new(), Vec::new());
    input.query_templates = vec![QueryTemplate::new(query(query_id, unknown), Vec::new())];
    assert_eq!(
        ReasoningBundle::admit(input),
        Err(BundleError::UnknownQueryRelation(unknown))
    );
}

#[test]
fn unknown_and_cyclic_query_template_references_fail_closed() {
    let relation = relation("relation:template");
    let first = QueryId::from_canonical_bytes(b"query:first").expect("query");
    let second = QueryId::from_canonical_bytes(b"query:second").expect("query");
    let missing = QueryId::from_canonical_bytes(b"query:missing").expect("query");

    let mut unknown = declaration(vec![schema(relation, "edge", 1)], Vec::new(), Vec::new());
    unknown.query_templates = vec![QueryTemplate::new(query(first, relation), vec![missing])];
    assert_eq!(
        ReasoningBundle::admit(unknown),
        Err(BundleError::UnknownQueryTemplate(missing))
    );

    let mut cyclic = declaration(vec![schema(relation, "edge", 1)], Vec::new(), Vec::new());
    cyclic.query_templates = vec![
        QueryTemplate::new(query(first, relation), vec![second]),
        QueryTemplate::new(query(second, relation), vec![first]),
    ];
    assert!(matches!(
        ReasoningBundle::admit(cyclic),
        Err(BundleError::CyclicQueryTemplateReference(id)) if id == first || id == second
    ));
}

#[test]
fn query_dependency_budget_is_incomplete_and_never_admitted() {
    let relation = relation("relation:budget");
    let first = QueryId::from_canonical_bytes(b"query:budget-first").expect("query");
    let second = QueryId::from_canonical_bytes(b"query:budget-second").expect("query");
    let mut input = declaration(vec![schema(relation, "edge", 1)], Vec::new(), Vec::new());
    input.query_templates = vec![
        QueryTemplate::new(query(first, relation), vec![second]),
        QueryTemplate::new(query(second, relation), Vec::new()),
    ];
    input.validation_profile = ValidationProfile {
        max_query_dependency_depth: 1,
        require_complete_evidence: true,
    };
    assert_eq!(
        ReasoningBundle::admit(input),
        Err(BundleError::QueryDependencyBudgetExceeded { limit: 1 })
    );
}

#[test]
fn inverse_goal_requires_an_admitted_query_template() {
    let relation = relation("relation:inverse");
    let missing = QueryId::from_canonical_bytes(b"query:missing-inverse").expect("query");
    let mut input = declaration(vec![schema(relation, "edge", 1)], Vec::new(), Vec::new());
    input.inverse_goals = vec![InverseGoal::new("why-not", missing).expect("inverse goal")];
    assert_eq!(
        ReasoningBundle::admit(input),
        Err(BundleError::UnknownQueryTemplate(missing))
    );
}

#[test]
fn canonical_bundle_is_order_independent_and_roundtrips_exactly() {
    let first_relation = relation("relation:first");
    let second_relation = relation("relation:second");
    let first_query = QueryId::from_canonical_bytes(b"query:first-canonical").expect("query");
    let second_query = QueryId::from_canonical_bytes(b"query:second-canonical").expect("query");

    let mut left = declaration(
        vec![
            schema(second_relation, "second", 1),
            schema(first_relation, "first", 1),
        ],
        Vec::new(),
        Vec::new(),
    );
    left.query_templates = vec![
        QueryTemplate::new(query(second_query, second_relation), vec![first_query]),
        QueryTemplate::new(query(first_query, first_relation), Vec::new()),
    ];
    let mut right = left.clone();
    right.relations.reverse();
    right.query_templates.reverse();

    let left = ReasoningBundle::admit(left).expect("left bundle");
    let right = ReasoningBundle::admit(right).expect("right bundle");
    assert_eq!(left.id(), right.id());
    assert_eq!(left.encode_canonical(), right.encode_canonical());
    assert_eq!(
        ReasoningBundle::decode_canonical(left.encode_canonical()).expect("decode"),
        left
    );
}
