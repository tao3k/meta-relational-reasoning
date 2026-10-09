//! Original parser buffers and Scheme finite rows; parent repeats admission.
use crate::{
    FiniteInferenceCandidate, FiniteInferenceError, NativeWorker, NativeWorkerError, ParseArtifact,
    ParseArtifactLoadError, ParserLanguage,
    native::{
        datum::{self, Value},
        parse_artifact,
    },
};
use std::sync::Arc;
pub(crate) fn list(value: &Value) -> Result<&Vec<Value>, NativeWorkerError> {
    value.as_array().ok_or(NativeWorkerError::Protocol)
}
pub(crate) fn text(value: &Value) -> Result<&str, NativeWorkerError> {
    value.as_str().ok_or(NativeWorkerError::Protocol)
}
pub(crate) fn decode(bytes: &[u8]) -> Result<Value, NativeWorkerError> {
    datum::decode(bytes).ok_or(NativeWorkerError::Protocol)
}
fn rows<const N: usize>(value: &Value) -> Result<Vec<[usize; N]>, NativeWorkerError> {
    list(value)?
        .iter()
        .map(|row| {
            let row = list(row)?;
            if row.len() != N {
                return Err(NativeWorkerError::Protocol);
            }
            let mut cells = [0; N];
            for (cell, value) in cells.iter_mut().zip(row) {
                *cell = value.integer().ok_or(NativeWorkerError::Protocol)?;
            }
            Ok(cells)
        })
        .collect()
}
fn numbers<const N: usize>(rows: impl IntoIterator<Item = [usize; N]>) -> Value {
    Value::List(
        rows.into_iter()
            .map(|row| Value::List(row.into_iter().map(Value::Integer).collect()))
            .collect(),
    )
}
impl NativeWorker {
    pub fn parse_artifact(
        &mut self,
        language: ParserLanguage,
        source: &str,
    ) -> Result<ParseArtifact, NativeWorkerError> {
        let result = self.parse_admitted(language, source);
        if matches!(&result, Err(NativeWorkerError::Protocol))
            || matches!(&result, Err(NativeWorkerError::Parser(e)) if !matches!(e, ParseArtifactLoadError::ParserFailed {..} | ParseArtifactLoadError::InteriorNul))
        {
            self.cancel();
        }
        result
    }
    fn parse_admitted(
        &mut self,
        language: ParserLanguage,
        source: &str,
    ) -> Result<ParseArtifact, NativeWorkerError> {
        let bytes = self.invoke(
            if language == ParserLanguage::Gql {
                5
            } else {
                6
            },
            source.as_bytes(),
        )?;
        let value = decode(&bytes)?;
        let fields = list(&value)?;
        if fields.len() != 2 {
            return Err(NativeWorkerError::Protocol);
        }
        let hex = text(&fields[0])?;
        if hex.len() % 2 != 0 {
            return Err(NativeWorkerError::Protocol);
        }
        let payload = hex
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                let s = std::str::from_utf8(pair).map_err(|_| NativeWorkerError::Protocol)?;
                u8::from_str_radix(s, 16).map_err(|_| NativeWorkerError::Protocol)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let catalog = Arc::new(
            parse_artifact::load_kind_catalog(&fields[1], language)
                .map_err(NativeWorkerError::Parser)?,
        );
        let artifact = parse_artifact::decode_parse_artifact(&payload, source, catalog)
            .map_err(NativeWorkerError::Parser)?;
        parse_artifact::validate_source_digest(&artifact, source)
            .map_err(NativeWorkerError::Parser)?;
        Ok(artifact)
    }
    pub fn evaluate_finite_relations(
        &mut self,
        nodes: usize,
        edges: Vec<(usize, usize)>,
        observations: Vec<usize>,
    ) -> Result<FiniteInferenceCandidate, NativeWorkerError> {
        let result = self.finite_admitted(nodes, edges, observations);
        if matches!(
            result,
            Err(NativeWorkerError::Protocol)
                | Err(NativeWorkerError::Finite(
                    FiniteInferenceError::InvalidNativeCandidate
                ))
        ) {
            self.cancel();
        }
        result
    }
    fn finite_admitted(
        &mut self,
        nodes: usize,
        edges: Vec<(usize, usize)>,
        observations: Vec<usize>,
    ) -> Result<FiniteInferenceCandidate, NativeWorkerError> {
        if edges.iter().any(|&(a, b)| a >= nodes || b >= nodes)
            || observations.iter().any(|&n| n >= nodes)
        {
            return Err(NativeWorkerError::Finite(FiniteInferenceError::ForeignNode));
        }
        let request = Value::List(vec![
            Value::Integer(nodes),
            numbers(edges.into_iter().map(|(a, b)| [a, b])),
            Value::List(observations.iter().copied().map(Value::Integer).collect()),
        ])
        .encode();
        let value = decode(&self.invoke(7, request.as_bytes())?)?;
        let fields = list(&value)?;
        if fields.len() != 2 {
            return Err(NativeWorkerError::Protocol);
        }
        let result = crate::native::finite::admit_candidate(
            nodes,
            &observations,
            rows(&fields[0])?,
            rows(&fields[1])?,
        )
        .map_err(NativeWorkerError::Finite);
        if result.is_err() {
            self.cancel();
        }
        result
    }
}
#[cfg(feature = "embedded-runtime")]
pub(crate) fn parser_request(operation: i32, source: &[u8]) -> Result<Vec<u8>, NativeWorkerError> {
    let source = std::str::from_utf8(source).map_err(|_| NativeWorkerError::InvalidInput)?;
    let (payload, catalog) = parse_artifact::request_parse_artifact(
        if operation == 5 {
            ParserLanguage::Gql
        } else {
            ParserLanguage::Cypher
        },
        source,
    )
    .map_err(NativeWorkerError::Parser)?;
    let hex = payload
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    Ok(
        Value::List(vec![Value::String(hex), catalog.descriptor.clone()])
            .encode()
            .into_bytes(),
    )
}
#[cfg(feature = "embedded-runtime")]
pub(crate) fn finite_request(bytes: &[u8]) -> Result<Vec<u8>, NativeWorkerError> {
    let value = decode(bytes)?;
    let fields = list(&value)?;
    if fields.len() != 3 {
        return Err(NativeWorkerError::Protocol);
    }
    let nodes = fields[0].integer().ok_or(NativeWorkerError::Protocol)?;
    let edges = rows::<2>(&fields[1])?
        .into_iter()
        .map(|[a, b]| (a, b))
        .collect();
    let observations = list(&fields[2])?
        .iter()
        .map(|v| v.integer().ok_or(NativeWorkerError::Protocol))
        .collect::<Result<Vec<_>, _>>()?;
    let candidate = crate::evaluate_finite_relations(nodes, edges, observations)
        .map_err(NativeWorkerError::Finite)?;
    Ok(Value::List(vec![
        numbers(candidate.paths.into_iter().map(|(a, b, c)| [a, b, c])),
        numbers(
            candidate
                .influences
                .into_iter()
                .map(|(a, b, c, d)| [a, b, c, d]),
        ),
    ])
    .encode()
    .into_bytes())
}
