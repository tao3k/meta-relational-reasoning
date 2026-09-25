// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::num::NonZeroUsize;

use ascent::{Dual, ascent};
use mrr_identity::{FactId, GenerationId, LineageEdgeId, LineageNodeId, QueryOperatorId};
use mrr_lineage::{
    ImpactError, ImpactGraph, LineageEdge, LineageEdgeKind, LineageGraph, LineageGraphError,
    LineageNode, LineageNodeKind, impact,
};
use sha2::{Digest, Sha256};

const RECEIPT_SCHEMA: &[u8] = b"mrr.search-reasoning-receipt.v1";

/// Backend-neutral responsibility of one consumer-defined Search factor.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SearchFactorRole {
    Acquisition,
    Refinement,
    Reasoning,
    Projection,
}

/// One factor registered by a downstream consumer.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SearchFactor {
    id: QueryOperatorId,
    role: SearchFactorRole,
}

impl SearchFactor {
    #[must_use]
    pub const fn new(id: QueryOperatorId, role: SearchFactorRole) -> Self {
        Self { id, role }
    }

    #[must_use]
    pub const fn id(&self) -> QueryOperatorId {
        self.id
    }

    #[must_use]
    pub const fn role(&self) -> SearchFactorRole {
        self.role
    }
}

/// Directed influence edge from an upstream factor to a downstream factor.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SearchFactorEdge {
    from: QueryOperatorId,
    to: QueryOperatorId,
}

impl SearchFactorEdge {
    #[must_use]
    pub const fn new(from: QueryOperatorId, to: QueryOperatorId) -> Self {
        Self { from, to }
    }

    #[must_use]
    pub const fn from(&self) -> QueryOperatorId {
        self.from
    }

    #[must_use]
    pub const fn to(&self) -> QueryOperatorId {
        self.to
    }
}

/// One immutable factor observation supplied by the consumer runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchObservation {
    id: FactId,
    candidate: FactId,
    factor: QueryOperatorId,
    logical_position: u64,
    causal_parents: Vec<FactId>,
}

impl SearchObservation {
    #[must_use]
    pub fn new(
        id: FactId,
        candidate: FactId,
        factor: QueryOperatorId,
        logical_position: u64,
        causal_parents: Vec<FactId>,
    ) -> Self {
        Self {
            id,
            candidate,
            factor,
            logical_position,
            causal_parents,
        }
    }

    #[must_use]
    pub const fn id(&self) -> FactId {
        self.id
    }

    #[must_use]
    pub const fn candidate(&self) -> FactId {
        self.candidate
    }

    #[must_use]
    pub const fn factor(&self) -> QueryOperatorId {
        self.factor
    }

    #[must_use]
    pub const fn logical_position(&self) -> u64 {
        self.logical_position
    }

    #[must_use]
    pub fn causal_parents(&self) -> &[FactId] {
        &self.causal_parents
    }
}

/// Hard pre-execution and publication budgets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SearchFrameworkLimits {
    max_factors: NonZeroUsize,
    max_edges: NonZeroUsize,
    max_observations: NonZeroUsize,
    max_influences: NonZeroUsize,
    max_results: NonZeroUsize,
}

impl SearchFrameworkLimits {
    #[must_use]
    pub const fn new(
        max_factors: NonZeroUsize,
        max_edges: NonZeroUsize,
        max_observations: NonZeroUsize,
        max_influences: NonZeroUsize,
        max_results: NonZeroUsize,
    ) -> Self {
        Self {
            max_factors,
            max_edges,
            max_observations,
            max_influences,
            max_results,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchFrameworkStatus {
    Complete,
    OutputTruncated,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SearchFrameworkError {
    EmptyFactors,
    DuplicateFactor(QueryOperatorId),
    DuplicateEdge(SearchFactorEdge),
    UnknownFactor(QueryOperatorId),
    SelfEdge(SearchFactorEdge),
    FactorCycle,
    DuplicateObservation(FactId),
    DuplicateCausalParent {
        observation: FactId,
        parent: FactId,
    },
    MissingCausalParent {
        observation: FactId,
        parent: FactId,
    },
    TemporalOrderViolation {
        observation: FactId,
        parent: FactId,
    },
    CausalFactorEdgeMissing {
        observation: FactId,
        parent: FactId,
    },
    NonAcquisitionRoot(FactId),
    BudgetExceeded {
        resource: &'static str,
        required: usize,
        limit: usize,
    },
    Lineage(LineageGraphError),
    InternalPathMismatch,
}

impl fmt::Display for SearchFrameworkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SearchFrameworkError {}

/// One candidate's shortest causal influence trajectory to a factor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchInfluence {
    candidate: FactId,
    factor: QueryOperatorId,
    support_event: FactId,
    factor_path: Vec<QueryOperatorId>,
    result_fact: FactId,
}

impl SearchInfluence {
    #[must_use]
    pub const fn candidate(&self) -> FactId {
        self.candidate
    }

    #[must_use]
    pub const fn factor(&self) -> QueryOperatorId {
        self.factor
    }

    #[must_use]
    pub const fn support_event(&self) -> FactId {
        self.support_event
    }

    #[must_use]
    pub fn factor_path(&self) -> &[QueryOperatorId] {
        &self.factor_path
    }

    #[must_use]
    pub const fn result_fact(&self) -> FactId {
        self.result_fact
    }
}

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SearchReasoningDigest([u8; 32]);

impl fmt::Debug for SearchReasoningDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl fmt::Display for SearchReasoningDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl SearchReasoningDigest {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Complete bounded reasoning output plus a reusable MRR lineage graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchFrameworkReceipt {
    generation: GenerationId,
    status: SearchFrameworkStatus,
    total_influence_count: usize,
    influences: Vec<SearchInfluence>,
    lineage: LineageGraph,
    digest: SearchReasoningDigest,
}

impl SearchFrameworkReceipt {
    #[must_use]
    pub const fn generation(&self) -> GenerationId {
        self.generation
    }

    #[must_use]
    pub const fn status(&self) -> SearchFrameworkStatus {
        self.status
    }

    #[must_use]
    pub const fn total_influence_count(&self) -> usize {
        self.total_influence_count
    }

    #[must_use]
    pub fn influences(&self) -> &[SearchInfluence] {
        &self.influences
    }

    #[must_use]
    pub const fn lineage(&self) -> &LineageGraph {
        &self.lineage
    }

    #[must_use]
    pub const fn digest(&self) -> SearchReasoningDigest {
        self.digest
    }

    pub fn impact(&self, observation: FactId) -> Result<ImpactGraph, ImpactError> {
        impact(&self.lineage, observation)
    }
}

/// Validates a finite causal factor graph, derives influence with Ascent, and
/// materializes deterministic shortest trajectories and MRR lineage.
pub fn evaluate_search_factors(
    generation: GenerationId,
    factors: &[SearchFactor],
    edges: &[SearchFactorEdge],
    observations: &[SearchObservation],
    limits: SearchFrameworkLimits,
) -> Result<SearchFrameworkReceipt, SearchFrameworkError> {
    validate_budget("factors", factors.len(), limits.max_factors.get())?;
    validate_budget("edges", edges.len(), limits.max_edges.get())?;
    validate_budget(
        "observations",
        observations.len(),
        limits.max_observations.get(),
    )?;
    if factors.is_empty() {
        return Err(SearchFrameworkError::EmptyFactors);
    }

    let factor_index = validate_factors(factors, edges)?;
    let observation_index = validate_observations(&factor_index, edges, observations)?;
    let potential = observations.len().saturating_mul(factors.len());
    validate_budget("influences", potential, limits.max_influences.get())?;

    let raw = run_ascent(edges, observations);
    let adjacency = adjacency(edges);
    let mut influences = Vec::with_capacity(raw.len());
    for (candidate, source, target, support_event, distance) in raw {
        let path = shortest_path(&adjacency, source, target)
            .ok_or(SearchFrameworkError::InternalPathMismatch)?;
        if path.len().saturating_sub(1) != distance {
            return Err(SearchFrameworkError::InternalPathMismatch);
        }
        influences.push(SearchInfluence {
            candidate,
            factor: target,
            support_event,
            result_fact: result_fact_id(candidate, support_event, target),
            factor_path: path,
        });
    }
    influences.sort_by(|left, right| {
        left.candidate
            .cmp(&right.candidate)
            .then(left.factor.cmp(&right.factor))
            .then(left.factor_path.len().cmp(&right.factor_path.len()))
            .then(left.support_event.cmp(&right.support_event))
    });
    influences.dedup_by(|left, right| {
        left.candidate == right.candidate
            && left.factor == right.factor
            && left.support_event == right.support_event
    });

    let total_influence_count = influences.len();
    let status = if total_influence_count > limits.max_results.get() {
        influences.truncate(limits.max_results.get());
        SearchFrameworkStatus::OutputTruncated
    } else {
        SearchFrameworkStatus::Complete
    };
    let lineage = build_lineage(&influences, &observation_index)?;
    let digest = receipt_digest(
        generation,
        status,
        total_influence_count,
        factors,
        edges,
        observations,
        &influences,
    );
    Ok(SearchFrameworkReceipt {
        generation,
        status,
        total_influence_count,
        influences,
        lineage,
        digest,
    })
}

fn validate_budget(
    resource: &'static str,
    required: usize,
    limit: usize,
) -> Result<(), SearchFrameworkError> {
    if required > limit {
        Err(SearchFrameworkError::BudgetExceeded {
            resource,
            required,
            limit,
        })
    } else {
        Ok(())
    }
}

fn validate_factors(
    factors: &[SearchFactor],
    edges: &[SearchFactorEdge],
) -> Result<BTreeMap<QueryOperatorId, SearchFactorRole>, SearchFrameworkError> {
    let mut factor_index = BTreeMap::new();
    for factor in factors {
        if factor_index.insert(factor.id, factor.role).is_some() {
            return Err(SearchFrameworkError::DuplicateFactor(factor.id));
        }
    }
    let mut unique_edges = BTreeSet::new();
    for edge in edges {
        if edge.from == edge.to {
            return Err(SearchFrameworkError::SelfEdge(*edge));
        }
        for factor in [edge.from, edge.to] {
            if !factor_index.contains_key(&factor) {
                return Err(SearchFrameworkError::UnknownFactor(factor));
            }
        }
        if !unique_edges.insert(*edge) {
            return Err(SearchFrameworkError::DuplicateEdge(*edge));
        }
    }
    if factor_graph_has_cycle(factor_index.keys().copied(), &unique_edges) {
        return Err(SearchFrameworkError::FactorCycle);
    }
    Ok(factor_index)
}

fn factor_graph_has_cycle(
    factors: impl Iterator<Item = QueryOperatorId>,
    edges: &BTreeSet<SearchFactorEdge>,
) -> bool {
    let mut indegree = BTreeMap::new();
    let mut outgoing: BTreeMap<QueryOperatorId, Vec<QueryOperatorId>> = BTreeMap::new();
    for factor in factors {
        indegree.insert(factor, 0_usize);
    }
    for edge in edges {
        *indegree.entry(edge.to).or_default() += 1;
        outgoing.entry(edge.from).or_default().push(edge.to);
    }
    let mut ready: BTreeSet<_> = indegree
        .iter()
        .filter_map(|(factor, degree)| (*degree == 0).then_some(*factor))
        .collect();
    let mut visited = 0_usize;
    while let Some(factor) = ready.pop_first() {
        visited += 1;
        for target in outgoing.get(&factor).into_iter().flatten() {
            let degree = indegree
                .get_mut(target)
                .expect("factor graph endpoints were validated");
            *degree -= 1;
            if *degree == 0 {
                ready.insert(*target);
            }
        }
    }
    visited != indegree.len()
}

fn validate_observations(
    factors: &BTreeMap<QueryOperatorId, SearchFactorRole>,
    edges: &[SearchFactorEdge],
    observations: &[SearchObservation],
) -> Result<BTreeMap<FactId, SearchObservation>, SearchFrameworkError> {
    let mut index = BTreeMap::new();
    for observation in observations {
        if !factors.contains_key(&observation.factor) {
            return Err(SearchFrameworkError::UnknownFactor(observation.factor));
        }
        if index.insert(observation.id, observation.clone()).is_some() {
            return Err(SearchFrameworkError::DuplicateObservation(observation.id));
        }
        let mut parents = BTreeSet::new();
        for parent in &observation.causal_parents {
            if !parents.insert(*parent) {
                return Err(SearchFrameworkError::DuplicateCausalParent {
                    observation: observation.id,
                    parent: *parent,
                });
            }
        }
    }
    for observation in observations {
        if observation.causal_parents.is_empty()
            && factors[&observation.factor] != SearchFactorRole::Acquisition
        {
            return Err(SearchFrameworkError::NonAcquisitionRoot(observation.id));
        }
        for parent_id in &observation.causal_parents {
            let Some(parent) = index.get(parent_id) else {
                return Err(SearchFrameworkError::MissingCausalParent {
                    observation: observation.id,
                    parent: *parent_id,
                });
            };
            if parent.logical_position > observation.logical_position {
                return Err(SearchFrameworkError::TemporalOrderViolation {
                    observation: observation.id,
                    parent: *parent_id,
                });
            }
            if parent.factor != observation.factor
                && !edges.contains(&SearchFactorEdge::new(parent.factor, observation.factor))
            {
                return Err(SearchFrameworkError::CausalFactorEdgeMissing {
                    observation: observation.id,
                    parent: *parent_id,
                });
            }
        }
    }
    Ok(index)
}

type RawInfluence = (FactId, QueryOperatorId, QueryOperatorId, FactId, usize);

fn run_ascent(edges: &[SearchFactorEdge], observations: &[SearchObservation]) -> Vec<RawInfluence> {
    ascent! {
        relation factor_edge(QueryOperatorId, QueryOperatorId);
        lattice factor_path(QueryOperatorId, QueryOperatorId, Dual<usize>);
        relation observed(FactId, QueryOperatorId, FactId);
        relation influenced(FactId, QueryOperatorId, QueryOperatorId, FactId, usize);

        factor_path(from, to, Dual(1_usize)) <-- factor_edge(from, to);
        factor_path(from, to, Dual(distance + 1)) <--
            factor_path(from, via, ?Dual(distance)),
            factor_edge(via, to);

        influenced(candidate, factor, factor, event, 0_usize) <--
            observed(candidate, factor, event);
        influenced(candidate, source, target, event, distance) <--
            observed(candidate, source, event),
            factor_path(source, target, ?Dual(distance));
    }

    let mut program = AscentProgram {
        factor_edge: edges.iter().map(|edge| (edge.from, edge.to)).collect(),
        observed: observations
            .iter()
            .map(|observation| (observation.candidate, observation.factor, observation.id))
            .collect(),
        ..AscentProgram::default()
    };
    program.run();
    program.influenced
}

fn adjacency(edges: &[SearchFactorEdge]) -> BTreeMap<QueryOperatorId, Vec<QueryOperatorId>> {
    let mut adjacency: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for edge in edges {
        adjacency.entry(edge.from).or_default().push(edge.to);
    }
    for targets in adjacency.values_mut() {
        targets.sort_unstable();
        targets.dedup();
    }
    adjacency
}

fn shortest_path(
    adjacency: &BTreeMap<QueryOperatorId, Vec<QueryOperatorId>>,
    source: QueryOperatorId,
    target: QueryOperatorId,
) -> Option<Vec<QueryOperatorId>> {
    if source == target {
        return Some(vec![source]);
    }
    let mut predecessor = BTreeMap::new();
    let mut visited = BTreeSet::from([source]);
    let mut pending = VecDeque::from([source]);
    while let Some(current) = pending.pop_front() {
        for next in adjacency.get(&current).into_iter().flatten().copied() {
            if visited.insert(next) {
                predecessor.insert(next, current);
                if next == target {
                    let mut reverse = vec![target];
                    let mut cursor = target;
                    while cursor != source {
                        cursor = predecessor[&cursor];
                        reverse.push(cursor);
                    }
                    reverse.reverse();
                    return Some(reverse);
                }
                pending.push_back(next);
            }
        }
    }
    None
}

fn build_lineage(
    influences: &[SearchInfluence],
    observations: &BTreeMap<FactId, SearchObservation>,
) -> Result<LineageGraph, SearchFrameworkError> {
    let mut nodes = BTreeMap::new();
    let mut edges = BTreeMap::new();
    for influence in influences {
        let observation = &observations[&influence.support_event];
        let source_id = lineage_node_id("source", &[observation.id.to_string()]);
        nodes.entry(source_id).or_insert_with(|| {
            LineageNode::new(source_id, LineageNodeKind::SourceFact(observation.id))
        });
        let mut previous = source_id;
        for (index, factor) in influence.factor_path.iter().copied().enumerate() {
            let common = [
                influence.candidate.to_string(),
                influence.support_event.to_string(),
                factor.to_string(),
                index.to_string(),
            ];
            let operator_id = lineage_node_id("operator", &common);
            nodes.entry(operator_id).or_insert_with(|| {
                LineageNode::new(operator_id, LineageNodeKind::QueryOperator(factor))
            });
            let derived_fact = result_fact_id(influence.candidate, influence.support_event, factor);
            let result_id = lineage_node_id("result", &[derived_fact.to_string()]);
            nodes.entry(result_id).or_insert_with(|| {
                LineageNode::new(result_id, LineageNodeKind::Result(derived_fact))
            });
            insert_lineage_edge(
                &mut edges,
                operator_id,
                previous,
                LineageEdgeKind::DependsOn,
            );
            insert_lineage_edge(
                &mut edges,
                result_id,
                operator_id,
                LineageEdgeKind::ProducedBy,
            );
            previous = result_id;
        }
    }
    LineageGraph::admit(nodes.into_values().collect(), edges.into_values().collect())
        .map_err(SearchFrameworkError::Lineage)
}

fn insert_lineage_edge(
    edges: &mut BTreeMap<LineageEdgeId, LineageEdge>,
    from: LineageNodeId,
    to: LineageNodeId,
    kind: LineageEdgeKind,
) {
    let id = lineage_edge_id(from, to, kind);
    edges
        .entry(id)
        .or_insert_with(|| LineageEdge::new(id, from, to, kind));
}

fn result_fact_id(candidate: FactId, event: FactId, factor: QueryOperatorId) -> FactId {
    FactId::from_canonical_bytes(
        format!("search-result\0{candidate}\0{event}\0{factor}").as_bytes(),
    )
    .expect("framed Search result identity is non-empty")
}

fn lineage_node_id(domain: &str, values: &[String]) -> LineageNodeId {
    let mut canonical = format!("search-lineage-node\0{domain}");
    for value in values {
        canonical.push('\0');
        canonical.push_str(value);
    }
    LineageNodeId::from_canonical_bytes(canonical.as_bytes())
        .expect("framed Search lineage node identity is non-empty")
}

fn lineage_edge_id(from: LineageNodeId, to: LineageNodeId, kind: LineageEdgeKind) -> LineageEdgeId {
    LineageEdgeId::from_canonical_bytes(
        format!("search-lineage-edge\0{from}\0{to}\0{kind:?}").as_bytes(),
    )
    .expect("framed Search lineage edge identity is non-empty")
}

fn receipt_digest(
    generation: GenerationId,
    status: SearchFrameworkStatus,
    total_influence_count: usize,
    factors: &[SearchFactor],
    edges: &[SearchFactorEdge],
    observations: &[SearchObservation],
    influences: &[SearchInfluence],
) -> SearchReasoningDigest {
    let mut hasher = Sha256::new();
    hash_frame(&mut hasher, RECEIPT_SCHEMA);
    hash_frame(&mut hasher, generation.to_string().as_bytes());
    hash_frame(&mut hasher, format!("{status:?}").as_bytes());
    hash_frame(&mut hasher, &total_influence_count.to_le_bytes());
    let mut factors = factors.to_vec();
    factors.sort_unstable();
    for factor in factors {
        hash_frame(&mut hasher, factor.id.to_string().as_bytes());
        hash_frame(&mut hasher, format!("{:?}", factor.role).as_bytes());
    }
    let mut edges = edges.to_vec();
    edges.sort_unstable();
    for edge in edges {
        hash_frame(&mut hasher, edge.from.to_string().as_bytes());
        hash_frame(&mut hasher, edge.to.to_string().as_bytes());
    }
    let mut observations = observations.to_vec();
    observations.sort_by_key(|observation| observation.id);
    for observation in observations {
        hash_frame(&mut hasher, observation.id.to_string().as_bytes());
        hash_frame(&mut hasher, observation.candidate.to_string().as_bytes());
        hash_frame(&mut hasher, observation.factor.to_string().as_bytes());
        hash_frame(&mut hasher, &observation.logical_position.to_le_bytes());
        let mut parents = observation.causal_parents;
        parents.sort_unstable();
        for parent in parents {
            hash_frame(&mut hasher, parent.to_string().as_bytes());
        }
    }
    for influence in influences {
        hash_frame(&mut hasher, influence.candidate.to_string().as_bytes());
        hash_frame(&mut hasher, influence.factor.to_string().as_bytes());
        hash_frame(&mut hasher, influence.support_event.to_string().as_bytes());
        for factor in &influence.factor_path {
            hash_frame(&mut hasher, factor.to_string().as_bytes());
        }
    }
    SearchReasoningDigest(hasher.finalize().into())
}

fn hash_frame(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}
