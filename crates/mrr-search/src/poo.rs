//! POO Flow owns the composition; MRR retains its identities and inference graph.
#[cfg(any(feature = "native-inference", feature = "worker-inference"))]
use crate::SearchFactorRole;
use crate::{SearchFactor, SearchFactorEdge};
#[cfg(any(feature = "native-inference", feature = "worker-inference"))]
pub use mrr_gerbil::{PooSearchPlan, PooSearchRole};
use mrr_identity::{GenerationId, QueryOperatorId};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct PooSearchProjection {
    generation: GenerationId,
    #[cfg(any(feature = "native-inference", feature = "worker-inference"))]
    role_evidence: Option<mrr_gerbil::PooRoleEvidence>,
    factors: Vec<SearchFactor>,
    edges: Vec<SearchFactorEdge>,
    pub(crate) names: BTreeMap<String, SearchFactor>,
    inputs: BTreeMap<String, SearchFactor>,
    dag_receipt: String,
    paths: Vec<(QueryOperatorId, QueryOperatorId, usize)>,
}
impl PooSearchProjection {
    #[must_use]
    pub fn generation(&self) -> GenerationId {
        self.generation
    }
    #[must_use]
    pub fn factors(&self) -> &[SearchFactor] {
        &self.factors
    }
    #[must_use]
    pub fn edges(&self) -> &[SearchFactorEdge] {
        &self.edges
    }
    #[must_use]
    pub fn factor_by_name(&self, name: &str) -> Option<SearchFactor> {
        self.names.get(name).copied()
    }
    #[must_use]
    #[cfg(any(feature = "native-inference", feature = "worker-inference"))]
    pub fn role_evidence(&self) -> Option<&mrr_gerbil::PooRoleEvidence> {
        self.role_evidence.as_ref()
    }
    /// Resolve original canonical stage input without minting a new factor.
    #[must_use]
    pub fn factor_by_input(&self, input: &str) -> Option<SearchFactor> {
        self.inputs.get(input).copied()
    }
    #[must_use]
    pub fn dag_receipt(&self) -> &str {
        &self.dag_receipt
    }
    #[must_use]
    pub fn paths(&self) -> &[(QueryOperatorId, QueryOperatorId, usize)] {
        &self.paths
    }
}
/// Compile through the original POO constructors and MRR Scheme projection.
/// Returned graphs cannot be manufactured through this public Rust type.
///
/// # Errors
/// Returns the POO admission or transport error, including invalid domains,
/// duplicate stage names, or graph identities outside the returned inventory.
#[cfg(any(feature = "native-inference", feature = "worker-inference"))]
pub fn compile_poo_search_plan(
    name: &str,
    generation: GenerationId,
    plan: &PooSearchPlan,
) -> Result<PooSearchProjection, String> {
    let projected = mrr_gerbil::project_poo_search_strategy(name, &generation.to_string(), plan)
        .map_err(|e| e.to_string())?;
    projection_from_graph(name, generation, projected)
}

#[cfg(any(feature = "native-inference", feature = "worker-inference"))]
pub(crate) fn projection_from_graph(
    name: &str,
    generation: GenerationId,
    projected: mrr_gerbil::PooSearchGraph,
) -> Result<PooSearchProjection, String> {
    if projected.generation != generation.to_string() || projected.strategy_name != name {
        return Err("foreign POO controller projection".into());
    }
    let prefix = format!("mrr.search.factor.v1:{name}:");
    let mut names = BTreeMap::new();
    let mut canonical = BTreeMap::new();
    let mut inputs = BTreeMap::new();
    let mut factors = Vec::new();
    for (input, role) in projected.factors {
        let stage = input
            .strip_prefix(&prefix)
            .ok_or("foreign POO factor identity")?;
        let role = match role {
            PooSearchRole::Acquisition => SearchFactorRole::Acquisition,
            PooSearchRole::Refinement => SearchFactorRole::Refinement,
            PooSearchRole::Reasoning => SearchFactorRole::Reasoning,
            PooSearchRole::Projection => SearchFactorRole::Projection,
        };
        let id =
            QueryOperatorId::from_canonical_bytes(input.as_bytes()).map_err(|e| e.to_string())?;
        let factor = SearchFactor::new(id, role);
        if names.insert(stage.to_owned(), factor).is_some() {
            return Err("duplicate POO stage".into());
        }
        inputs.insert(input.clone(), factor);
        canonical.insert(input, id);
        factors.push(factor);
    }
    let edges = projected
        .edges
        .into_iter()
        .map(|(from, to)| {
            Ok(SearchFactorEdge::new(
                *canonical.get(&from).ok_or("foreign POO edge")?,
                *canonical.get(&to).ok_or("foreign POO edge")?,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let paths = projected
        .paths
        .into_iter()
        .map(|(from, to, distance)| {
            Ok((
                *canonical.get(&from).ok_or("foreign POO path")?,
                *canonical.get(&to).ok_or("foreign POO path")?,
                distance,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    if let Some(evidence) = &projected.role_evidence {
        let nodes = evidence
            .graph
            .iter()
            .map(|(id, parents)| (id.as_str(), parents))
            .collect::<BTreeMap<_, _>>();
        let roots = evidence
            .roots
            .iter()
            .map(|(input, root)| (input.as_str(), root.as_str()))
            .collect::<BTreeMap<_, _>>();
        let orders = evidence
            .runtime_orders
            .iter()
            .map(|(input, order)| (input.as_str(), order))
            .collect::<BTreeMap<_, _>>();
        let unique = |items: &[String]| {
            items
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == items.len()
        };
        if nodes.len() != evidence.graph.len()
            || roots.len() != inputs.len()
            || roots.len() != evidence.roots.len()
            || orders.len() != inputs.len()
            || orders.len() != evidence.runtime_orders.len()
            || evidence.graph.iter().any(|(id, parents)| {
                id.is_empty()
                    || !unique(parents)
                    || parents
                        .iter()
                        .any(|parent| !nodes.contains_key(parent.as_str()))
            })
            || inputs.keys().any(|input| {
                !roots.contains_key(input.as_str()) || !orders.contains_key(input.as_str())
            })
            || roots.iter().any(|(input, root)| {
                !nodes.contains_key(root)
                    || orders.get(input).is_none_or(|order| {
                        order.first().map(String::as_str) != Some(*root)
                            || !unique(order)
                            || order.iter().any(|id| !nodes.contains_key(id.as_str()))
                    })
            })
        {
            return Err("foreign or incomplete POO role evidence".into());
        }
    }
    Ok(PooSearchProjection {
        generation,
        role_evidence: projected.role_evidence,
        factors,
        edges,
        names,
        inputs,
        dag_receipt: projected.dag_receipt,
        paths,
    })
}
