//! Safe fixed-width boundary to the Scheme-owned outer resource scheduler.

use std::{error::Error, fmt};

use super::{
    ffi,
    runtime::{native_runtime_access, native_runtime_status_is_ready},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
/// Scheme-owned outer-loop phase.
pub enum DriverPhase {
    /// The model resource may propose a candidate.
    AwaitProposal = 0,
    /// The MRR kernel must derive and atomically admit the candidate.
    AwaitClosure = 1,
    /// An admitted closure is available.
    Complete = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
/// Resource selected by the Scheme AOT scheduler.
pub enum DriverResource {
    /// Non-authoritative model proposal resource.
    ModelProposal = 0,
    /// Authoritative Rust/Ascent closure resource.
    MrrClosure = 1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
/// Typed resource outcome consumed by the Scheme scheduler.
pub enum DriverStatus {
    /// The model returned a candidate, not a fact or decision.
    Candidate = 0,
    /// The MRR kernel returned a fully admitted closure.
    Admitted = 1,
    /// The MRR kernel rejected the candidate.
    Rejected = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Fail-closed native scheduler failure.
pub enum DriverError {
    /// The process-global Gambit lock was poisoned.
    NativeRuntimePoisoned,
    /// Gambit initialization failed with the returned code.
    NativeRuntimeInitialization(i32),
    /// No declared resource transition matches the supplied receipt.
    InvalidTransition,
    /// A rejection consumed the final allowed cycle.
    BudgetExhausted,
    /// A cycle value cannot cross the fixed-width ABI.
    CycleOverflow,
}

/// Complete input to one Scheme-owned state transition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DriverTransition {
    /// Current outer-loop phase.
    pub phase: DriverPhase,
    /// Resource returning the receipt.
    pub resource: DriverResource,
    /// Typed receipt outcome.
    pub status: DriverStatus,
    /// Zero-based loop cycle.
    pub cycle: usize,
    /// Positive maximum cycle count.
    pub max_cycles: usize,
}

impl fmt::Display for DriverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NativeRuntimePoisoned => formatter.write_str("native runtime lock is poisoned"),
            Self::NativeRuntimeInitialization(code) => {
                write!(
                    formatter,
                    "native runtime initialization failed with {code}"
                )
            }
            Self::InvalidTransition => formatter.write_str("Scheme driver rejected the transition"),
            Self::BudgetExhausted => {
                formatter.write_str("Scheme driver exhausted its cycle budget")
            }
            Self::CycleOverflow => formatter.write_str("driver cycle does not fit the native ABI"),
        }
    }
}

impl Error for DriverError {}

fn with_runtime<T>(operation: impl FnOnce() -> T) -> Result<T, DriverError> {
    let _access = native_runtime_access().map_err(|()| DriverError::NativeRuntimePoisoned)?;
    let init = ffi::runtime_init();
    if !native_runtime_status_is_ready(init) {
        return Err(DriverError::NativeRuntimeInitialization(init));
    }
    Ok(operation())
}

/// Ask the Scheme AOT scheduler which resource owns the next step.
pub fn driver_request(phase: DriverPhase) -> Result<Option<DriverResource>, DriverError> {
    if phase == DriverPhase::Complete {
        return Ok(None);
    }
    let code = with_runtime(|| ffi::reasoning_driver_request_resource(phase as i32))?;
    match code {
        0 => Ok(Some(DriverResource::ModelProposal)),
        1 => Ok(Some(DriverResource::MrrClosure)),
        _ => Err(DriverError::InvalidTransition),
    }
}

/// Submit one typed resource receipt to the Scheme AOT scheduler.
pub fn driver_transition(input: DriverTransition) -> Result<DriverPhase, DriverError> {
    let cycle = i64::try_from(input.cycle).map_err(|_| DriverError::CycleOverflow)?;
    let max_cycles = i64::try_from(input.max_cycles).map_err(|_| DriverError::CycleOverflow)?;
    let code = with_runtime(|| {
        ffi::reasoning_driver_transition(
            input.phase as i32,
            input.resource as i32,
            input.status as i32,
            cycle,
            max_cycles,
        )
    })?;
    match code {
        0 => Ok(DriverPhase::AwaitProposal),
        1 => Ok(DriverPhase::AwaitClosure),
        2 => Ok(DriverPhase::Complete),
        -2 => Err(DriverError::BudgetExhausted),
        _ => Err(DriverError::InvalidTransition),
    }
}
