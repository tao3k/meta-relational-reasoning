//! Typed Rust ownership of parser-published ParseArtifact v1 events.

use std::collections::BTreeMap;
use std::ffi::CString;
use std::sync::{Arc, OnceLock};

use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{
    ffi,
    runtime::{NativeRuntimeError, with_native_runtime},
};

/// Canonical parser result schema. Its version is owned by `gerbil-parser`.
pub const PARSE_ARTIFACT_SCHEMA_V1: &str = "gerbil-parser.parse-artifact.v1";
/// Canonical parser-owned native descriptor schema.
pub const PARSER_NATIVE_DESCRIPTOR_SCHEMA_V1: &str = "gerbil-parser.native-descriptor.v1";

/// Parser-owned category for one stable syntax kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParserKindCategory {
    Node,
    Token,
}

/// One grammar-declared syntax kind in canonical declaration order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParserKindSpec {
    name: String,
    category: ParserKindCategory,
    fields: Vec<String>,
}

impl ParserKindSpec {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn category(&self) -> ParserKindCategory {
        self.category
    }

    #[must_use]
    pub fn fields(&self) -> &[String] {
        &self.fields
    }
}

/// Stable parser-owned catalog used to assign Rowan-compatible numeric kinds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParserKindCatalog {
    grammar_digest: String,
    kinds: Vec<ParserKindSpec>,
    by_name: BTreeMap<String, u16>,
    terminals: BTreeMap<String, u16>,
}

impl ParserKindCatalog {
    #[must_use]
    pub fn grammar_digest(&self) -> &str {
        &self.grammar_digest
    }

    #[must_use]
    pub fn kinds(&self) -> &[ParserKindSpec] {
        &self.kinds
    }

    #[must_use]
    pub fn kind_id(&self, name: &str) -> Option<u16> {
        self.by_name.get(name).copied()
    }

    /// Resolves one parser-owned Rowan kind without introducing a second enum.
    #[must_use]
    pub fn kind_name(&self, id: u16) -> Option<&str> {
        self.kinds.get(usize::from(id)).map(ParserKindSpec::name)
    }

    #[must_use]
    pub fn terminal_kind_id(&self, terminal: &str) -> Option<u16> {
        self.terminals.get(terminal).copied()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseArtifactStatus {
    Accepted,
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseEvent {
    StartNode {
        id: u64,
        kind: String,
        start: u32,
    },
    FinishNode {
        id: u64,
        kind: String,
        end: u32,
    },
    StartField {
        field: String,
        start: u32,
    },
    FinishField {
        field: String,
        end: u32,
    },
    Token {
        id: u64,
        kind: String,
        lexeme: String,
        start: u32,
        end: u32,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseArtifact {
    pub schema: String,
    pub kind_catalog: Arc<ParserKindCatalog>,
    pub status: ParseArtifactStatus,
    pub grammar_digest: String,
    pub source_digest: String,
    pub events: Vec<ParseEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseArtifactLoadError {
    InteriorNul,
    RuntimeUnavailable,
    RuntimeStatus(i32),
    ParserFailed {
        call_status: i32,
        result_status: i32,
        diagnostic: Option<String>,
    },
    InvalidPayload,
    InvalidSchema(String),
    InvalidGrammarDigest(String),
    InvalidSourceDigest(String),
    NativeDescriptorFailed {
        call_status: i32,
        result_status: i32,
        diagnostic: Option<String>,
    },
    InvalidHostDescriptor,
    MissingField(&'static str),
    InvalidStatus(String),
    InvalidEvent {
        row: i64,
        kind: String,
    },
    InvalidNumber {
        row: i64,
        column: &'static str,
        value: i64,
    },
}

static PARSER_KIND_CATALOG: OnceLock<Result<Arc<ParserKindCatalog>, ParseArtifactLoadError>> =
    OnceLock::new();

pub fn parse_gql_artifact(source: &str) -> Result<ParseArtifact, ParseArtifactLoadError> {
    let (payload, kind_catalog) = request_parse_artifact(source)?;
    let artifact = decode_parse_artifact(&payload, kind_catalog)?;
    validate_source_digest(&artifact, source)?;
    Ok(artifact)
}

pub(crate) fn validate_source_digest(
    artifact: &ParseArtifact,
    source: &str,
) -> Result<(), ParseArtifactLoadError> {
    let expected = format!("sha256:{:x}", Sha256::digest(source.as_bytes()));
    if artifact.source_digest != expected {
        return Err(ParseArtifactLoadError::InvalidSourceDigest(
            artifact.source_digest.clone(),
        ));
    }
    Ok(())
}

pub(crate) fn decode_parse_artifact(
    payload: &Value,
    kind_catalog: Arc<ParserKindCatalog>,
) -> Result<ParseArtifact, ParseArtifactLoadError> {
    let schema = string_field(payload, "schema")?.to_owned();
    if schema != PARSE_ARTIFACT_SCHEMA_V1 {
        return Err(ParseArtifactLoadError::InvalidSchema(schema));
    }
    let status_text = string_field(payload, "status")?.to_owned();
    let status = match status_text.as_str() {
        "accepted" => ParseArtifactStatus::Accepted,
        "rejected" => ParseArtifactStatus::Rejected,
        _ => return Err(ParseArtifactLoadError::InvalidStatus(status_text)),
    };
    let grammar_digest = string_field(payload, "grammarDigest")?.to_owned();
    if grammar_digest != kind_catalog.grammar_digest {
        return Err(ParseArtifactLoadError::InvalidGrammarDigest(grammar_digest));
    }
    let source_digest = string_field(payload, "sourceDigest")?.to_owned();
    let event_values = payload
        .get("events")
        .and_then(Value::as_array)
        .ok_or(ParseArtifactLoadError::MissingField("events"))?;
    let events = event_values
        .iter()
        .enumerate()
        .map(|(row, event)| load_event(row as i64, event))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ParseArtifact {
        schema,
        kind_catalog,
        status,
        grammar_digest,
        source_digest,
        events,
    })
}

fn request_parse_artifact(
    source: &str,
) -> Result<(Value, Arc<ParserKindCatalog>), ParseArtifactLoadError> {
    let kind_catalog = PARSER_KIND_CATALOG
        .get_or_init(load_native_kind_catalog)
        .clone()?;
    let source = CString::new(source).map_err(|_| ParseArtifactLoadError::InteriorNul)?;
    let native = with_native_runtime(move || ffi::parser_native_parse(&source))
        .map_err(parse_runtime_error)?;
    let payload = native_payload(native, |call_status, result_status, diagnostic| {
        ParseArtifactLoadError::ParserFailed {
            call_status,
            result_status,
            diagnostic,
        }
    })?;
    let payload =
        serde_json::from_slice(&payload).map_err(|_| ParseArtifactLoadError::InvalidPayload)?;
    Ok((payload, kind_catalog))
}

fn load_native_kind_catalog() -> Result<Arc<ParserKindCatalog>, ParseArtifactLoadError> {
    let (abi, native) = with_native_runtime(|| {
        (
            ffi::parser_native_abi_version(),
            ffi::parser_native_descriptor(),
        )
    })
    .map_err(parse_runtime_error)?;
    if abi != 1 {
        return Err(ParseArtifactLoadError::InvalidHostDescriptor);
    }
    let payload = native_payload(native, |call_status, result_status, diagnostic| {
        ParseArtifactLoadError::NativeDescriptorFailed {
            call_status,
            result_status,
            diagnostic,
        }
    })?;
    let descriptor = serde_json::from_slice(&payload)
        .map_err(|_| ParseArtifactLoadError::InvalidHostDescriptor)?;
    load_kind_catalog(&descriptor).map(Arc::new)
}

fn parse_runtime_error(error: NativeRuntimeError) -> ParseArtifactLoadError {
    match error {
        NativeRuntimeError::Unavailable => ParseArtifactLoadError::RuntimeUnavailable,
        NativeRuntimeError::Status(status) => ParseArtifactLoadError::RuntimeStatus(status),
    }
}

fn native_payload(
    result: ffi::ParserNativeResult,
    failure: impl FnOnce(i32, i32, Option<String>) -> ParseArtifactLoadError,
) -> Result<Vec<u8>, ParseArtifactLoadError> {
    if result.call_status != 0 || result.result_status != 0 {
        let diagnostic = result
            .payload
            .as_deref()
            .map(String::from_utf8_lossy)
            .map(std::borrow::Cow::into_owned);
        return Err(failure(
            result.call_status,
            result.result_status,
            diagnostic,
        ));
    }
    result
        .payload
        .ok_or_else(|| failure(result.call_status, result.result_status, None))
}

pub(crate) fn load_kind_catalog(
    payload: &Value,
) -> Result<ParserKindCatalog, ParseArtifactLoadError> {
    if string_field(payload, "schema")? != PARSER_NATIVE_DESCRIPTOR_SCHEMA_V1 {
        return Err(ParseArtifactLoadError::InvalidHostDescriptor);
    }
    let grammar_digest = string_field(payload, "grammarDigest")?.to_owned();
    if !valid_sha256_digest(&grammar_digest) {
        return Err(ParseArtifactLoadError::InvalidHostDescriptor);
    }
    let rows = payload
        .get("syntaxKinds")
        .and_then(Value::as_array)
        .ok_or(ParseArtifactLoadError::InvalidHostDescriptor)?;
    let mut kinds = Vec::with_capacity(rows.len());
    let mut by_name = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        let row = row
            .as_array()
            .ok_or(ParseArtifactLoadError::InvalidHostDescriptor)?;
        let id = u16::try_from(index).map_err(|_| ParseArtifactLoadError::InvalidHostDescriptor)?;
        let name = event_string(row, 0, "syntaxKind.name")?.to_owned();
        let category = match event_string(row, 1, "syntaxKind.category")? {
            "node" => ParserKindCategory::Node,
            "token" => ParserKindCategory::Token,
            _ => return Err(ParseArtifactLoadError::InvalidHostDescriptor),
        };
        let fields = row
            .get(2)
            .and_then(Value::as_array)
            .ok_or(ParseArtifactLoadError::InvalidHostDescriptor)?
            .iter()
            .map(|field| {
                field
                    .as_str()
                    .map(str::to_owned)
                    .ok_or(ParseArtifactLoadError::InvalidHostDescriptor)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if by_name.insert(name.clone(), id).is_some() {
            return Err(ParseArtifactLoadError::InvalidHostDescriptor);
        }
        kinds.push(ParserKindSpec {
            name,
            category,
            fields,
        });
    }
    let terminal_rows = payload
        .get("terminals")
        .and_then(Value::as_array)
        .ok_or(ParseArtifactLoadError::InvalidHostDescriptor)?;
    let mut terminals = BTreeMap::new();
    for row in terminal_rows {
        let row = row
            .as_array()
            .ok_or(ParseArtifactLoadError::InvalidHostDescriptor)?;
        let terminal = event_string(row, 0, "terminal.name")?.to_owned();
        let kind_name = event_string(row, 1, "terminal.kind")?;
        let id = by_name
            .get(kind_name)
            .copied()
            .ok_or(ParseArtifactLoadError::InvalidHostDescriptor)?;
        if kinds[usize::from(id)].category != ParserKindCategory::Token
            || terminals.insert(terminal, id).is_some()
        {
            return Err(ParseArtifactLoadError::InvalidHostDescriptor);
        }
    }
    Ok(ParserKindCatalog {
        grammar_digest,
        kinds,
        by_name,
        terminals,
    })
}

fn valid_sha256_digest(digest: &str) -> bool {
    digest
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn string_field<'a>(
    payload: &'a Value,
    name: &'static str,
) -> Result<&'a str, ParseArtifactLoadError> {
    payload
        .get(name)
        .and_then(Value::as_str)
        .ok_or(ParseArtifactLoadError::MissingField(name))
}

fn event_string<'a>(
    event: &'a [Value],
    column: usize,
    name: &'static str,
) -> Result<&'a str, ParseArtifactLoadError> {
    event
        .get(column)
        .and_then(Value::as_str)
        .ok_or(ParseArtifactLoadError::MissingField(name))
}

fn number(
    row: i64,
    event: &[Value],
    column: usize,
    name: &'static str,
) -> Result<u64, ParseArtifactLoadError> {
    event
        .get(column)
        .and_then(Value::as_u64)
        .ok_or(ParseArtifactLoadError::InvalidNumber {
            row,
            column: name,
            value: event
                .get(column)
                .and_then(Value::as_i64)
                .unwrap_or(i64::MIN),
        })
}

fn offset(
    row: i64,
    event: &[Value],
    column: usize,
    name: &'static str,
) -> Result<u32, ParseArtifactLoadError> {
    let value = number(row, event, column, name)?;
    u32::try_from(value).map_err(|_| ParseArtifactLoadError::InvalidNumber {
        row,
        column: name,
        value: i64::try_from(value).unwrap_or(i64::MAX),
    })
}

fn load_event(row: i64, value: &Value) -> Result<ParseEvent, ParseArtifactLoadError> {
    let event = value
        .as_array()
        .ok_or(ParseArtifactLoadError::MissingField("event"))?;
    let kind = event_string(event, 0, "event.kind")?;
    match kind {
        "start-node" => Ok(ParseEvent::StartNode {
            id: number(row, event, 1, "id")?,
            kind: event_string(event, 2, "node.kind")?.to_owned(),
            start: offset(row, event, 3, "start")?,
        }),
        "finish-node" => Ok(ParseEvent::FinishNode {
            id: number(row, event, 1, "id")?,
            kind: event_string(event, 2, "node.kind")?.to_owned(),
            end: offset(row, event, 3, "end")?,
        }),
        "start-field" => Ok(ParseEvent::StartField {
            field: event_string(event, 1, "field")?.to_owned(),
            start: offset(row, event, 2, "start")?,
        }),
        "finish-field" => Ok(ParseEvent::FinishField {
            field: event_string(event, 1, "field")?.to_owned(),
            end: offset(row, event, 2, "end")?,
        }),
        "token" => Ok(ParseEvent::Token {
            id: number(row, event, 1, "id")?,
            kind: event_string(event, 2, "token.kind")?.to_owned(),
            lexeme: event_string(event, 3, "token.lexeme")?.to_owned(),
            start: offset(row, event, 4, "start")?,
            end: offset(row, event, 5, "end")?,
        }),
        _ => Err(ParseArtifactLoadError::InvalidEvent {
            row,
            kind: kind.to_owned(),
        }),
    }
}
