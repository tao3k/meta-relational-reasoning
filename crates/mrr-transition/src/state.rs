//! Finite relational state-machine contracts.

use std::collections::BTreeSet;

use mrr_identity::{ActionId, FactId, StateId};

const STATE_SCHEMA: &[u8] = b"mrr.state.v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateSchema {
    allowed_facts: Vec<FactId>,
}

impl StateSchema {
    pub fn new(mut allowed_facts: Vec<FactId>) -> Result<Self, TransitionModelError> {
        if allowed_facts.is_empty() {
            return Err(TransitionModelError::EmptyStateSchema);
        }
        allowed_facts.sort_unstable();
        if let Some(duplicate) = duplicate(&allowed_facts) {
            return Err(TransitionModelError::DuplicateSchemaFact(duplicate));
        }
        Ok(Self { allowed_facts })
    }

    #[must_use]
    pub fn allowed_facts(&self) -> &[FactId] {
        &self.allowed_facts
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateSnapshot {
    id: StateId,
    facts: Vec<FactId>,
}

impl StateSnapshot {
    pub fn from_facts(mut facts: Vec<FactId>) -> Result<Self, TransitionModelError> {
        facts.sort_unstable();
        if let Some(duplicate) = duplicate(&facts) {
            return Err(TransitionModelError::DuplicateStateFact(duplicate));
        }
        let id = derive_state_id(&facts)?;
        Ok(Self { id, facts })
    }

    #[must_use]
    pub const fn id(&self) -> StateId {
        self.id
    }

    #[must_use]
    pub fn facts(&self) -> &[FactId] {
        &self.facts
    }

    #[must_use]
    pub fn contains(&self, fact: FactId) -> bool {
        self.facts.binary_search(&fact).is_ok()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitialState(StateSnapshot);

impl InitialState {
    #[must_use]
    pub const fn new(state: StateSnapshot) -> Self {
        Self(state)
    }

    #[must_use]
    pub const fn state(&self) -> &StateSnapshot {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatePredicate {
    Present(FactId),
    Absent(FactId),
}

impl StatePredicate {
    #[must_use]
    pub const fn fact(self) -> FactId {
        match self {
            Self::Present(fact) | Self::Absent(fact) => fact,
        }
    }

    #[must_use]
    pub fn holds(self, state: &StateSnapshot) -> bool {
        match self {
            Self::Present(fact) => state.contains(fact),
            Self::Absent(fact) => !state.contains(fact),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Precondition {
    all: Vec<StatePredicate>,
}

impl Precondition {
    #[must_use]
    pub fn all(predicates: Vec<StatePredicate>) -> Self {
        Self { all: predicates }
    }

    #[must_use]
    pub fn predicates(&self) -> &[StatePredicate] {
        &self.all
    }

    #[must_use]
    pub fn holds(&self, state: &StateSnapshot) -> bool {
        self.all.iter().all(|predicate| predicate.holds(state))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Effect {
    insertions: Vec<FactId>,
    retractions: Vec<FactId>,
}

impl Effect {
    pub fn new(
        mut insertions: Vec<FactId>,
        mut retractions: Vec<FactId>,
    ) -> Result<Self, TransitionModelError> {
        insertions.sort_unstable();
        retractions.sort_unstable();
        if let Some(duplicate) = duplicate(&insertions) {
            return Err(TransitionModelError::DuplicateEffectFact(duplicate));
        }
        if let Some(duplicate) = duplicate(&retractions) {
            return Err(TransitionModelError::DuplicateEffectFact(duplicate));
        }
        if let Some(conflict) = insertions
            .iter()
            .find(|fact| retractions.binary_search(fact).is_ok())
        {
            return Err(TransitionModelError::ConflictingEffect(*conflict));
        }
        Ok(Self {
            insertions,
            retractions,
        })
    }

    #[must_use]
    pub fn insertions(&self) -> &[FactId] {
        &self.insertions
    }

    #[must_use]
    pub fn retractions(&self) -> &[FactId] {
        &self.retractions
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Action {
    id: ActionId,
    precondition: Precondition,
    effect: Effect,
}

impl Action {
    #[must_use]
    pub const fn new(id: ActionId, precondition: Precondition, effect: Effect) -> Self {
        Self {
            id,
            precondition,
            effect,
        }
    }

    #[must_use]
    pub const fn id(&self) -> ActionId {
        self.id
    }

    #[must_use]
    pub const fn precondition(&self) -> &Precondition {
        &self.precondition
    }

    #[must_use]
    pub const fn effect(&self) -> &Effect {
        &self.effect
    }

    #[must_use]
    pub fn enabled(&self, state: &StateSnapshot) -> bool {
        self.precondition.holds(state)
    }

    pub fn apply(&self, state: &StateSnapshot) -> Result<StateSnapshot, TransitionModelError> {
        let mut facts: BTreeSet<_> = state.facts.iter().copied().collect();
        for fact in &self.effect.retractions {
            facts.remove(fact);
        }
        facts.extend(&self.effect.insertions);
        StateSnapshot::from_facts(facts.into_iter().collect())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Invariant {
    ForbiddenAll { name: String, facts: Vec<FactId> },
    RequiredAny { name: String, facts: Vec<FactId> },
}

impl Invariant {
    pub fn forbidden_all(
        name: impl Into<String>,
        facts: Vec<FactId>,
    ) -> Result<Self, TransitionModelError> {
        validate_invariant(name.into(), facts, true)
    }

    pub fn required_any(
        name: impl Into<String>,
        facts: Vec<FactId>,
    ) -> Result<Self, TransitionModelError> {
        validate_invariant(name.into(), facts, false)
    }

    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::ForbiddenAll { name, .. } | Self::RequiredAny { name, .. } => name,
        }
    }

    #[must_use]
    pub fn facts(&self) -> &[FactId] {
        match self {
            Self::ForbiddenAll { facts, .. } | Self::RequiredAny { facts, .. } => facts,
        }
    }

    #[must_use]
    pub fn holds(&self, state: &StateSnapshot) -> bool {
        match self {
            Self::ForbiddenAll { facts, .. } => !facts.iter().all(|fact| state.contains(*fact)),
            Self::RequiredAny { facts, .. } => facts.iter().any(|fact| state.contains(*fact)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitionSystem {
    schema: StateSchema,
    initial: InitialState,
    actions: Vec<Action>,
    invariants: Vec<Invariant>,
}

impl TransitionSystem {
    pub fn admit(
        schema: StateSchema,
        initial: InitialState,
        mut actions: Vec<Action>,
        invariants: Vec<Invariant>,
    ) -> Result<Self, TransitionModelError> {
        let allowed: BTreeSet<_> = schema.allowed_facts.iter().copied().collect();
        validate_references(initial.state().facts(), &allowed)?;
        actions.sort_by_key(Action::id);
        for pair in actions.windows(2) {
            if pair[0].id() == pair[1].id() {
                return Err(TransitionModelError::DuplicateAction(pair[0].id()));
            }
        }
        for action in &actions {
            validate_references(
                &action
                    .precondition
                    .predicates()
                    .iter()
                    .map(|predicate| predicate.fact())
                    .collect::<Vec<_>>(),
                &allowed,
            )?;
            validate_references(action.effect.insertions(), &allowed)?;
            validate_references(action.effect.retractions(), &allowed)?;
        }
        for invariant in &invariants {
            validate_references(invariant.facts(), &allowed)?;
        }
        Ok(Self {
            schema,
            initial,
            actions,
            invariants,
        })
    }

    #[must_use]
    pub const fn schema(&self) -> &StateSchema {
        &self.schema
    }

    #[must_use]
    pub const fn initial(&self) -> &InitialState {
        &self.initial
    }

    #[must_use]
    pub fn actions(&self) -> &[Action] {
        &self.actions
    }

    #[must_use]
    pub fn invariants(&self) -> &[Invariant] {
        &self.invariants
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransitionModelError {
    EmptyStateSchema,
    DuplicateSchemaFact(FactId),
    DuplicateStateFact(FactId),
    DuplicateEffectFact(FactId),
    ConflictingEffect(FactId),
    UnknownFact(FactId),
    DuplicateAction(ActionId),
    InvalidInvariantName,
    EmptyInvariant,
}

fn derive_state_id(facts: &[FactId]) -> Result<StateId, TransitionModelError> {
    let mut bytes = Vec::new();
    push_field(&mut bytes, STATE_SCHEMA);
    for fact in facts {
        push_field(&mut bytes, fact.digest_bytes());
    }
    StateId::from_canonical_bytes(bytes).map_err(|_| TransitionModelError::EmptyStateSchema)
}

fn validate_invariant(
    name: String,
    mut facts: Vec<FactId>,
    forbidden: bool,
) -> Result<Invariant, TransitionModelError> {
    if name.is_empty() || name.trim() != name {
        return Err(TransitionModelError::InvalidInvariantName);
    }
    if facts.is_empty() {
        return Err(TransitionModelError::EmptyInvariant);
    }
    facts.sort_unstable();
    facts.dedup();
    Ok(if forbidden {
        Invariant::ForbiddenAll { name, facts }
    } else {
        Invariant::RequiredAny { name, facts }
    })
}

fn validate_references(
    facts: &[FactId],
    allowed: &BTreeSet<FactId>,
) -> Result<(), TransitionModelError> {
    if let Some(fact) = facts.iter().find(|fact| !allowed.contains(fact)) {
        Err(TransitionModelError::UnknownFact(*fact))
    } else {
        Ok(())
    }
}

fn duplicate<T: Copy + Eq>(values: &[T]) -> Option<T> {
    values
        .windows(2)
        .find(|pair| pair[0] == pair[1])
        .map(|pair| pair[0])
}

fn push_field(output: &mut Vec<u8>, field: &[u8]) {
    output.extend_from_slice(&(field.len() as u64).to_be_bytes());
    output.extend_from_slice(field);
}
