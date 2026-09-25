// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: AGPL-3.0-only

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

/// Parser language selected explicitly at the native ABI boundary.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ParserLanguage {
    /// ISO/IEC 39075:2024 GQL language pack.
    Gql,
    /// Pinned openCypher 2024.1 language pack.
    Cypher,
}

impl ParserLanguage {
    /// Returns the stable native ABI selector.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Gql => "gql",
            Self::Cypher => "cypher",
        }
    }
}

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
    language: ParserLanguage,
    grammar_digest: String,
    kinds: Vec<ParserKindSpec>,
    by_name: BTreeMap<String, u16>,
    terminals: BTreeMap<String, u16>,
    terminal_names: Vec<String>,
    field_names: Vec<String>,
}

impl ParserKindCatalog {
    /// Returns the language that owns this kind catalog.
    #[must_use]
    pub const fn language(&self) -> ParserLanguage {
        self.language
    }

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

    fn terminal_name(&self, id: u32) -> Option<&str> {
        self.terminal_names.get(id as usize).map(String::as_str)
    }

    fn field_name(&self, id: u32) -> Option<&str> {
        self.field_names.get(id as usize).map(String::as_str)
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
    pub language: ParserLanguage,
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
    RuntimeStatus(super::NativeRuntimeStatus),
    ParserFailed {
        call_status: i32,
        result_status: i32,
        diagnostic: Option<String>,
    },
    InvalidPayload,
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
}

static GQL_KIND_CATALOG: OnceLock<Result<Arc<ParserKindCatalog>, ParseArtifactLoadError>> =
    OnceLock::new();
static CYPHER_KIND_CATALOG: OnceLock<Result<Arc<ParserKindCatalog>, ParseArtifactLoadError>> =
    OnceLock::new();

/// Parses source with the linked ISO GQL language pack.
pub fn parse_gql_artifact(source: &str) -> Result<ParseArtifact, ParseArtifactLoadError> {
    parse_artifact(ParserLanguage::Gql, source)
}

/// Parses source with the linked openCypher language pack.
pub fn parse_cypher_artifact(source: &str) -> Result<ParseArtifact, ParseArtifactLoadError> {
    parse_artifact(ParserLanguage::Cypher, source)
}

fn parse_artifact(
    language: ParserLanguage,
    source: &str,
) -> Result<ParseArtifact, ParseArtifactLoadError> {
    let (payload, kind_catalog) = request_parse_artifact(language, source)?;
    let artifact = decode_parse_artifact(&payload, source, kind_catalog)?;
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
    payload: &[u8],
    source: &str,
    kind_catalog: Arc<ParserKindCatalog>,
) -> Result<ParseArtifact, ParseArtifactLoadError> {
    const HEADER_SIZE: usize = 80;
    const EVENT_SIZE: usize = 24;
    if payload.len() < HEADER_SIZE || &payload[..4] != b"GPA1" || read_u32(payload, 4)? != 1 {
        return Err(ParseArtifactLoadError::InvalidPayload);
    }
    let status = match read_u32(payload, 8)? {
        0 => ParseArtifactStatus::Accepted,
        1 => ParseArtifactStatus::Rejected,
        value => return Err(ParseArtifactLoadError::InvalidStatus(value.to_string())),
    };
    let event_count = usize::try_from(read_u32(payload, 12)?)
        .map_err(|_| ParseArtifactLoadError::InvalidPayload)?;
    let expected_length = HEADER_SIZE
        .checked_add(
            event_count
                .checked_mul(EVENT_SIZE)
                .ok_or(ParseArtifactLoadError::InvalidPayload)?,
        )
        .ok_or(ParseArtifactLoadError::InvalidPayload)?;
    if payload.len() != expected_length {
        return Err(ParseArtifactLoadError::InvalidPayload);
    }
    let grammar_digest = digest_text(&payload[16..48]);
    if grammar_digest != kind_catalog.grammar_digest {
        return Err(ParseArtifactLoadError::InvalidGrammarDigest(grammar_digest));
    }
    let source_digest = digest_text(&payload[48..80]);
    let mut events = Vec::with_capacity(event_count);
    for row in 0..event_count {
        let offset = HEADER_SIZE + row * EVENT_SIZE;
        events.push(load_binary_event(
            row,
            &payload[offset..offset + EVENT_SIZE],
            source,
            &kind_catalog,
        )?);
    }
    Ok(ParseArtifact {
        schema: PARSE_ARTIFACT_SCHEMA_V1.to_owned(),
        language: kind_catalog.language,
        kind_catalog,
        status,
        grammar_digest,
        source_digest,
        events,
    })
}

fn request_parse_artifact(
    language: ParserLanguage,
    source: &str,
) -> Result<(Vec<u8>, Arc<ParserKindCatalog>), ParseArtifactLoadError> {
    let kind_catalog = match language {
        ParserLanguage::Gql => &GQL_KIND_CATALOG,
        ParserLanguage::Cypher => &CYPHER_KIND_CATALOG,
    }
    .get_or_init(|| load_native_kind_catalog(language))
    .clone()?;
    let language = CString::new(language.as_str()).expect("static parser language has no NUL");
    let source = CString::new(source).map_err(|_| ParseArtifactLoadError::InteriorNul)?;
    let native = with_native_runtime(move || ffi::parser_native_parse(&language, &source))
        .map_err(parse_runtime_error)?;
    let payload = native_payload(native, |call_status, result_status, diagnostic| {
        ParseArtifactLoadError::ParserFailed {
            call_status,
            result_status,
            diagnostic,
        }
    })?;
    Ok((payload, kind_catalog))
}

fn read_u32(payload: &[u8], offset: usize) -> Result<u32, ParseArtifactLoadError> {
    let bytes = payload
        .get(offset..offset + 4)
        .ok_or(ParseArtifactLoadError::InvalidPayload)?;
    Ok(u32::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| ParseArtifactLoadError::InvalidPayload)?,
    ))
}

fn read_u64(payload: &[u8], offset: usize) -> Result<u64, ParseArtifactLoadError> {
    let bytes = payload
        .get(offset..offset + 8)
        .ok_or(ParseArtifactLoadError::InvalidPayload)?;
    Ok(u64::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| ParseArtifactLoadError::InvalidPayload)?,
    ))
}

fn digest_text(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut digest = String::with_capacity(71);
    digest.push_str("sha256:");
    for byte in bytes {
        digest.push(char::from(HEX[usize::from(byte >> 4)]));
        digest.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    digest
}

fn load_binary_event(
    row: usize,
    event: &[u8],
    source: &str,
    catalog: &ParserKindCatalog,
) -> Result<ParseEvent, ParseArtifactLoadError> {
    let tag = read_u32(event, 0)?;
    let symbol = read_u32(event, 4)?;
    let id = read_u64(event, 8)?;
    let start = read_u32(event, 16)?;
    let end = read_u32(event, 20)?;
    let invalid = || ParseArtifactLoadError::InvalidEvent {
        row: i64::try_from(row).unwrap_or(i64::MAX),
        kind: format!("binary-tag-{tag}"),
    };
    match tag {
        1 => Ok(ParseEvent::StartNode {
            id,
            kind: catalog
                .kind_name(u16::try_from(symbol).map_err(|_| invalid())?)
                .ok_or_else(invalid)?
                .to_owned(),
            start,
        }),
        2 => Ok(ParseEvent::FinishNode {
            id,
            kind: catalog
                .kind_name(u16::try_from(symbol).map_err(|_| invalid())?)
                .ok_or_else(invalid)?
                .to_owned(),
            end,
        }),
        3 => Ok(ParseEvent::StartField {
            field: catalog.field_name(symbol).ok_or_else(invalid)?.to_owned(),
            start,
        }),
        4 => Ok(ParseEvent::FinishField {
            field: catalog.field_name(symbol).ok_or_else(invalid)?.to_owned(),
            end,
        }),
        5 => {
            let lexeme = source
                .get(start as usize..end as usize)
                .ok_or_else(invalid)?
                .to_owned();
            Ok(ParseEvent::Token {
                id,
                kind: catalog
                    .terminal_name(symbol)
                    .ok_or_else(invalid)?
                    .to_owned(),
                lexeme,
                start,
                end,
            })
        }
        _ => Err(invalid()),
    }
}

fn load_native_kind_catalog(
    language: ParserLanguage,
) -> Result<Arc<ParserKindCatalog>, ParseArtifactLoadError> {
    let language_name = CString::new(language.as_str()).expect("static parser language has no NUL");
    let (abi, native) = with_native_runtime(move || {
        (
            ffi::parser_native_abi_version(),
            ffi::parser_native_descriptor(&language_name),
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
    load_kind_catalog(&descriptor, language).map(Arc::new)
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
    language: ParserLanguage,
) -> Result<ParserKindCatalog, ParseArtifactLoadError> {
    if string_field(payload, "schema")? != PARSER_NATIVE_DESCRIPTOR_SCHEMA_V1 {
        return Err(ParseArtifactLoadError::InvalidHostDescriptor);
    }
    if string_field(payload, "language")? != language.as_str() {
        return Err(ParseArtifactLoadError::InvalidHostDescriptor);
    }
    let grammar_digest = string_field(payload, "grammarDigest")?.to_owned();
    if !valid_sha256_digest(&grammar_digest) {
        return Err(ParseArtifactLoadError::InvalidHostDescriptor);
    }
    let field_rows = payload
        .get("fields")
        .and_then(Value::as_array)
        .ok_or(ParseArtifactLoadError::InvalidHostDescriptor)?;
    let mut field_names = Vec::with_capacity(field_rows.len());
    let mut field_ids = BTreeMap::new();
    for field in field_rows {
        let field = field
            .as_str()
            .ok_or(ParseArtifactLoadError::InvalidHostDescriptor)?
            .to_owned();
        if field_ids.insert(field.clone(), field_names.len()).is_some() {
            return Err(ParseArtifactLoadError::InvalidHostDescriptor);
        }
        field_names.push(field);
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
        if fields.iter().any(|field| !field_ids.contains_key(field)) {
            return Err(ParseArtifactLoadError::InvalidHostDescriptor);
        }
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
    let mut terminal_names = Vec::with_capacity(terminal_rows.len());
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
            || terminals.insert(terminal.clone(), id).is_some()
        {
            return Err(ParseArtifactLoadError::InvalidHostDescriptor);
        }
        terminal_names.push(terminal);
    }
    Ok(ParserKindCatalog {
        language,
        grammar_digest,
        kinds,
        by_name,
        terminals,
        terminal_names,
        field_names,
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
