//! Validated portable composition boundary for MRR contracts.
#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::Cursor;

pub use mrr_identity::{QueryId, ReasoningBundleId, RelationId, RulePackId};
pub use mrr_logic::Rule;
pub use mrr_query::MetaQueryIr;
use mrr_relation::EvidenceCompleteness;
pub use mrr_relation::{Fact, RelationError, RelationSchema};
pub use mrr_transition::Transition;
use serde::{Deserialize, Serialize};

const BUNDLE_PREFIX: &[u8] = b"mrr.reasoning-bundle.v1\0";

/// A language-neutral query plus explicit dependencies on other templates.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct QueryTemplate {
    query: MetaQueryIr,
    dependencies: Vec<QueryId>,
}

impl QueryTemplate {
    #[must_use]
    pub fn new(query: MetaQueryIr, dependencies: Vec<QueryId>) -> Self {
        Self {
            query,
            dependencies,
        }
    }

    #[must_use]
    pub const fn id(&self) -> QueryId {
        self.query.id()
    }

    #[must_use]
    pub const fn query(&self) -> &MetaQueryIr {
        &self.query
    }

    #[must_use]
    pub fn dependencies(&self) -> &[QueryId] {
        &self.dependencies
    }

    fn normalize(&mut self) {
        self.dependencies.sort_unstable();
        self.dependencies.dedup();
        self.query = self.query.clone().normalized();
    }
}

/// A named collection of rules admitted as one authority unit.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RulePack {
    id: RulePackId,
    rules: Vec<Rule>,
}

impl RulePack {
    #[must_use]
    pub fn new(id: RulePackId, rules: Vec<Rule>) -> Self {
        Self { id, rules }
    }

    #[must_use]
    pub const fn id(&self) -> RulePackId {
        self.id
    }

    #[must_use]
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    fn normalize(&mut self) {
        self.rules.sort_by_key(Rule::id);
    }
}

/// A bounded reverse goal bound to a known query template.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InverseGoal {
    name: String,
    query_template: QueryId,
}

impl InverseGoal {
    pub fn new(name: impl Into<String>, query_template: QueryId) -> Result<Self, BundleError> {
        let name = name.into();
        validate_name(&name)?;
        Ok(Self {
            name,
            query_template,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn query_template(&self) -> QueryId {
        self.query_template
    }
}

/// A named collection of immutable-generation transitions.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TransitionSystem {
    name: String,
    relation_refs: Vec<RelationId>,
    transitions: Vec<Transition>,
}

impl TransitionSystem {
    pub fn new(
        name: impl Into<String>,
        relation_refs: Vec<RelationId>,
        transitions: Vec<Transition>,
    ) -> Result<Self, BundleError> {
        let name = name.into();
        validate_name(&name)?;
        Ok(Self {
            name,
            relation_refs,
            transitions,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn transitions(&self) -> &[Transition] {
        &self.transitions
    }

    #[must_use]
    pub fn relation_refs(&self) -> &[RelationId] {
        &self.relation_refs
    }

    fn normalize(&mut self) {
        self.relation_refs.sort_unstable();
        self.relation_refs.dedup();
        self.transitions
            .sort_by_key(|transition| (transition.from(), transition.to()));
    }
}

/// Required lineage retention for conclusions produced from the bundle.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum LineagePolicy {
    /// Retain every admitted derivation.
    #[default]
    Complete,
    /// Retain one deterministic shortest witness when explicitly requested.
    DeterministicShortestWitness,
}

/// Controls which validated intermediate artifacts may be projected to callers.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProjectionPolicy {
    pub include_source_facts: bool,
    pub include_intermediate_derivations: bool,
}

impl Default for ProjectionPolicy {
    fn default() -> Self {
        Self {
            include_source_facts: true,
            include_intermediate_derivations: true,
        }
    }
}

/// Fail-closed validation settings embedded in the bundle identity.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ValidationProfile {
    pub max_query_dependency_depth: u32,
    pub require_complete_evidence: bool,
}

impl Default for ValidationProfile {
    fn default() -> Self {
        Self {
            max_query_dependency_depth: 64,
            require_complete_evidence: true,
        }
    }
}

/// Untrusted input to the bundle admission boundary.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReasoningBundleDeclaration {
    pub relations: Vec<RelationSchema>,
    pub facts: Vec<Fact>,
    pub query_templates: Vec<QueryTemplate>,
    pub rule_packs: Vec<RulePack>,
    pub inverse_goals: Vec<InverseGoal>,
    pub transition_systems: Vec<TransitionSystem>,
    pub lineage_policy: LineagePolicy,
    pub projection_policy: ProjectionPolicy,
    pub validation_profile: ValidationProfile,
}

/// Fully validated and canonically identified reasoning program.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReasoningBundle {
    id: ReasoningBundleId,
    declaration: ReasoningBundleDeclaration,
    canonical: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BundleError {
    EmptyRelations,
    DuplicateRelation,
    DuplicateFact,
    DuplicateQueryTemplate(QueryId),
    DuplicateRulePack(RulePackId),
    DuplicateRule,
    DuplicateInverseGoal(String),
    DuplicateTransitionSystem(String),
    EmptyRulePack(RulePackId),
    UnknownFactRelation,
    UnknownQueryRelation(RelationId),
    UnknownQueryTemplate(QueryId),
    CyclicQueryTemplateReference(QueryId),
    QueryDependencyBudgetExceeded { limit: u32 },
    UnknownRuleRelation(RelationId),
    UnknownTransitionRelation(RelationId),
    UnknownRetractedFact,
    IncompleteEvidence(mrr_identity::FactId),
    InvalidRelationSchema(RelationError),
    InvalidFact(RelationError),
    InvalidName(String),
    InvalidValidationProfile,
    SchemaMismatch,
    TrailingBytes,
    NonCanonicalEncoding,
    Encoding(String),
    Decoding(String),
    Identity(mrr_identity::IdentityError),
}

impl fmt::Display for BundleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for BundleError {}

impl ReasoningBundle {
    /// Validates, normalizes, canonically encodes, and identifies a declaration.
    pub fn admit(mut declaration: ReasoningBundleDeclaration) -> Result<Self, BundleError> {
        normalize(&mut declaration);
        validate(&declaration)?;
        let canonical = encode_declaration(&declaration)?;
        let id =
            ReasoningBundleId::from_canonical_bytes(&canonical).map_err(BundleError::Identity)?;
        Ok(Self {
            id,
            declaration,
            canonical,
        })
    }

    /// Decodes only canonical bytes whose full semantic graph passes admission.
    pub fn decode_canonical(encoded: &[u8]) -> Result<Self, BundleError> {
        let payload = encoded
            .strip_prefix(BUNDLE_PREFIX)
            .ok_or(BundleError::SchemaMismatch)?;
        let mut cursor = Cursor::new(payload);
        let declaration: ReasoningBundleDeclaration = ciborium::from_reader(&mut cursor)
            .map_err(|error| BundleError::Decoding(error.to_string()))?;
        if cursor.position() != payload.len() as u64 {
            return Err(BundleError::TrailingBytes);
        }
        let bundle = Self::admit(declaration)?;
        if bundle.canonical != encoded {
            return Err(BundleError::NonCanonicalEncoding);
        }
        Ok(bundle)
    }

    #[must_use]
    pub const fn id(&self) -> ReasoningBundleId {
        self.id
    }

    #[must_use]
    pub fn encode_canonical(&self) -> &[u8] {
        &self.canonical
    }

    #[must_use]
    pub const fn declaration(&self) -> &ReasoningBundleDeclaration {
        &self.declaration
    }

    pub fn validate(&self) -> Result<(), BundleError> {
        validate(&self.declaration)
    }

    #[must_use]
    pub fn relations(&self) -> &[RelationSchema] {
        &self.declaration.relations
    }

    #[must_use]
    pub fn facts(&self) -> &[Fact] {
        &self.declaration.facts
    }

    #[must_use]
    pub fn query_templates(&self) -> &[QueryTemplate] {
        &self.declaration.query_templates
    }

    pub fn rules(&self) -> impl Iterator<Item = &Rule> {
        self.declaration.rule_packs.iter().flat_map(RulePack::rules)
    }

    #[must_use]
    pub fn rule_packs(&self) -> &[RulePack] {
        &self.declaration.rule_packs
    }

    #[must_use]
    pub fn inverse_goals(&self) -> &[InverseGoal] {
        &self.declaration.inverse_goals
    }

    #[must_use]
    pub fn transition_systems(&self) -> &[TransitionSystem] {
        &self.declaration.transition_systems
    }

    #[must_use]
    pub fn transition_count(&self) -> usize {
        self.declaration
            .transition_systems
            .iter()
            .map(|system| system.transitions.len())
            .sum()
    }

    #[must_use]
    pub const fn lineage_policy(&self) -> LineagePolicy {
        self.declaration.lineage_policy
    }

    #[must_use]
    pub const fn projection_policy(&self) -> ProjectionPolicy {
        self.declaration.projection_policy
    }

    #[must_use]
    pub const fn validation_profile(&self) -> ValidationProfile {
        self.declaration.validation_profile
    }
}

fn encode_declaration(declaration: &ReasoningBundleDeclaration) -> Result<Vec<u8>, BundleError> {
    let mut encoded = BUNDLE_PREFIX.to_vec();
    ciborium::into_writer(declaration, &mut encoded)
        .map_err(|error| BundleError::Encoding(error.to_string()))?;
    Ok(encoded)
}

fn normalize(declaration: &mut ReasoningBundleDeclaration) {
    declaration.relations.sort_by_key(RelationSchema::id);
    declaration.facts.sort_by_key(Fact::id);
    for template in &mut declaration.query_templates {
        template.normalize();
    }
    declaration.query_templates.sort_by_key(QueryTemplate::id);
    for pack in &mut declaration.rule_packs {
        pack.normalize();
    }
    declaration.rule_packs.sort_by_key(RulePack::id);
    declaration
        .inverse_goals
        .sort_by(|left, right| left.name.cmp(&right.name));
    for system in &mut declaration.transition_systems {
        system.normalize();
    }
    declaration
        .transition_systems
        .sort_by(|left, right| left.name.cmp(&right.name));
}

fn validate(declaration: &ReasoningBundleDeclaration) -> Result<(), BundleError> {
    if declaration.relations.is_empty() {
        return Err(BundleError::EmptyRelations);
    }
    if declaration.validation_profile.max_query_dependency_depth == 0 {
        return Err(BundleError::InvalidValidationProfile);
    }

    let mut schemas = BTreeMap::new();
    for schema in &declaration.relations {
        if schemas.insert(schema.id(), schema).is_some() {
            return Err(BundleError::DuplicateRelation);
        }
    }

    let mut fact_ids = BTreeSet::new();
    for fact in &declaration.facts {
        validate_fact(
            fact,
            &schemas,
            &mut fact_ids,
            declaration.validation_profile.require_complete_evidence,
        )?;
    }

    let mut templates = BTreeMap::new();
    for template in &declaration.query_templates {
        if templates.insert(template.id(), template).is_some() {
            return Err(BundleError::DuplicateQueryTemplate(template.id()));
        }
        for relation in template.query.referenced_relations() {
            if !schemas.contains_key(&relation) {
                return Err(BundleError::UnknownQueryRelation(relation));
            }
        }
    }
    for template in &declaration.query_templates {
        for dependency in &template.dependencies {
            if !templates.contains_key(dependency) {
                return Err(BundleError::UnknownQueryTemplate(*dependency));
            }
        }
    }
    validate_query_graph(&templates, declaration.validation_profile)?;

    let mut pack_ids = BTreeSet::new();
    let mut rule_ids = BTreeSet::new();
    for pack in &declaration.rule_packs {
        if !pack_ids.insert(pack.id) {
            return Err(BundleError::DuplicateRulePack(pack.id));
        }
        if pack.rules.is_empty() {
            return Err(BundleError::EmptyRulePack(pack.id));
        }
        for rule in &pack.rules {
            if !rule_ids.insert(rule.id()) {
                return Err(BundleError::DuplicateRule);
            }
            for atom in std::iter::once(rule.head()).chain(rule.body()) {
                if !schemas.contains_key(&atom.relation) {
                    return Err(BundleError::UnknownRuleRelation(atom.relation));
                }
            }
        }
    }

    let mut inverse_names = BTreeSet::new();
    for inverse in &declaration.inverse_goals {
        validate_name(&inverse.name)?;
        if !inverse_names.insert(&inverse.name) {
            return Err(BundleError::DuplicateInverseGoal(inverse.name.clone()));
        }
        if !templates.contains_key(&inverse.query_template) {
            return Err(BundleError::UnknownQueryTemplate(inverse.query_template));
        }
    }

    let mut transition_names = BTreeSet::new();
    for system in &declaration.transition_systems {
        validate_name(&system.name)?;
        if !transition_names.insert(&system.name) {
            return Err(BundleError::DuplicateTransitionSystem(system.name.clone()));
        }
        for relation in &system.relation_refs {
            if !schemas.contains_key(relation) {
                return Err(BundleError::UnknownTransitionRelation(*relation));
            }
        }
        for transition in &system.transitions {
            for fact in transition.insertions() {
                validate_fact(
                    fact,
                    &schemas,
                    &mut fact_ids,
                    declaration.validation_profile.require_complete_evidence,
                )?;
            }
        }
    }
    for system in &declaration.transition_systems {
        for transition in &system.transitions {
            if transition
                .retractions()
                .iter()
                .any(|fact| !fact_ids.contains(fact))
            {
                return Err(BundleError::UnknownRetractedFact);
            }
        }
    }
    Ok(())
}

fn validate_query_graph(
    templates: &BTreeMap<QueryId, &QueryTemplate>,
    profile: ValidationProfile,
) -> Result<(), BundleError> {
    fn visit(
        id: QueryId,
        templates: &BTreeMap<QueryId, &QueryTemplate>,
        states: &mut BTreeMap<QueryId, u8>,
        depth: u32,
        limit: u32,
    ) -> Result<(), BundleError> {
        if depth > limit {
            return Err(BundleError::QueryDependencyBudgetExceeded { limit });
        }
        match states.get(&id) {
            Some(1) => return Err(BundleError::CyclicQueryTemplateReference(id)),
            Some(2) => return Ok(()),
            _ => {}
        }
        states.insert(id, 1);
        for dependency in &templates[&id].dependencies {
            visit(*dependency, templates, states, depth + 1, limit)?;
        }
        states.insert(id, 2);
        Ok(())
    }

    let mut states = BTreeMap::new();
    for id in templates.keys().copied() {
        visit(
            id,
            templates,
            &mut states,
            1,
            profile.max_query_dependency_depth,
        )?;
    }
    Ok(())
}

fn validate_fact(
    fact: &Fact,
    schemas: &BTreeMap<RelationId, &RelationSchema>,
    fact_ids: &mut BTreeSet<mrr_identity::FactId>,
    require_complete_evidence: bool,
) -> Result<(), BundleError> {
    if !fact_ids.insert(fact.id()) {
        return Err(BundleError::DuplicateFact);
    }
    let schema = schemas
        .get(&fact.relation())
        .ok_or(BundleError::UnknownFactRelation)?;
    if require_complete_evidence && fact.context().completeness() != EvidenceCompleteness::Complete
    {
        return Err(BundleError::IncompleteEvidence(fact.id()));
    }
    schema.validate_fact(fact).map_err(BundleError::InvalidFact)
}

fn validate_name(name: &str) -> Result<(), BundleError> {
    let valid = !name.is_empty()
        && name.trim() == name
        && name.is_ascii()
        && name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"-_".contains(&byte)
        });
    if valid {
        Ok(())
    } else {
        Err(BundleError::InvalidName(name.to_owned()))
    }
}
