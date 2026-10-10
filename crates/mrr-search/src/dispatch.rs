//! Bounded physical dispatch leases over an original Scheme/POO projection.
//! Data owns physical source admission and execution; this gate owns scheduling
//! capacity and monotonic retirement. POO owns retained dependency decisions. Memory is a reservation, not measured RSS.
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
    ControllerRejected,
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
    #[cfg(any(feature = "native-inference", feature = "worker-inference"))]
    controller: Option<Arc<mrr_gerbil::PooSearchController>>,
}
/// Immutable admission metadata; it is not a source authenticity certificate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SearchDispatchReceipt {
    pub generation: GenerationId,
    pub factor: SearchFactor,
    pub input_bytes: usize,
    pub output_bytes: usize,
    pub results: usize,
    /// Scheme logical (revision, attempt); absent for the legacy root-only gate.
    pub attempt: Option<(usize, usize)>,
}
pub struct SearchDispatchLease {
    dispatch: SearchDispatch,
    factor: SearchFactor,
    reservation: SearchDispatchResources,
    #[cfg(any(feature = "native-inference", feature = "worker-inference"))]
    request: Option<mrr_gerbil::PooSearchRequest>,
}
impl SearchDispatch {
    #[cfg(any(feature = "native-inference", feature = "worker-inference"))]
    fn controller_error(
        state: &mut State,
        error: mrr_gerbil::TemporalRuntimeError,
    ) -> SearchDispatchError {
        // Code 4 is the owner's bounded pre-commit validation rejection.
        // Other failures can lose an acknowledgement after a state transition.
        if !matches!(error, mrr_gerbil::TemporalRuntimeError::NativeRejected(4)) {
            state.snapshot.retired = true;
        }
        SearchDispatchError::ControllerRejected
    }
    fn controlled(&self) -> bool {
        #[cfg(any(feature = "native-inference", feature = "worker-inference"))]
        {
            self.controller.is_some()
        }
        #[cfg(not(any(feature = "native-inference", feature = "worker-inference")))]
        {
            false
        }
    }
    #[cfg(any(feature = "native-inference", feature = "worker-inference"))]
    fn stage_name(&self, factor: SearchFactor) -> Result<&str, SearchDispatchError> {
        self.projection
            .names
            .iter()
            .find_map(|(name, value)| (*value == factor).then_some(name.as_str()))
            .ok_or(SearchDispatchError::ForeignFactor)
    }
    /// Build the original projection and retained controller in the same Scheme session.
    /// # Errors
    /// Rejects invalid POO plans, unavailable owners and transport/admission failures.
    #[cfg(any(feature = "native-inference", feature = "worker-inference"))]
    pub fn from_plan(
        name: &str,
        generation: GenerationId,
        plan: &crate::PooSearchPlan,
        configuration: &str,
        source_cut: &str,
        max_in_flight: NonZeroUsize,
        limits: SearchDispatchResources,
    ) -> Result<Self, String> {
        let controller = mrr_gerbil::PooSearchController::new(
            name,
            &generation.to_string(),
            configuration,
            source_cut,
            plan,
        )
        .map_err(|e| e.to_string())?;
        let projection =
            crate::poo::projection_from_graph(name, generation, controller.graph().clone())?;
        let mut dispatch = Self::new(projection, max_in_flight, limits);
        dispatch.controller = Some(Arc::new(controller));
        Ok(dispatch)
    }
    /// Resolve original stage identity without constructing a second factor registry.
    #[must_use]
    pub fn factor_by_name(&self, name: &str) -> Option<SearchFactor> {
        self.projection.factor_by_name(name)
    }
    /// # Errors
    /// Requires a retained controller and rejects retired or failed owner sessions.
    #[cfg(any(feature = "native-inference", feature = "worker-inference"))]
    pub fn frontier(&self) -> Result<Vec<SearchFactor>, SearchDispatchError> {
        let state = self
            .state
            .lock()
            .map_err(|_| SearchDispatchError::Poisoned)?;
        if state.snapshot.retired {
            return Err(SearchDispatchError::Retired);
        }
        self.controller
            .as_ref()
            .ok_or(SearchDispatchError::UnsupportedPrerequisites)?
            .frontier()
            .map_err(|_| SearchDispatchError::ControllerRejected)?
            .iter()
            .map(|name| {
                self.factor_by_name(name)
                    .ok_or(SearchDispatchError::ForeignFactor)
            })
            .collect()
    }
    /// Delegate selective invalidation to Scheme. Physical work remains lease-owned.
    /// # Errors
    /// Requires a live controller and original changed factors; rejects invalid cuts.
    #[cfg(any(feature = "native-inference", feature = "worker-inference"))]
    pub fn revise(
        &self,
        changed: &[SearchFactor],
        source_cut: &str,
    ) -> Result<(), SearchDispatchError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| SearchDispatchError::Poisoned)?;
        if state.snapshot.retired {
            return Err(SearchDispatchError::Retired);
        }
        let names = changed
            .iter()
            .map(|factor| self.stage_name(*factor).map(str::to_owned))
            .collect::<Result<Vec<_>, _>>()?;
        self.controller
            .as_ref()
            .ok_or(SearchDispatchError::UnsupportedPrerequisites)?
            .revise(&names, source_cut)
            .map_err(|error| Self::controller_error(&mut state, error))?;
        Ok(())
    }

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
            #[cfg(any(feature = "native-inference", feature = "worker-inference"))]
            controller: None,
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
    /// legacy requests are acquisition roots. Controllers created with from_plan
    /// admit dependent stages only through the retained Scheme frontier.
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
        if (!self.controlled() && factor.role() != SearchFactorRole::Acquisition)
            || !self.projection.factors().contains(&factor)
        {
            return Err(SearchDispatchError::ForeignFactor);
        }
        if !self.controlled()
            && self
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
        #[cfg(any(feature = "native-inference", feature = "worker-inference"))]
        let request = self
            .controller
            .as_ref()
            .map(|controller| {
                controller
                    .issue(self.stage_name(factor)?)
                    .map_err(|error| Self::controller_error(&mut state, error))
            })
            .transpose()?;
        state.snapshot.reserved = reserved;
        state.snapshot.consumed.input_bytes = used.input_bytes;
        state.snapshot.in_flight += 1;
        drop(state);
        Ok(SearchDispatchLease {
            dispatch: self.clone(),
            factor,
            reservation,
            #[cfg(any(feature = "native-inference", feature = "worker-inference"))]
            request,
        })
    }
}
impl SearchDispatchLease {
    /// Admit a completed physical result. Retirement and admission serialize on
    /// the same lock: a result that loses this race cannot be published.
    /// # Errors
    /// Rejects late results, output/result overflow or a poisoned state.
    pub fn admit(
        #[allow(unused_mut)] mut self,
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
        #[cfg(any(feature = "native-inference", feature = "worker-inference"))]
        let attempt = self
            .request
            .as_ref()
            .map(|request| (request.revision(), request.attempt()));
        #[cfg(not(any(feature = "native-inference", feature = "worker-inference")))]
        let attempt = None;
        #[cfg(any(feature = "native-inference", feature = "worker-inference"))]
        if let (Some(controller), Some(request)) = (&self.dispatch.controller, &self.request) {
            controller
                .complete(request)
                .map_err(|error| SearchDispatch::controller_error(&mut state, error))?;
            self.request = None;
        }
        state.snapshot.consumed.output_bytes += output_bytes;
        state.snapshot.consumed.results += results;
        let receipt = SearchDispatchReceipt {
            generation: self.dispatch.generation(),
            factor: self.factor,
            input_bytes: self.reservation.input_bytes,
            output_bytes,
            results,
            attempt,
        };
        drop(state);
        Ok(receipt)
    }
}
impl Drop for SearchDispatchLease {
    fn drop(&mut self) {
        #[cfg(any(feature = "native-inference", feature = "worker-inference"))]
        let cancellation_failed =
            if let (Some(controller), Some(request)) = (&self.dispatch.controller, &self.request) {
                controller.cancel(request).is_err()
            } else {
                false
            };
        if let Ok(mut state) = self.dispatch.state.lock() {
            #[cfg(any(feature = "native-inference", feature = "worker-inference"))]
            if cancellation_failed {
                state.snapshot.retired = true;
            }
            state.snapshot.in_flight -= 1;
            state.snapshot.reserved.memory_bytes -= self.reservation.memory_bytes;
            state.snapshot.reserved.input_bytes -= self.reservation.input_bytes;
            state.snapshot.reserved.output_bytes -= self.reservation.output_bytes;
            state.snapshot.reserved.results -= self.reservation.results;
        }
    }
}
