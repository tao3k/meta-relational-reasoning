//! Safe projection of Scheme-owned finite inference using request-local coordinates.

use std::{collections::BTreeSet, error::Error, fmt};

use super::{
    NativeRuntimeStatus, ffi,
    runtime::{NativeRuntimeError, with_native_runtime},
};

/// Failure never selects a fallback evaluator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FiniteInferenceError {
    CoordinateOverflow,
    Worker(crate::worker_profile::WorkerFailure),
    ForeignNode,
    RuntimeUnavailable,
    RuntimeInitialization(NativeRuntimeStatus),
    UnsupportedAbi,
    NativeRejected(i32),
    InvalidNativeCandidate,
}

impl fmt::Display for FiniteInferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for FiniteInferenceError {}

/// Original native relation rows. Coordinates index the caller's immutable mapping.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteInferenceCandidate {
    pub paths: Vec<(usize, usize, usize)>,
    /// (observation index, source coordinate, target coordinate, shortest distance).
    pub influences: Vec<(usize, usize, usize, usize)>,
}

/// Execute bounded shortest-path inference and observation projection in Scheme.
///
/// This returns a candidate, not an identity, generation, coverage or authority
/// receipt. The semantic caller retains those bindings and admits the result.
pub fn evaluate_finite_relations(
    node_count: usize,
    edges: Vec<(usize, usize)>,
    observation_factors: Vec<usize>,
) -> Result<FiniteInferenceCandidate, FiniteInferenceError> {
    if let Some(result) = crate::worker_profile::with_worker(|worker| {
        worker.evaluate_finite_relations(node_count, edges.clone(), observation_factors.clone())
    }) {
        return result.map_err(|error| match error {
            crate::NativeWorkerError::Finite(error) => error,
            error => FiniteInferenceError::Worker(error.failure()),
        });
    }
    let native_nodes =
        i64::try_from(node_count).map_err(|_| FiniteInferenceError::CoordinateOverflow)?;
    let edge_count =
        i64::try_from(edges.len()).map_err(|_| FiniteInferenceError::CoordinateOverflow)?;
    let observation_count = i64::try_from(observation_factors.len())
        .map_err(|_| FiniteInferenceError::CoordinateOverflow)?;
    if edges
        .iter()
        .any(|&(from, to)| from >= node_count || to >= node_count)
        || observation_factors
            .iter()
            .any(|&factor| factor >= node_count)
    {
        return Err(FiniteInferenceError::ForeignNode);
    }
    let pair_bound = node_count
        .checked_mul(node_count)
        .ok_or(FiniteInferenceError::CoordinateOverflow)?;
    let influence_bound = node_count
        .checked_mul(observation_factors.len())
        .ok_or(FiniteInferenceError::CoordinateOverflow)?;
    with_native_runtime(move || {
        if ffi::finite_version() != 1 {
            return Err(FiniteInferenceError::UnsupportedAbi);
        }
        // Stage, solve, read and release form one operation on the owner thread.
        let result = (|| {
            check(ffi::finite_start(
                native_nodes,
                edge_count,
                observation_count,
            ))?;
            for &(from, to) in &edges {
                check(ffi::finite_edge(from as i64, to as i64))?;
            }
            for &factor in &observation_factors {
                check(ffi::finite_observe(factor as i64))?;
            }
            check(ffi::finite_solve())?;
            let paths = rows::<3>(0, pair_bound)?;
            let influences = rows::<4>(1, influence_bound)?;
            admit_candidate(node_count, &observation_factors, paths, influences)
        })();
        ffi::finite_reset();
        result
    })
    .map_err(|error| match error {
        NativeRuntimeError::Unavailable => FiniteInferenceError::RuntimeUnavailable,
        NativeRuntimeError::Status(status) => FiniteInferenceError::RuntimeInitialization(status),
    })?
}

fn check(code: i32) -> Result<(), FiniteInferenceError> {
    if code == 0 {
        Ok(())
    } else {
        Err(FiniteInferenceError::NativeRejected(code))
    }
}

fn rows<const N: usize>(table: i32, bound: usize) -> Result<Vec<[usize; N]>, FiniteInferenceError> {
    let count = usize::try_from(ffi::finite_count(table))
        .map_err(|_| FiniteInferenceError::InvalidNativeCandidate)?;
    if count > bound {
        return Err(FiniteInferenceError::InvalidNativeCandidate);
    }
    let mut values = Vec::with_capacity(count);
    for row in 0..count {
        let mut value = [0; N];
        for (column, cell) in value.iter_mut().enumerate() {
            *cell = usize::try_from(ffi::finite_cell(table, row as i64, column as i64))
                .map_err(|_| FiniteInferenceError::InvalidNativeCandidate)?;
        }
        values.push(value);
    }
    Ok(values)
}

pub(crate) fn admit_candidate(
    node_count: usize,
    observation_factors: &[usize],
    paths: Vec<[usize; 3]>,
    influences: Vec<[usize; 4]>,
) -> Result<FiniteInferenceCandidate, FiniteInferenceError> {
    if paths.len() > node_count.saturating_mul(node_count)
        || influences.len() > node_count.saturating_mul(observation_factors.len())
    {
        return Err(FiniteInferenceError::InvalidNativeCandidate);
    }
    let mut pairs = BTreeSet::new();
    for &[from, to, distance] in &paths {
        if from >= node_count
            || to >= node_count
            || distance == 0
            || distance > node_count
            || !pairs.insert((from, to))
        {
            return Err(FiniteInferenceError::InvalidNativeCandidate);
        }
    }
    let mut projected = BTreeSet::new();
    for &[index, from, to, distance] in &influences {
        if observation_factors.get(index) != Some(&from)
            || to >= node_count
            || distance >= node_count
            || !projected.insert((index, to))
        {
            return Err(FiniteInferenceError::InvalidNativeCandidate);
        }
        if from == to {
            if distance != 0 {
                return Err(FiniteInferenceError::InvalidNativeCandidate);
            }
        } else if !paths.contains(&[from, to, distance]) {
            return Err(FiniteInferenceError::InvalidNativeCandidate);
        }
    }
    Ok(FiniteInferenceCandidate {
        paths: paths
            .into_iter()
            .map(|[from, to, distance]| (from, to, distance))
            .collect(),
        influences: influences
            .into_iter()
            .map(|[index, from, to, distance]| (index, from, to, distance))
            .collect(),
    })
}
