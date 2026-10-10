//! Independent bounded checking of finite native route projections.
use super::transformation::{
    TransformationAdmission, TransformationEndpoint, TransformationError, TransformationLimits,
    digest,
};
/// An ordered structural witness awaiting concrete instance and premise checks.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct TransformationRouteCandidate {
    pub source: TransformationEndpoint,
    pub target: TransformationEndpoint,
    /// Ordered admission digests, retaining evidence rather than only content IDs.
    pub edges: Vec<[u8; 32]>,
}
/// Validate actual native output against an independent adjacency-matrix closure.
/// Complete output must contain exactly the finite reachable pairs; truncated
/// output must still carry a sound ordered path for every returned pair.
pub fn verify_transformation_projection(
    edges: &[TransformationAdmission],
    routes: &[TransformationRouteCandidate],
    status: mrr_deduction::ClosureStatus,
    limits: TransformationLimits,
) -> Result<[u8; 32], TransformationError> {
    use std::collections::{BTreeMap, BTreeSet};
    let mut endpoints = Vec::new();
    let mut by_digest = BTreeMap::new();
    for edge in edges {
        if by_digest.insert(*edge.digest(), edge).is_some() {
            return Err(TransformationError::DuplicateDependency);
        }
        for endpoint in [&edge.definition().source, &edge.definition().target] {
            if !endpoints.contains(endpoint) {
                endpoints.push(endpoint.clone());
            }
        }
    }
    let n = endpoints.len();
    let cells = n.checked_mul(n).ok_or(TransformationError::Budget)?;
    let work = cells.checked_mul(n).ok_or(TransformationError::Budget)?;
    if work > limits.max_bytes.get() || routes.len() > cells {
        return Err(TransformationError::Budget);
    }
    let locate = |endpoint: &TransformationEndpoint| {
        endpoints
            .iter()
            .position(|e| e == endpoint)
            .ok_or(TransformationError::Rejected)
    };
    let mut reach = vec![false; cells];
    for edge in edges {
        reach[locate(&edge.definition().source)? * n + locate(&edge.definition().target)?] = true;
    }
    for pivot in 0..n {
        for source in 0..n {
            for target in 0..n {
                reach[source * n + target] |=
                    reach[source * n + pivot] && reach[pivot * n + target];
            }
        }
    }
    let mut seen = BTreeSet::new();
    for route in routes {
        if route.edges.is_empty() || route.edges.len() > limits.max_steps.get() {
            return Err(TransformationError::Rejected);
        }
        let source = locate(&route.source)?;
        let target = locate(&route.target)?;
        if !seen.insert((source, target)) || !reach[source * n + target] {
            return Err(TransformationError::Rejected);
        }
        let mut current = &route.source;
        for support in &route.edges {
            let edge = by_digest
                .get(support)
                .ok_or(TransformationError::Rejected)?;
            if current != &edge.definition().source {
                return Err(TransformationError::EndpointMismatch);
            }
            current = &edge.definition().target;
        }
        if current != &route.target {
            return Err(TransformationError::EndpointMismatch);
        }
    }
    if status == mrr_deduction::ClosureStatus::Complete
        && seen.len() != reach.iter().filter(|reachable| **reachable).count()
    {
        return Err(TransformationError::Rejected);
    }
    digest(
        &(
            "mrr.transformation-projection-checked.v1",
            edges.iter().map(|edge| edge.digest()).collect::<Vec<_>>(),
            routes,
            status == mrr_deduction::ClosureStatus::Complete,
        ),
        limits,
    )
}
