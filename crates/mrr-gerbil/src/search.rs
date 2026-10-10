//! Typed host declaration compiled by the existing POO Flow Scheme owner.
use crate::{
    TemporalHost, TemporalRuntimeError,
    native::datum::{self, Value},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PooSearchRole {
    Acquisition,
    Refinement,
    Reasoning,
    Projection,
}
impl PooSearchRole {
    fn label(self) -> &'static str {
        match self {
            Self::Acquisition => "acquisition",
            Self::Refinement => "refinement",
            Self::Reasoning => "reasoning",
            Self::Projection => "projection",
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PooSearchPlan {
    Stage {
        name: String,
        role: PooSearchRole,
        input_domain: String,
        output_domain: String,
    },
    Chain {
        name: String,
        children: Vec<Self>,
    },
    Parallel {
        name: String,
        children: Vec<Self>,
    },
    Merge {
        name: String,
        parallel: Box<Self>,
        stage_name: String,
        role: PooSearchRole,
        output_domain: String,
    },
}
impl PooSearchPlan {
    pub(crate) fn wire(
        &self,
        depth: usize,
        count: &mut usize,
    ) -> Result<Value, TemporalRuntimeError> {
        *count += 1;
        if depth > 24 || *count > 256 {
            return Err(TemporalRuntimeError::InvalidInput);
        }
        let text = |v: &str| Value::String(v.to_owned());
        let row = match self {
            Self::Stage {
                name,
                role,
                input_domain,
                output_domain,
            } => vec![
                text("stage"),
                text(name),
                text(role.label()),
                text(input_domain),
                text(output_domain),
            ],
            Self::Chain { name, children } | Self::Parallel { name, children } => {
                let kind = if matches!(self, Self::Chain { .. }) {
                    "chain"
                } else {
                    "parallel"
                };
                let children = children
                    .iter()
                    .map(|child| child.wire(depth + 1, count))
                    .collect::<Result<Vec<_>, _>>()?;
                vec![text(kind), text(name), Value::List(children)]
            }
            Self::Merge {
                name,
                parallel,
                stage_name,
                role,
                output_domain,
            } => vec![
                text("merge"),
                text(name),
                parallel.wire(depth + 1, count)?,
                text(stage_name),
                text(role.label()),
                text(output_domain),
            ],
        };
        Ok(Value::List(row))
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PooRoleEvidence {
    pub graph: Vec<(String, Vec<String>)>,
    pub roots: Vec<(String, String)>,
    pub runtime_orders: Vec<(String, Vec<String>)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PooSearchGraph {
    pub strategy_name: String,
    pub generation: String,
    pub factors: Vec<(String, PooSearchRole)>,
    pub edges: Vec<(String, String)>,
    pub dag_receipt: String,
    pub paths: Vec<(String, String, usize)>,
    pub role_evidence: Option<PooRoleEvidence>,
}
/// Build real POO stage/composition objects, project their graph and retain the DAG receipt.
/// No Rust implementation of POO type checking or graph compilation is selected.
///
/// # Errors
/// Returns an error when the plan exceeds transport limits, the POO owner
/// rejects its composition, or the returned graph cannot be decoded.
pub fn project_poo_search_strategy(
    name: &str,
    generation: &str,
    plan: &PooSearchPlan,
) -> Result<PooSearchGraph, TemporalRuntimeError> {
    let request = Value::List(vec![
        Value::String(name.into()),
        Value::String(generation.into()),
        plan.wire(0, &mut 0)?,
    ])
    .encode();
    let bytes = TemporalHost.compile_search_projection(request.as_bytes())?;
    decode_projection(name, generation, &bytes)
}

pub(crate) fn decode_projection(
    name: &str,
    generation: &str,
    bytes: &[u8],
) -> Result<PooSearchGraph, TemporalRuntimeError> {
    let invalid = || TemporalRuntimeError::InvalidInput;
    let value = datum::decode(bytes).ok_or_else(invalid)?;
    let rows = value.as_array().ok_or_else(invalid)?;
    if !matches!(rows.len(), 7 | 8)
        || rows[0].as_str() != Some("mrr.poo.search.projection.v1")
        || rows[1].as_str() != Some(name)
        || rows[2].as_str() != Some(generation)
    {
        return Err(invalid());
    }
    let factors = rows[3]
        .as_array()
        .ok_or_else(invalid)?
        .iter()
        .map(|row| {
            let row = row.as_array().ok_or_else(invalid)?;
            if row.len() != 2 {
                return Err(invalid());
            }
            let role = match row[1].as_str() {
                Some("acquisition") => PooSearchRole::Acquisition,
                Some("refinement") => PooSearchRole::Refinement,
                Some("reasoning") => PooSearchRole::Reasoning,
                Some("projection") => PooSearchRole::Projection,
                _ => return Err(invalid()),
            };
            Ok((row[0].as_str().ok_or_else(invalid)?.to_owned(), role))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let edges = rows[4]
        .as_array()
        .ok_or_else(invalid)?
        .iter()
        .map(|row| {
            let row = row.as_array().ok_or_else(invalid)?;
            if row.len() != 2 {
                return Err(invalid());
            }
            Ok((
                row[0].as_str().ok_or_else(invalid)?.to_owned(),
                row[1].as_str().ok_or_else(invalid)?.to_owned(),
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let dag_receipt = rows[5].as_str().ok_or_else(invalid)?.to_owned();
    let paths = rows[6]
        .as_array()
        .ok_or_else(invalid)?
        .iter()
        .map(|row| {
            let row = row.as_array().ok_or_else(invalid)?;
            if row.len() != 3 {
                return Err(invalid());
            }
            Ok((
                row[0].as_str().ok_or_else(invalid)?.to_owned(),
                row[1].as_str().ok_or_else(invalid)?.to_owned(),
                row[2].integer().ok_or_else(invalid)?,
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if factors.is_empty() || factors.len() > 256 || dag_receipt.is_empty() {
        return Err(invalid());
    }
    let role_evidence = rows
        .get(7)
        .map(|value| {
            let rows = value.as_array().ok_or_else(invalid)?;
            if rows.len() != 4 || rows[0].as_str() != Some("mrr.poo.search.roles.v1") {
                return Err(invalid());
            }
            let strings = |value: &Value| -> Result<Vec<String>, TemporalRuntimeError> {
                value
                    .as_array()
                    .ok_or_else(invalid)?
                    .iter()
                    .map(|item| item.as_str().map(str::to_owned).ok_or_else(invalid))
                    .collect()
            };
            let lists =
                |value: &Value| -> Result<Vec<(String, Vec<String>)>, TemporalRuntimeError> {
                    value
                        .as_array()
                        .ok_or_else(invalid)?
                        .iter()
                        .map(|item| {
                            let pair = item.as_array().ok_or_else(invalid)?;
                            if pair.len() != 2 {
                                return Err(invalid());
                            }
                            Ok((
                                pair[0].as_str().ok_or_else(invalid)?.to_owned(),
                                strings(&pair[1])?,
                            ))
                        })
                        .collect()
                };
            let graph = lists(&rows[1])?;
            let roots = rows[2]
                .as_array()
                .ok_or_else(invalid)?
                .iter()
                .map(|item| {
                    let pair = item.as_array().ok_or_else(invalid)?;
                    if pair.len() != 2 {
                        return Err(invalid());
                    }
                    Ok((
                        pair[0].as_str().ok_or_else(invalid)?.to_owned(),
                        pair[1].as_str().ok_or_else(invalid)?.to_owned(),
                    ))
                })
                .collect::<Result<Vec<_>, TemporalRuntimeError>>()?;
            let runtime_orders = lists(&rows[3])?;
            if graph.is_empty()
                || graph.len() > 512
                || roots.len() != factors.len()
                || runtime_orders.len() != factors.len()
            {
                return Err(invalid());
            }
            Ok(PooRoleEvidence {
                graph,
                roots,
                runtime_orders,
            })
        })
        .transpose()?;
    Ok(PooSearchGraph {
        strategy_name: name.into(),
        generation: generation.into(),
        factors,
        edges,
        dag_receipt,
        paths,
        role_evidence,
    })
}
