//! Bounded missing-premise analysis over safe, groundable MRR rules.

use std::num::NonZeroUsize;

use mrr_identity::GenerationId;
use mrr_query::{Atom, RelationId, Term, Variable};
use mrr_relation::{EvidenceCompleteness, Fact, FactValidity, Value};

use crate::Rule;

/// A fully ground relational atom used in WHY-NOT receipts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroundAtom {
    relation: RelationId,
    values: Vec<Value>,
}

impl GroundAtom {
    #[must_use]
    pub fn new(relation: RelationId, values: Vec<Value>) -> Self {
        Self { relation, values }
    }

    #[must_use]
    pub const fn relation(&self) -> RelationId {
        self.relation
    }

    #[must_use]
    pub fn values(&self) -> &[Value] {
        &self.values
    }
}

/// Hard recursion and output bounds for missing-premise analysis.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WhyNotLimits {
    max_depth: NonZeroUsize,
    max_expansions: NonZeroUsize,
    max_alternatives: NonZeroUsize,
}

impl WhyNotLimits {
    #[must_use]
    pub const fn new(
        max_depth: NonZeroUsize,
        max_expansions: NonZeroUsize,
        max_alternatives: NonZeroUsize,
    ) -> Self {
        Self {
            max_depth,
            max_expansions,
            max_alternatives,
        }
    }
}

/// Why a definitive missing-premise result could not be produced.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WhyNotIncomplete {
    Cycle,
    DepthBudget,
    ExpansionBudget,
    AlternativeBudget,
    EvidenceCoverage,
}

/// Result class of bounded WHY-NOT analysis.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WhyNotStatus {
    Proven,
    MissingPremises { alternatives: Vec<Vec<GroundAtom>> },
    NoApplicableRule,
    Incomplete(WhyNotIncomplete),
}

/// Generation-bound WHY-NOT receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WhyNotReceipt {
    goal: GroundAtom,
    generation: GenerationId,
    status: WhyNotStatus,
    expansions: usize,
}

impl WhyNotReceipt {
    #[must_use]
    pub const fn goal(&self) -> &GroundAtom {
        &self.goal
    }

    #[must_use]
    pub const fn generation(&self) -> GenerationId {
        self.generation
    }

    #[must_use]
    pub const fn status(&self) -> &WhyNotStatus {
        &self.status
    }

    #[must_use]
    pub const fn expansions(&self) -> usize {
        self.expansions
    }
}

/// Malformed goals or rules that cannot be safely grounded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WhyNotError {
    NonGroundGoal,
    UnboundBodyVariable {
        rule: mrr_identity::RuleId,
        variable: Variable,
    },
}

enum Proof {
    Proven,
    Missing(Vec<Vec<GroundAtom>>),
    NoRule,
    Incomplete(WhyNotIncomplete),
}

struct Analysis<'a> {
    generation: GenerationId,
    rules: Vec<&'a Rule>,
    facts: &'a [Fact],
    limits: WhyNotLimits,
    expansions: usize,
}

/// Explains why a ground goal is not currently derivable in one generation.
pub fn why_not(
    goal: &Atom,
    generation: GenerationId,
    rules: &[Rule],
    facts: &[Fact],
    limits: WhyNotLimits,
) -> Result<WhyNotReceipt, WhyNotError> {
    let goal = ground_goal(goal)?;
    let mut sorted_rules: Vec<_> = rules.iter().collect();
    sorted_rules.sort_by_key(|rule| rule.id());
    let mut analysis = Analysis {
        generation,
        rules: sorted_rules,
        facts,
        limits,
        expansions: 0,
    };
    let status = match analysis.prove(&goal, 0, &mut Vec::new())? {
        Proof::Proven => WhyNotStatus::Proven,
        Proof::Missing(alternatives) => WhyNotStatus::MissingPremises { alternatives },
        Proof::NoRule => WhyNotStatus::NoApplicableRule,
        Proof::Incomplete(reason) => WhyNotStatus::Incomplete(reason),
    };
    Ok(WhyNotReceipt {
        goal,
        generation,
        status,
        expansions: analysis.expansions,
    })
}

impl Analysis<'_> {
    fn prove(
        &mut self,
        goal: &GroundAtom,
        depth: usize,
        active: &mut Vec<GroundAtom>,
    ) -> Result<Proof, WhyNotError> {
        let matching_facts: Vec<_> = self
            .facts
            .iter()
            .filter(|fact| {
                fact.context().generation() == self.generation
                    && fact.relation() == goal.relation
                    && fact.values() == goal.values
            })
            .collect();
        if matching_facts.iter().any(|fact| {
            fact.context().completeness() == EvidenceCompleteness::Complete
                && fact.context().validity() == FactValidity::Valid
        }) {
            return Ok(Proof::Proven);
        }
        if !matching_facts.is_empty() {
            return Ok(Proof::Incomplete(WhyNotIncomplete::EvidenceCoverage));
        }
        if active.contains(goal) {
            return Ok(Proof::Incomplete(WhyNotIncomplete::Cycle));
        }
        if depth >= self.limits.max_depth.get() {
            return Ok(Proof::Incomplete(WhyNotIncomplete::DepthBudget));
        }

        let applicable: Vec<_> = self
            .rules
            .iter()
            .filter_map(|rule| unify_head(rule, goal).map(|bindings| (*rule, bindings)))
            .collect();
        if applicable.is_empty() {
            return Ok(Proof::NoRule);
        }

        active.push(goal.clone());
        let mut alternatives = Vec::new();
        let mut incomplete = None;
        for (rule, bindings) in applicable {
            if self.expansions >= self.limits.max_expansions.get() {
                active.pop();
                return Ok(Proof::Incomplete(WhyNotIncomplete::ExpansionBudget));
            }
            self.expansions += 1;
            let mut branch_alternatives = vec![Vec::new()];
            let mut branch_incomplete = None;
            for atom in rule.body() {
                let premise = ground_atom(atom, &bindings, rule.id())?;
                match self.prove(&premise, depth + 1, active)? {
                    Proof::Proven => {}
                    Proof::Missing(premise_alternatives) => {
                        let Some(combined) = combine_alternatives(
                            &branch_alternatives,
                            &premise_alternatives,
                            self.limits.max_alternatives.get(),
                        ) else {
                            branch_incomplete = Some(WhyNotIncomplete::AlternativeBudget);
                            break;
                        };
                        branch_alternatives = combined;
                    }
                    Proof::NoRule => {
                        for branch in &mut branch_alternatives {
                            append_unique(branch, [premise.clone()]);
                        }
                    }
                    Proof::Incomplete(reason) => branch_incomplete = Some(reason),
                }
            }
            if branch_alternatives == [Vec::new()] && branch_incomplete.is_none() {
                active.pop();
                return Ok(Proof::Proven);
            }
            if let Some(reason) = branch_incomplete {
                incomplete.get_or_insert(reason);
            } else {
                alternatives.extend(branch_alternatives);
                if alternatives.len() > self.limits.max_alternatives.get() {
                    active.pop();
                    return Ok(Proof::Incomplete(WhyNotIncomplete::AlternativeBudget));
                }
            }
        }
        active.pop();
        if let Some(reason) = incomplete {
            Ok(Proof::Incomplete(reason))
        } else {
            Ok(Proof::Missing(alternatives))
        }
    }
}

fn ground_goal(atom: &Atom) -> Result<GroundAtom, WhyNotError> {
    let mut values = Vec::with_capacity(atom.terms.len());
    for term in &atom.terms {
        let Term::Value(value) = term else {
            return Err(WhyNotError::NonGroundGoal);
        };
        values.push(value.clone());
    }
    Ok(GroundAtom::new(atom.relation, values))
}

fn unify_head(rule: &Rule, goal: &GroundAtom) -> Option<Vec<(Variable, Value)>> {
    if rule.head().relation != goal.relation || rule.head().terms.len() != goal.values.len() {
        return None;
    }
    let mut bindings = Vec::new();
    for (term, value) in rule.head().terms.iter().zip(&goal.values) {
        match term {
            Term::Value(expected) if expected != value => return None,
            Term::Value(_) => {}
            Term::Variable(variable) => {
                if let Some((_, bound)) = bindings.iter().find(|(name, _)| name == variable) {
                    if bound != value {
                        return None;
                    }
                } else {
                    bindings.push((variable.clone(), value.clone()));
                }
            }
        }
    }
    Some(bindings)
}

fn ground_atom(
    atom: &Atom,
    bindings: &[(Variable, Value)],
    rule: mrr_identity::RuleId,
) -> Result<GroundAtom, WhyNotError> {
    let values = atom
        .terms
        .iter()
        .map(|term| match term {
            Term::Value(value) => Ok(value.clone()),
            Term::Variable(variable) => bindings
                .iter()
                .find(|(name, _)| name == variable)
                .map(|(_, value)| value.clone())
                .ok_or_else(|| WhyNotError::UnboundBodyVariable {
                    rule,
                    variable: variable.clone(),
                }),
        })
        .collect::<Result<_, _>>()?;
    Ok(GroundAtom::new(atom.relation, values))
}

fn combine_alternatives(
    left: &[Vec<GroundAtom>],
    right: &[Vec<GroundAtom>],
    limit: usize,
) -> Option<Vec<Vec<GroundAtom>>> {
    let mut combined = Vec::new();
    for left_branch in left {
        for right_branch in right {
            let mut branch = left_branch.clone();
            append_unique(&mut branch, right_branch.iter().cloned());
            combined.push(branch);
            if combined.len() > limit {
                return None;
            }
        }
    }
    Some(combined)
}

fn append_unique(target: &mut Vec<GroundAtom>, values: impl IntoIterator<Item = GroundAtom>) {
    for value in values {
        if !target.contains(&value) {
            target.push(value);
        }
    }
}
