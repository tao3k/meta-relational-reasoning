//! Bounded explicit-state safety checking.

use std::collections::{BTreeMap, VecDeque};
use std::num::NonZeroUsize;

use mrr_identity::{ActionId, StateId, TransitionId};

use crate::StatePredicate;
use crate::state::{StateSnapshot, TransitionModelError, TransitionSystem};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SafetyLimits {
    max_states: NonZeroUsize,
    max_transitions: NonZeroUsize,
}

impl SafetyLimits {
    #[must_use]
    pub const fn new(max_states: NonZeroUsize, max_transitions: NonZeroUsize) -> Self {
        Self {
            max_states,
            max_transitions,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SafetyStatus {
    Safe,
    Unsafe,
    Incomplete,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitionStep {
    id: TransitionId,
    action: ActionId,
    from: StateId,
    to: StateId,
}

impl TransitionStep {
    #[must_use]
    pub const fn id(&self) -> TransitionId {
        self.id
    }

    #[must_use]
    pub const fn action(&self) -> ActionId {
        self.action
    }

    #[must_use]
    pub const fn from(&self) -> StateId {
        self.from
    }

    #[must_use]
    pub const fn to(&self) -> StateId {
        self.to
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CounterexampleIr {
    states: Vec<StateSnapshot>,
    steps: Vec<TransitionStep>,
    violated_invariant: String,
}

impl CounterexampleIr {
    #[must_use]
    pub fn states(&self) -> &[StateSnapshot] {
        &self.states
    }

    #[must_use]
    pub fn steps(&self) -> &[TransitionStep] {
        &self.steps
    }

    #[must_use]
    pub fn violated_invariant(&self) -> &str {
        &self.violated_invariant
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SafetyCheckReceipt {
    status: SafetyStatus,
    explored_states: usize,
    explored_transitions: usize,
    counterexample: Option<CounterexampleIr>,
}

impl SafetyCheckReceipt {
    #[must_use]
    pub const fn status(&self) -> SafetyStatus {
        self.status
    }

    #[must_use]
    pub const fn explored_states(&self) -> usize {
        self.explored_states
    }

    #[must_use]
    pub const fn explored_transitions(&self) -> usize {
        self.explored_transitions
    }

    #[must_use]
    pub const fn counterexample(&self) -> Option<&CounterexampleIr> {
        self.counterexample.as_ref()
    }
}

struct ExploredState {
    state: StateSnapshot,
    parent: Option<(usize, TransitionStep)>,
}

pub fn check_safety(
    system: &TransitionSystem,
    limits: SafetyLimits,
) -> Result<SafetyCheckReceipt, TransitionModelError> {
    let initial = system.initial().state().clone();
    let mut explored = vec![ExploredState {
        state: initial.clone(),
        parent: None,
    }];
    let mut indices = BTreeMap::from([(initial.id(), 0_usize)]);
    let mut pending = VecDeque::from([0_usize]);
    let mut transitions = 0_usize;
    let mut actions_by_present_fact: BTreeMap<_, Vec<_>> = BTreeMap::new();
    let mut unanchored_actions = Vec::new();
    for action in system.actions() {
        if let Some(anchor) = action
            .precondition()
            .predicates()
            .iter()
            .find_map(|predicate| match predicate {
                StatePredicate::Present(fact) => Some(*fact),
                StatePredicate::Absent(_) => None,
            })
        {
            actions_by_present_fact
                .entry(anchor)
                .or_default()
                .push(action);
        } else {
            unanchored_actions.push(action);
        }
    }

    if let Some(invariant) = system
        .invariants()
        .iter()
        .find(|invariant| !invariant.holds(&initial))
    {
        return Ok(unsafe_receipt(&explored, 0, transitions, invariant.name()));
    }

    while let Some(index) = pending.pop_front() {
        let state = explored[index].state.clone();
        let mut candidate_actions = unanchored_actions.clone();
        for fact in state.facts() {
            candidate_actions.extend(
                actions_by_present_fact
                    .get(fact)
                    .into_iter()
                    .flatten()
                    .copied(),
            );
        }
        candidate_actions.sort_by_key(|action| action.id());
        for action in candidate_actions
            .into_iter()
            .filter(|action| action.enabled(&state))
        {
            if transitions >= limits.max_transitions.get() {
                return Ok(incomplete_receipt(explored.len(), transitions));
            }
            transitions += 1;
            let next = action.apply(&state)?;
            let step = transition_step(action.id(), state.id(), next.id())?;
            if let Some(existing) = indices.get(&next.id()).copied() {
                if let Some(invariant) = system
                    .invariants()
                    .iter()
                    .find(|invariant| !invariant.holds(&explored[existing].state))
                {
                    return Ok(unsafe_receipt(
                        &explored,
                        existing,
                        transitions,
                        invariant.name(),
                    ));
                }
                continue;
            }
            if explored.len() >= limits.max_states.get() {
                return Ok(incomplete_receipt(explored.len(), transitions));
            }
            let next_index = explored.len();
            indices.insert(next.id(), next_index);
            explored.push(ExploredState {
                state: next,
                parent: Some((index, step)),
            });
            if let Some(invariant) = system
                .invariants()
                .iter()
                .find(|invariant| !invariant.holds(&explored[next_index].state))
            {
                return Ok(unsafe_receipt(
                    &explored,
                    next_index,
                    transitions,
                    invariant.name(),
                ));
            }
            pending.push_back(next_index);
        }
    }
    Ok(SafetyCheckReceipt {
        status: SafetyStatus::Safe,
        explored_states: explored.len(),
        explored_transitions: transitions,
        counterexample: None,
    })
}

fn transition_step(
    action: ActionId,
    from: StateId,
    to: StateId,
) -> Result<TransitionStep, TransitionModelError> {
    let mut bytes = Vec::new();
    for field in [
        action.digest_bytes(),
        from.digest_bytes(),
        to.digest_bytes(),
    ] {
        bytes.extend_from_slice(&(field.len() as u64).to_be_bytes());
        bytes.extend_from_slice(field);
    }
    let id = TransitionId::from_canonical_bytes(bytes)
        .map_err(|_| TransitionModelError::EmptyStateSchema)?;
    Ok(TransitionStep {
        id,
        action,
        from,
        to,
    })
}

fn unsafe_receipt(
    explored: &[ExploredState],
    terminal: usize,
    transitions: usize,
    invariant: &str,
) -> SafetyCheckReceipt {
    let mut states = Vec::new();
    let mut steps = Vec::new();
    let mut cursor = terminal;
    loop {
        states.push(explored[cursor].state.clone());
        let Some((parent, step)) = &explored[cursor].parent else {
            break;
        };
        steps.push(step.clone());
        cursor = *parent;
    }
    states.reverse();
    steps.reverse();
    SafetyCheckReceipt {
        status: SafetyStatus::Unsafe,
        explored_states: explored.len(),
        explored_transitions: transitions,
        counterexample: Some(CounterexampleIr {
            states,
            steps,
            violated_invariant: invariant.to_owned(),
        }),
    }
}

fn incomplete_receipt(states: usize, transitions: usize) -> SafetyCheckReceipt {
    SafetyCheckReceipt {
        status: SafetyStatus::Incomplete,
        explored_states: states,
        explored_transitions: transitions,
        counterexample: None,
    }
}
