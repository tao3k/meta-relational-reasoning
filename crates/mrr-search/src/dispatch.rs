//! Bounded physical dispatch leases over an original Scheme/POO projection.
//! Data owns physical source admission and execution; this gate owns scheduling
//! capacity and monotonic retirement. Memory is a reservation, not measured RSS.
use crate::{PooSearchProjection, SearchFactor, SearchFactorRole};
use mrr_identity::GenerationId;
use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SearchDispatchResources {
    pub memory_bytes: usize,
    pub input_bytes: usize,
    pub output_bytes: usize,
    pub results: usize,
}
impl SearchDispatchResources {
    fn checked_add(self, other: Self) -> Option<Self> {
        Some(Self {
            memory_bytes: self.memory_bytes.checked_add(other.memory_bytes)?,
            input_bytes: self.input_bytes.checked_add(other.input_bytes)?,
            output_bytes: self.output_bytes.checked_add(other.output_bytes)?,
            results: self.results.checked_add(other.results)?,
        })
    }
    fn within(self, limit: Self) -> bool {
        self.memory_bytes <= limit.memory_bytes
            && self.input_bytes <= limit.input_bytes
            && self.output_bytes <= limit.output_bytes
            && self.results <= limit.results
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchDispatchError {
    Retired,
    GenerationMismatch,
    ForeignFactor,
    UnsupportedPrerequisites,
    CapacityExceeded,
    ReservationExceeded,
    Poisoned,
}
impl std::fmt::Display for SearchDispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SearchDispatchError {}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SearchDispatchSnapshot {
    pub retired: bool,
    pub in_flight: usize,
    pub reserved: SearchDispatchResources,
    /// Input remains charged after errors or cancelled futures; output/results
    /// are charged only at the result admission linearization point.
    pub consumed: SearchDispatchResources,
}
struct State {
    snapshot: SearchDispatchSnapshot,
}
#[derive(Clone)]
pub struct SearchDispatch {
    projection: Arc<PooSearchProjection>,
    max_in_flight: NonZeroUsize,
    limits: SearchDispatchResources,
    state: Arc<Mutex<State>>,
}
/// Immutable admission metadata; it is not a source authenticity certificate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SearchDispatchReceipt {
    pub generation: GenerationId,
    pub factor: SearchFactor,
    pub input_bytes: usize,
    pub output_bytes: usize,
    pub results: usize,
}
pub struct SearchDispatchLease {
    dispatch: SearchDispatch,
    factor: SearchFactor,
    reservation: SearchDispatchResources,
}
impl SearchDispatch {
    #[must_use]
    pub fn new(
        projection: PooSearchProjection,
        max_in_flight: NonZeroUsize,
        limits: SearchDispatchResources,
    ) -> Self {
        Self {
            projection: Arc::new(projection),
            max_in_flight,
            limits,
            state: Arc::new(Mutex::new(State {
                snapshot: SearchDispatchSnapshot::default(),
            })),
        }
    }
    #[must_use]
    pub fn generation(&self) -> GenerationId {
        self.projection.generation()
    }
    /// Retire this generation permanently. Reusing the same generation identity
    /// requires a separate instance and cannot reactivate existing leases.
    /// # Errors
    /// Rejects a poisoned dispatch state.
    pub fn retire(&self) -> Result<(), SearchDispatchError> {
        self.state
            .lock()
            .map_err(|_| SearchDispatchError::Poisoned)?
            .snapshot
            .retired = true;
        Ok(())
    }
    /// # Errors
    /// Rejects a poisoned dispatch state.
    pub fn snapshot(&self) -> Result<SearchDispatchSnapshot, SearchDispatchError> {
        Ok(self
            .state
            .lock()
            .map_err(|_| SearchDispatchError::Poisoned)?
            .snapshot)
    }
    /// Reserve capacity before acquiring physical backend resources. The first
    /// supported requests are original acquisition roots with no prerequisites;
    /// dependent stages must return through the Scheme continuation.
    /// # Errors
    /// Rejects foreign generation/factor, unsupported prerequisites, retirement,
    /// arithmetic overflow or aggregate capacity exhaustion.
    pub fn reserve(
        &self,
        generation: GenerationId,
        factor: SearchFactor,
        reservation: SearchDispatchResources,
    ) -> Result<SearchDispatchLease, SearchDispatchError> {
        if generation != self.generation() {
            return Err(SearchDispatchError::GenerationMismatch);
        }
        if factor.role() != SearchFactorRole::Acquisition
            || !self.projection.factors().contains(&factor)
        {
            return Err(SearchDispatchError::ForeignFactor);
        }
        if self
            .projection
            .edges()
            .iter()
            .any(|edge| edge.to() == factor.id())
        {
            return Err(SearchDispatchError::UnsupportedPrerequisites);
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| SearchDispatchError::Poisoned)?;
        let old = state.snapshot;
        if old.retired {
            return Err(SearchDispatchError::Retired);
        }
        let reserved = old
            .reserved
            .checked_add(reservation)
            .ok_or(SearchDispatchError::CapacityExceeded)?;
        let mut used = old.consumed;
        used.memory_bytes = 0;
        used.input_bytes = used
            .input_bytes
            .checked_add(reservation.input_bytes)
            .ok_or(SearchDispatchError::CapacityExceeded)?;
        let mut remaining = reserved;
        remaining.input_bytes = 0;
        if old.in_flight >= self.max_in_flight.get()
            || !used
                .checked_add(remaining)
                .is_some_and(|total| total.within(self.limits))
        {
            return Err(SearchDispatchError::CapacityExceeded);
        }
        state.snapshot.reserved = reserved;
        state.snapshot.consumed.input_bytes = used.input_bytes;
        state.snapshot.in_flight += 1;
        drop(state);
        Ok(SearchDispatchLease {
            dispatch: self.clone(),
            factor,
            reservation,
        })
    }
}
impl SearchDispatchLease {
    /// Admit a completed physical result. Retirement and admission serialize on
    /// the same lock: a result that loses this race cannot be published.
    /// # Errors
    /// Rejects late results, output/result overflow or a poisoned state.
    pub fn admit(
        self,
        output_bytes: usize,
        results: usize,
    ) -> Result<SearchDispatchReceipt, SearchDispatchError> {
        let mut state = self
            .dispatch
            .state
            .lock()
            .map_err(|_| SearchDispatchError::Poisoned)?;
        if state.snapshot.retired {
            return Err(SearchDispatchError::Retired);
        }
        if output_bytes > self.reservation.output_bytes || results > self.reservation.results {
            return Err(SearchDispatchError::ReservationExceeded);
        }
        state.snapshot.consumed.output_bytes += output_bytes;
        state.snapshot.consumed.results += results;
        let receipt = SearchDispatchReceipt {
            generation: self.dispatch.generation(),
            factor: self.factor,
            input_bytes: self.reservation.input_bytes,
            output_bytes,
            results,
        };
        drop(state);
        Ok(receipt)
    }
}
impl Drop for SearchDispatchLease {
    fn drop(&mut self) {
        if let Ok(mut state) = self.dispatch.state.lock() {
            state.snapshot.in_flight -= 1;
            state.snapshot.reserved.memory_bytes -= self.reservation.memory_bytes;
            state.snapshot.reserved.input_bytes -= self.reservation.input_bytes;
            state.snapshot.reserved.output_bytes -= self.reservation.output_bytes;
            state.snapshot.reserved.results -= self.reservation.results;
        }
    }
}
