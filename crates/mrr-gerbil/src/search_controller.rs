//! Retained Scheme control authority; Rust transports decisions and request identities.
use crate::native::datum::{self, Value};
use crate::{PooSearchGraph, PooSearchPlan, TemporalHost, TemporalRuntimeError};
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

pub struct PooSearchController {
    key: String,
    generation: String,
    configuration: String,
    graph: PooSearchGraph,
}
#[derive(Debug)]
pub struct PooSearchRequest {
    key: String,
    generation: String,
    configuration: String,
    stage: String,
    source_cut: String,
    revision: usize,
    attempt: usize,
}
/// A transport projection of one Temporal event; Scheme owns semantic admission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PooSearchObservation {
    pub identity: String,
    pub generation: String,
    pub stage: String,
    pub source_cut: String,
    pub logical_position: usize,
    pub payload_identity: String,
    pub causal_parents: Vec<String>,
    pub modality: PooSearchModality,
    pub committed: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PooSearchModality {
    Observed,
    Derived,
    Hypothesized,
}
impl PooSearchObservation {
    fn wire(&self) -> Value {
        Value::List(vec![
            text(&self.identity),
            text(&self.generation),
            text(&self.stage),
            text(&self.source_cut),
            Value::Integer(self.logical_position),
            text(&self.payload_identity),
            Value::List(self.causal_parents.iter().map(|s| text(s)).collect()),
            text(match self.modality {
                PooSearchModality::Observed => "observed",
                PooSearchModality::Derived => "derived",
                PooSearchModality::Hypothesized => "hypothesized",
            }),
            Value::Bool(self.committed),
        ])
    }
    fn decode(value: &Value) -> Result<Self, TemporalRuntimeError> {
        let r = value
            .as_array()
            .filter(|r| r.len() == 9)
            .ok_or(TemporalRuntimeError::InvalidInput)?;
        let string = |i: usize| {
            r[i].as_str()
                .map(str::to_owned)
                .ok_or(TemporalRuntimeError::InvalidInput)
        };
        Ok(Self {
            identity: string(0)?,
            generation: string(1)?,
            stage: string(2)?,
            source_cut: string(3)?,
            logical_position: r[4].integer().ok_or(TemporalRuntimeError::InvalidInput)?,
            payload_identity: string(5)?,
            causal_parents: names(r[6].clone())?,
            modality: match r[7].as_str() {
                Some("observed") => PooSearchModality::Observed,
                Some("derived") => PooSearchModality::Derived,
                _ => return Err(TemporalRuntimeError::InvalidInput),
            },
            committed: match r[8] {
                Value::Bool(b) => b,
                _ => return Err(TemporalRuntimeError::InvalidInput),
            },
        })
    }
}
impl PooSearchRequest {
    #[must_use]
    pub fn stage(&self) -> &str {
        &self.stage
    }
    #[must_use]
    pub fn revision(&self) -> usize {
        self.revision
    }
    #[must_use]
    pub fn attempt(&self) -> usize {
        self.attempt
    }
}
impl PooSearchController {
    /// Creates one bounded owner session from actual POO constructors.
    /// # Errors
    /// Rejects invalid plans, transport bounds, unavailable owners or session exhaustion.
    pub fn new(
        name: &str,
        generation: &str,
        configuration: &str,
        source_cut: &str,
        plan: &PooSearchPlan,
    ) -> Result<Self, TemporalRuntimeError> {
        Self::start(name, generation, configuration, source_cut, plan, "start")
    }
    /// Creates a session that requires Temporal evidence for every completion.
    /// # Errors
    /// Rejects invalid plans, bounded transport failures and unavailable owners.
    pub fn with_evidence(
        name: &str,
        generation: &str,
        configuration: &str,
        source_cut: &str,
        plan: &PooSearchPlan,
    ) -> Result<Self, TemporalRuntimeError> {
        Self::start(
            name,
            generation,
            configuration,
            source_cut,
            plan,
            "start-evidence",
        )
    }
    fn start(
        name: &str,
        generation: &str,
        configuration: &str,
        source_cut: &str,
        plan: &PooSearchPlan,
        command: &str,
    ) -> Result<Self, TemporalRuntimeError> {
        if configuration.is_empty()
            || configuration.len() > 4096
            || source_cut.is_empty()
            || source_cut.len() > 4096
            || generation.is_empty()
            || generation.len() > 128
        {
            return Err(TemporalRuntimeError::InvalidInput);
        }
        let id = NEXT_SESSION
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| TemporalRuntimeError::InvalidInput)?;
        let key = format!("mrr.search.session.v1:{id}");
        let started = call(
            &key,
            generation,
            configuration,
            command,
            vec![text(name), plan.wire(0, &mut 0)?, text(source_cut)],
        );
        let decoded = started.and_then(|value| {
            let rows = value
                .as_array()
                .filter(|v| v.len() == 2)
                .ok_or(TemporalRuntimeError::InvalidInput)?;
            crate::search::decode_projection(
                name,
                generation,
                rows[0]
                    .as_str()
                    .ok_or(TemporalRuntimeError::InvalidInput)?
                    .as_bytes(),
            )
        });
        let graph = match decoded {
            Ok(graph) => graph,
            Err(error) => {
                // Start may have committed before an acknowledgement was lost.
                // Release this nonce without touching any other owner's session.
                let _ = call(&key, generation, configuration, "release", vec![]);
                return Err(error);
            }
        };
        Ok(Self {
            key,
            generation: generation.into(),
            configuration: configuration.into(),
            graph,
        })
    }
    #[must_use]
    pub fn graph(&self) -> &PooSearchGraph {
        &self.graph
    }
    /// # Errors
    /// Propagates released sessions, transport failure or invalid owner decisions.
    pub fn frontier(&self) -> Result<Vec<String>, TemporalRuntimeError> {
        names(self.invoke("frontier", vec![])?)
    }
    /// # Errors
    /// Rejects stages that are already active, complete, foreign or missing prerequisites.
    pub fn issue(&self, stage: &str) -> Result<PooSearchRequest, TemporalRuntimeError> {
        let value = self.invoke("issue", vec![text(stage)])?;
        let rows = value
            .as_array()
            .filter(|v| v.len() == 4)
            .ok_or(TemporalRuntimeError::InvalidInput)?;
        if rows[0].as_str() != Some(stage) {
            return Err(TemporalRuntimeError::InvalidInput);
        }
        Ok(PooSearchRequest {
            key: self.key.clone(),
            generation: self.generation.clone(),
            configuration: self.configuration.clone(),
            stage: stage.into(),
            source_cut: rows[1]
                .as_str()
                .ok_or(TemporalRuntimeError::InvalidInput)?
                .into(),
            revision: rows[2]
                .integer()
                .ok_or(TemporalRuntimeError::InvalidInput)?,
            attempt: rows[3]
                .integer()
                .ok_or(TemporalRuntimeError::InvalidInput)?,
        })
    }
    /// # Errors
    /// Rejects stale, duplicate or foreign request identities at the Scheme owner.
    pub fn complete(
        &self,
        request: &PooSearchRequest,
    ) -> Result<Vec<String>, TemporalRuntimeError> {
        self.finish("complete", request)
    }
    /// Cancels only the matching active attempt; an old cancellation is a no-op.
    /// # Errors
    /// Propagates transport failure or a foreign session.
    pub fn cancel(&self, request: &PooSearchRequest) -> Result<Vec<String>, TemporalRuntimeError> {
        self.finish("cancel", request)
    }
    /// # Errors
    /// Rejects foreign changed stages or an invalid source cut.
    pub fn revise(
        &self,
        changed: &[String],
        source_cut: &str,
    ) -> Result<Vec<String>, TemporalRuntimeError> {
        names(self.invoke(
            "revise",
            vec![
                Value::List(changed.iter().map(|s| text(s)).collect()),
                text(source_cut),
            ],
        )?)
    }
    /// Returns the actual current predecessor observations for this active request.
    /// # Errors
    /// Rejects stale/foreign requests, missing evidence and identity-only sessions.
    pub fn inputs(
        &self,
        request: &PooSearchRequest,
    ) -> Result<Vec<PooSearchObservation>, TemporalRuntimeError> {
        let value = self.invoke("inputs", self.request_args(request)?)?;
        value
            .as_array()
            .ok_or(TemporalRuntimeError::InvalidInput)?
            .iter()
            .map(PooSearchObservation::decode)
            .collect()
    }
    /// Transports an observation to the POO owner without deriving causal parents in Rust.
    /// # Errors
    /// Rejects foreign/stale scope, bad causal evidence and reused event identities.
    pub fn observe(
        &self,
        request: &PooSearchRequest,
        observation: &PooSearchObservation,
    ) -> Result<Vec<String>, TemporalRuntimeError> {
        let mut args = self.request_args(request)?;
        args.push(observation.wire());
        names(self.invoke("observe", args)?)
    }
    fn request_args(&self, request: &PooSearchRequest) -> Result<Vec<Value>, TemporalRuntimeError> {
        if request.key != self.key
            || request.generation != self.generation
            || request.configuration != self.configuration
        {
            return Err(TemporalRuntimeError::InvalidInput);
        }
        Ok(vec![
            text(&request.stage),
            text(&request.source_cut),
            Value::Integer(request.revision),
            Value::Integer(request.attempt),
        ])
    }
    fn finish(
        &self,
        command: &str,
        request: &PooSearchRequest,
    ) -> Result<Vec<String>, TemporalRuntimeError> {
        if request.key != self.key
            || request.generation != self.generation
            || request.configuration != self.configuration
        {
            return Err(TemporalRuntimeError::InvalidInput);
        }
        names(self.invoke(
            command,
            vec![
                text(&request.stage),
                text(&request.source_cut),
                Value::Integer(request.revision),
                Value::Integer(request.attempt),
            ],
        )?)
    }
    fn invoke(&self, command: &str, args: Vec<Value>) -> Result<Value, TemporalRuntimeError> {
        call(
            &self.key,
            &self.generation,
            &self.configuration,
            command,
            args,
        )
    }
}
impl Drop for PooSearchController {
    fn drop(&mut self) {
        let _ = self.invoke("release", vec![]);
    }
}
fn text(value: &str) -> Value {
    Value::String(value.into())
}
fn names(value: Value) -> Result<Vec<String>, TemporalRuntimeError> {
    value
        .as_array()
        .ok_or(TemporalRuntimeError::InvalidInput)?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_owned)
                .ok_or(TemporalRuntimeError::InvalidInput)
        })
        .collect()
}
fn call(
    key: &str,
    generation: &str,
    configuration: &str,
    command: &str,
    args: Vec<Value>,
) -> Result<Value, TemporalRuntimeError> {
    let mut row = vec![
        text(command),
        text(key),
        text(generation),
        text(configuration),
    ];
    row.extend(args);
    let bytes = TemporalHost.search_engine(Value::List(row).encode().as_bytes())?;
    let value = datum::decode(&bytes).ok_or(TemporalRuntimeError::InvalidInput)?;
    let row = value
        .as_array()
        .filter(|v| v.len() == 5)
        .ok_or(TemporalRuntimeError::InvalidInput)?;
    if row[0].as_str() != Some("mrr.poo.search.engine.v1")
        || row[1].as_str() != Some(key)
        || row[2].as_str() != Some(generation)
        || row[3].as_str() != Some(configuration)
    {
        return Err(TemporalRuntimeError::InvalidInput);
    }
    Ok(row[4].clone())
}
