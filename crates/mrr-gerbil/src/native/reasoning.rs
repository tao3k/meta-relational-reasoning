//! Safe projection from Gerbil reasoning AOT tables into a validated bundle.

use std::collections::BTreeMap;
use std::fmt;

use mrr_bundle::{
    BundleError, InverseGoal, LineagePolicy, ProjectionPolicy, QueryTemplate, ReasoningBundle,
    ReasoningBundleDeclaration, RulePack, TransitionSystem, ValidationProfile,
};
use mrr_identity::{QueryId, QueryOperatorId, RelationId, RuleId, RulePackId};
use mrr_logic::{Atom, Rule, Term};
use mrr_query::{
    Binding, Direction, Expression, GraphPattern, MetaQueryIr, NodePattern, PathPattern,
    PathSegment, Projection, RelationPattern, Variable,
};
use mrr_relation::{RelationCardinality, RelationField, RelationSchema, ValueType};

use super::{NativeGrammar, ffi, runtime::native_runtime_access};

#[derive(Clone, Copy)]
#[repr(i32)]
enum Table {
    Relations = 0,
    Queries = 1,
    Rules = 2,
    InverseGoals = 3,
    TransitionSystems = 4,
    LineagePolicy = 5,
    ProjectionPolicy = 6,
    ValidationProfile = 7,
}

/// Fail-closed errors at the Gerbil reasoning AOT boundary.
#[derive(Debug, Eq, PartialEq)]
pub enum ReasoningBundleLoadError {
    NativeGrammar(String),
    InvalidCount {
        table: &'static str,
        value: i64,
    },
    InvalidText {
        table: &'static str,
        row: i64,
        column: i64,
    },
    InvalidCodepoint(i32),
    UnknownRelation(String),
    UnknownQuery(String),
    UnknownValueType(String),
    UnknownCardinality(String),
    UnknownLineagePolicy(String),
    InvalidBoolean(String),
    InvalidInteger(String),
    UnsupportedRuleArity {
        rule: String,
        body: usize,
    },
    Query(String),
    Rule(String),
    Bundle(BundleError),
}

impl fmt::Display for ReasoningBundleLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Gerbil reasoning bundle rejected: {self:?}")
    }
}

impl std::error::Error for ReasoningBundleLoadError {}

/// Loads, validates, and canonically identifies the AOT-declared reasoning module.
pub fn load_reasoning_bundle() -> Result<ReasoningBundle, ReasoningBundleLoadError> {
    let runtime = native_runtime_access().map_err(|()| {
        ReasoningBundleLoadError::NativeGrammar("native Gerbil runtime lock is poisoned".to_owned())
    })?;
    NativeGrammar::load_with_runtime(&runtime)
        .map_err(|error| ReasoningBundleLoadError::NativeGrammar(error.to_string()))?;
    let relations = load_relations()?;
    let relation_ids = relations
        .iter()
        .map(|schema| (schema.predicate().to_owned(), schema.id()))
        .collect::<BTreeMap<_, _>>();
    let (query_templates, query_ids) = load_queries(&relation_ids)?;
    let declaration = ReasoningBundleDeclaration {
        relations,
        facts: Vec::new(),
        query_templates,
        rule_packs: load_rules(&relation_ids)?,
        inverse_goals: load_inverse_goals(&query_ids)?,
        transition_systems: load_transition_systems(&relation_ids)?,
        lineage_policy: load_lineage_policy()?,
        projection_policy: load_projection_policy()?,
        validation_profile: load_validation_profile()?,
    };
    ReasoningBundle::admit(declaration).map_err(ReasoningBundleLoadError::Bundle)
}

fn load_relations() -> Result<Vec<RelationSchema>, ReasoningBundleLoadError> {
    (0..count(Table::Relations, "relation-schemas")?)
        .map(|row| {
            let name = text(Table::Relations, "relation-schemas", row, 0)?;
            let cardinality = match text(Table::Relations, "relation-schemas", row, 1)?.as_str() {
                "one-to-one" => RelationCardinality::OneToOne,
                "one-to-many" => RelationCardinality::OneToMany,
                "many-to-one" => RelationCardinality::ManyToOne,
                "many-to-many" => RelationCardinality::ManyToMany,
                value => {
                    return Err(ReasoningBundleLoadError::UnknownCardinality(
                        value.to_owned(),
                    ));
                }
            };
            let fields = (0..nested_count(Table::Relations, "relation-schemas", row)?)
                .map(|field| {
                    let field_name =
                        nested_text(Table::Relations, "relation-schemas", row, field, 0)?;
                    let field_type =
                        nested_text(Table::Relations, "relation-schemas", row, field, 1)?;
                    let value_type = match field_type.as_str() {
                        "entity" => ValueType::Entity,
                        "null" => ValueType::Null,
                        "boolean" => ValueType::Boolean,
                        "integer" => ValueType::Integer,
                        "decimal" => ValueType::Decimal,
                        "string" => ValueType::String,
                        "list" => ValueType::List,
                        value => {
                            return Err(ReasoningBundleLoadError::UnknownValueType(
                                value.to_owned(),
                            ));
                        }
                    };
                    RelationField::new(field_name, value_type).map_err(|error| {
                        ReasoningBundleLoadError::Bundle(BundleError::InvalidRelationSchema(error))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            RelationSchema::new(relation_id(&name), name, fields, cardinality).map_err(|error| {
                ReasoningBundleLoadError::Bundle(BundleError::InvalidRelationSchema(error))
            })
        })
        .collect()
}

fn load_queries(
    relations: &BTreeMap<String, RelationId>,
) -> Result<(Vec<QueryTemplate>, BTreeMap<String, QueryId>), ReasoningBundleLoadError> {
    let mut names = BTreeMap::new();
    for row in 0..count(Table::Queries, "query-templates")? {
        let name = text(Table::Queries, "query-templates", row, 0)?;
        names.insert(name.clone(), query_id(&name));
    }
    let templates = (0..count(Table::Queries, "query-templates")?)
        .map(|row| {
            let name = text(Table::Queries, "query-templates", row, 0)?;
            let relation_name = text(Table::Queries, "query-templates", row, 1)?;
            let relation = *relations
                .get(&relation_name)
                .ok_or_else(|| ReasoningBundleLoadError::UnknownRelation(relation_name.clone()))?;
            let dependencies = (0..nested_count(Table::Queries, "query-templates", row)?)
                .map(|index| {
                    let dependency = nested_text(Table::Queries, "query-templates", row, index, 0)?;
                    names
                        .get(&dependency)
                        .copied()
                        .ok_or(ReasoningBundleLoadError::UnknownQuery(dependency))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(QueryTemplate::new(
                query_ir(names[&name], relation).map_err(ReasoningBundleLoadError::Query)?,
                dependencies,
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((templates, names))
}

fn load_rules(
    relations: &BTreeMap<String, RelationId>,
) -> Result<Vec<RulePack>, ReasoningBundleLoadError> {
    let mut packs: BTreeMap<String, Vec<Rule>> = BTreeMap::new();
    for row in 0..count(Table::Rules, "rules")? {
        let pack = text(Table::Rules, "rules", row, 0)?;
        let name = text(Table::Rules, "rules", row, 1)?;
        let head_name = text(Table::Rules, "rules", row, 2)?;
        let head = *relations
            .get(&head_name)
            .ok_or_else(|| ReasoningBundleLoadError::UnknownRelation(head_name.clone()))?;
        let body = (0..nested_count(Table::Rules, "rules", row)?)
            .map(|index| {
                let relation_name = nested_text(Table::Rules, "rules", row, index, 0)?;
                relations
                    .get(&relation_name)
                    .copied()
                    .ok_or(ReasoningBundleLoadError::UnknownRelation(relation_name))
            })
            .collect::<Result<Vec<_>, _>>()?;
        packs
            .entry(pack.clone())
            .or_default()
            .push(rule(&pack, &name, head, &body)?);
    }
    packs
        .into_iter()
        .map(|(name, rules)| Ok(RulePack::new(rule_pack_id(&name), rules)))
        .collect()
}

fn load_inverse_goals(
    queries: &BTreeMap<String, QueryId>,
) -> Result<Vec<InverseGoal>, ReasoningBundleLoadError> {
    (0..count(Table::InverseGoals, "inverse-goals")?)
        .map(|row| {
            let name = text(Table::InverseGoals, "inverse-goals", row, 0)?;
            let query = text(Table::InverseGoals, "inverse-goals", row, 1)?;
            let query = queries
                .get(&query)
                .copied()
                .ok_or(ReasoningBundleLoadError::UnknownQuery(query))?;
            InverseGoal::new(name, query).map_err(ReasoningBundleLoadError::Bundle)
        })
        .collect()
}

fn load_transition_systems(
    relations: &BTreeMap<String, RelationId>,
) -> Result<Vec<TransitionSystem>, ReasoningBundleLoadError> {
    (0..count(Table::TransitionSystems, "transition-systems")?)
        .map(|row| {
            let name = text(Table::TransitionSystems, "transition-systems", row, 0)?;
            let relation_refs =
                (0..nested_count(Table::TransitionSystems, "transition-systems", row)?)
                    .map(|index| {
                        let relation = nested_text(
                            Table::TransitionSystems,
                            "transition-systems",
                            row,
                            index,
                            0,
                        )?;
                        relations
                            .get(&relation)
                            .copied()
                            .ok_or(ReasoningBundleLoadError::UnknownRelation(relation))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
            TransitionSystem::new(name, relation_refs, Vec::new())
                .map_err(ReasoningBundleLoadError::Bundle)
        })
        .collect()
}

fn load_lineage_policy() -> Result<LineagePolicy, ReasoningBundleLoadError> {
    match text(Table::LineagePolicy, "lineage-policy", 0, 0)?.as_str() {
        "complete" => Ok(LineagePolicy::Complete),
        "deterministic-shortest-witness" => Ok(LineagePolicy::DeterministicShortestWitness),
        value => Err(ReasoningBundleLoadError::UnknownLineagePolicy(
            value.to_owned(),
        )),
    }
}

fn load_projection_policy() -> Result<ProjectionPolicy, ReasoningBundleLoadError> {
    Ok(ProjectionPolicy {
        include_source_facts: boolean(&text(Table::ProjectionPolicy, "projection-policy", 0, 0)?)?,
        include_intermediate_derivations: boolean(&text(
            Table::ProjectionPolicy,
            "projection-policy",
            0,
            1,
        )?)?,
    })
}

fn load_validation_profile() -> Result<ValidationProfile, ReasoningBundleLoadError> {
    let depth = text(Table::ValidationProfile, "validation-profile", 0, 0)?;
    Ok(ValidationProfile {
        max_query_dependency_depth: depth
            .parse()
            .map_err(|_| ReasoningBundleLoadError::InvalidInteger(depth))?,
        require_complete_evidence: boolean(&text(
            Table::ValidationProfile,
            "validation-profile",
            0,
            1,
        )?)?,
    })
}

fn query_ir(id: QueryId, relation: RelationId) -> Result<MetaQueryIr, String> {
    let left = binding("left")?;
    let right = binding("right")?;
    let graph = GraphPattern::new(
        operator_id("graph"),
        vec![PathPattern::new(
            NodePattern::new(left, Vec::new()),
            vec![PathSegment::new(
                RelationPattern::new(None, vec![relation], Direction::Outgoing, 1, Some(1))
                    .map_err(|error| format!("{error:?}"))?,
                NodePattern::new(right.clone(), Vec::new()),
            )],
        )],
    )
    .map_err(|error| format!("{error:?}"))?;
    MetaQueryIr::new(
        id,
        graph,
        Vec::new(),
        vec![Projection::new(
            operator_id("projection"),
            Expression::Binding(right),
            binding("result")?,
        )],
        Vec::new(),
        Vec::new(),
        None,
    )
    .map_err(|error| format!("{error:?}"))
}

fn rule(
    pack: &str,
    name: &str,
    head: RelationId,
    body: &[RelationId],
) -> Result<Rule, ReasoningBundleLoadError> {
    let (head_variables, body_variables): (&[&str], &[&[&str]]) = match body.len() {
        1 => (&["x", "y"], &[&["x", "y"]]),
        2 => (&["x", "z"], &[&["x", "y"], &["y", "z"]]),
        count => {
            return Err(ReasoningBundleLoadError::UnsupportedRuleArity {
                rule: name.to_owned(),
                body: count,
            });
        }
    };
    Rule::new(
        rule_id(pack, name),
        atom(head, head_variables),
        body.iter()
            .zip(body_variables)
            .map(|(relation, variables)| atom(*relation, variables))
            .collect(),
    )
    .map_err(|error| ReasoningBundleLoadError::Rule(format!("{error:?}")))
}

fn atom(relation: RelationId, variables: &[&str]) -> Atom {
    Atom {
        relation,
        terms: variables
            .iter()
            .map(|name| Term::Variable(Variable::new(*name).expect("static variable")))
            .collect(),
    }
}

fn binding(name: &str) -> Result<Binding, String> {
    Binding::new(name).map_err(|error| format!("{error:?}"))
}

fn boolean(value: &str) -> Result<bool, ReasoningBundleLoadError> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        value => Err(ReasoningBundleLoadError::InvalidBoolean(value.to_owned())),
    }
}

fn relation_id(name: &str) -> RelationId {
    RelationId::from_canonical_bytes(format!("gerbil:relation:{name}"))
        .expect("non-empty Gerbil relation identity")
}

fn query_id(name: &str) -> QueryId {
    QueryId::from_canonical_bytes(format!("gerbil:query:{name}"))
        .expect("non-empty Gerbil query identity")
}

fn rule_pack_id(name: &str) -> RulePackId {
    RulePackId::from_canonical_bytes(format!("gerbil:rule-pack:{name}"))
        .expect("non-empty Gerbil rule-pack identity")
}

fn rule_id(pack: &str, name: &str) -> RuleId {
    RuleId::from_canonical_bytes(format!("gerbil:rule:{pack}:{name}"))
        .expect("non-empty Gerbil rule identity")
}

fn operator_id(name: &str) -> QueryOperatorId {
    QueryOperatorId::from_canonical_bytes(format!("gerbil:query-operator:{name}"))
        .expect("non-empty Gerbil query operator identity")
}

fn count(table: Table, name: &'static str) -> Result<i64, ReasoningBundleLoadError> {
    let value = ffi::reasoning_table_count(table as i32);
    if value < 0 {
        Err(ReasoningBundleLoadError::InvalidCount { table: name, value })
    } else {
        Ok(value)
    }
}

fn nested_count(
    table: Table,
    name: &'static str,
    row: i64,
) -> Result<i64, ReasoningBundleLoadError> {
    let value = ffi::reasoning_nested_count(table as i32, row);
    if value < 0 {
        Err(ReasoningBundleLoadError::InvalidCount { table: name, value })
    } else {
        Ok(value)
    }
}

fn text(
    table: Table,
    name: &'static str,
    row: i64,
    column: i64,
) -> Result<String, ReasoningBundleLoadError> {
    let length = ffi::reasoning_row_text_length(table as i32, row, column);
    collect_text(name, row, column, length, |index| {
        ffi::reasoning_row_text_char(table as i32, row, column, index)
    })
}

fn nested_text(
    table: Table,
    name: &'static str,
    row: i64,
    nested_row: i64,
    column: i64,
) -> Result<String, ReasoningBundleLoadError> {
    let length = ffi::reasoning_nested_text_length(table as i32, row, nested_row, column);
    collect_text(name, row, column, length, |index| {
        ffi::reasoning_nested_text_char(table as i32, row, nested_row, column, index)
    })
}

fn collect_text(
    table: &'static str,
    row: i64,
    column: i64,
    length: i64,
    character: impl Fn(i64) -> i32,
) -> Result<String, ReasoningBundleLoadError> {
    if length < 0 {
        return Err(ReasoningBundleLoadError::InvalidText { table, row, column });
    }
    (0..length)
        .map(|index| {
            let codepoint = character(index);
            char::from_u32(codepoint as u32)
                .ok_or(ReasoningBundleLoadError::InvalidCodepoint(codepoint))
        })
        .collect()
}
