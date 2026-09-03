//! Semantic-generation transitions over immutable MRR fact identities.
#![forbid(unsafe_code)]

use std::collections::HashSet;

pub use mrr_identity::{FactId, GenerationId};
pub use mrr_relation::Fact;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Transition {
    from: GenerationId,
    to: GenerationId,
    insertions: Vec<Fact>,
    retractions: Vec<FactId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransitionError {
    SameGeneration,
    ConflictingFact(FactId),
}

impl Transition {
    pub fn new(
        from: GenerationId,
        to: GenerationId,
        insertions: Vec<Fact>,
        retractions: Vec<FactId>,
    ) -> Result<Self, TransitionError> {
        if from == to {
            return Err(TransitionError::SameGeneration);
        }
        let retracted = retractions.iter().copied().collect::<HashSet<_>>();
        if let Some(conflict) = insertions
            .iter()
            .map(Fact::id)
            .find(|fact_id| retracted.contains(fact_id))
        {
            return Err(TransitionError::ConflictingFact(conflict));
        }
        Ok(Self {
            from,
            to,
            insertions,
            retractions,
        })
    }

    #[must_use]
    pub const fn from(&self) -> GenerationId {
        self.from
    }

    #[must_use]
    pub const fn to(&self) -> GenerationId {
        self.to
    }

    #[must_use]
    pub fn insertions(&self) -> &[Fact] {
        &self.insertions
    }

    #[must_use]
    pub fn retractions(&self) -> &[FactId] {
        &self.retractions
    }
}
