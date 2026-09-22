//! Safe rules over language-neutral MRR atoms.
#![forbid(unsafe_code)]

pub use mrr_identity::RuleId;
pub use mrr_query::{Atom, Term, Variable};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Rule {
    id: RuleId,
    head: Atom,
    body: Vec<Atom>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuleError {
    EmptyBody,
    UnsafeHeadVariable(Variable),
}

impl Rule {
    pub fn new(id: RuleId, head: Atom, body: Vec<Atom>) -> Result<Self, RuleError> {
        if body.is_empty() {
            return Err(RuleError::EmptyBody);
        }
        for variable in head.terms.iter().filter_map(|term| match term {
            Term::Variable(variable) => Some(variable),
            Term::Value(_) => None,
        }) {
            let is_bound = body
                .iter()
                .flat_map(|atom| &atom.terms)
                .any(|term| matches!(term, Term::Variable(candidate) if candidate == variable));
            if !is_bound {
                return Err(RuleError::UnsafeHeadVariable(variable.clone()));
            }
        }
        Ok(Self { id, head, body })
    }

    #[must_use]
    pub const fn id(&self) -> RuleId {
        self.id
    }

    #[must_use]
    pub fn head(&self) -> &Atom {
        &self.head
    }

    #[must_use]
    pub fn body(&self) -> &[Atom] {
        &self.body
    }
}
