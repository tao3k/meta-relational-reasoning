//! Deterministic generation and fail-closed admission for tracked Rust grammar projections.

use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

const PROVENANCE_SCHEMA: &str = "mrr.gerbil-grammar-projection.v1";
const BRIDGE_REVISION: &str = "a83fb649ddbbeaabdb538a6eaf0ded10838f7fad";
const INPUT_PATHS: &[&str] = &[
    "build.ss",
    "gerbil.pkg",
    "scheme/grammar/gql-declaration.ss",
    "scheme/grammar/gql-profile.ss",
    "scheme/grammar/core.ss",
    "scheme/grammar/gql.ss",
    "scheme/grammar/cypher.ss",
    "scheme/reasoning/core.ss",
    "scheme/reasoning/declaration.ss",
    "scheme/reasoning/default.ss",
    "scheme/grammar/native.ss",
];

/// Computes a framed SHA-256 over every canonical Scheme generator input.
///
/// # Errors
///
/// Returns an I/O error when a canonical input is absent or unreadable.
pub fn workspace_input_fingerprint(workspace: &Path) -> Result<String, GrammarProjectionError> {
    let mut hasher = Sha256::new();
    for relative_path in INPUT_PATHS {
        let contents = fs::read(workspace.join(relative_path)).map_err(|source| {
            GrammarProjectionError::Io {
                operation: "read grammar input",
                source,
            }
        })?;
        hash_framed_value(&mut hasher, relative_path.as_bytes());
        hash_framed_value(&mut hasher, &contents);
    }
    Ok(hex_digest(hasher.finalize()))
}

/// Stamps an exact Gerbil-generated body with input and body provenance.
#[must_use]
pub fn stamp_projection(body: &str, input_fingerprint: &str) -> String {
    let body_fingerprint = sha256(body.as_bytes());
    format!(
        "// {PROVENANCE_SCHEMA} input-sha256={input_fingerprint} \
         body-sha256={body_fingerprint} gerbil-scheme-rust-rev={BRIDGE_REVISION}\n{body}"
    )
}

/// Validates the tracked projection against current Scheme inputs and content.
///
/// # Errors
///
/// Returns a fail-closed drift error for a malformed header, stale input
/// fingerprint, modified body, or unexpected bridge revision.
pub fn validate_projection(
    tracked: &str,
    expected_input_fingerprint: &str,
) -> Result<(), GrammarProjectionError> {
    let (header, body) = tracked
        .split_once('\n')
        .ok_or(GrammarProjectionError::Drift(
            "missing provenance header separator",
        ))?;
    let mut fields = header.split_ascii_whitespace();
    if fields.next() != Some("//") || fields.next() != Some(PROVENANCE_SCHEMA) {
        return Err(GrammarProjectionError::Drift(
            "missing grammar projection provenance schema",
        ));
    }
    let input = fingerprint_field(fields.next(), "input-sha256=")?;
    let body_fingerprint = fingerprint_field(fields.next(), "body-sha256=")?;
    let bridge_revision = fields
        .next()
        .and_then(|field| field.strip_prefix("gerbil-scheme-rust-rev="))
        .ok_or(GrammarProjectionError::Drift(
            "missing Gerbil bridge revision",
        ))?;
    if fields.next().is_some() {
        return Err(GrammarProjectionError::Drift(
            "unexpected grammar projection provenance fields",
        ));
    }
    if input != expected_input_fingerprint {
        return Err(GrammarProjectionError::Drift(
            "input fingerprint does not match current Scheme grammar",
        ));
    }
    if body_fingerprint != sha256(body.as_bytes()) {
        return Err(GrammarProjectionError::Drift(
            "body fingerprint does not match tracked Rust projection",
        ));
    }
    if bridge_revision != BRIDGE_REVISION {
        return Err(GrammarProjectionError::Drift(
            "Gerbil bridge revision does not match the admitted PR16 dependency",
        ));
    }
    Ok(())
}

fn fingerprint_field<'a>(
    field: Option<&'a str>,
    prefix: &str,
) -> Result<&'a str, GrammarProjectionError> {
    let fingerprint =
        field
            .and_then(|field| field.strip_prefix(prefix))
            .ok_or(GrammarProjectionError::Drift(
                "missing grammar projection fingerprint field",
            ))?;
    if fingerprint.len() != 64
        || !fingerprint
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(GrammarProjectionError::Drift(
            "grammar projection fingerprint is not lowercase SHA-256",
        ));
    }
    Ok(fingerprint)
}

fn sha256(value: &[u8]) -> String {
    hex_digest(Sha256::digest(value))
}

fn hash_framed_value(hasher: &mut Sha256, value: &[u8]) {
    hasher.update(value.len().to_le_bytes());
    hasher.update(value);
}

fn hex_digest(digest: impl AsRef<[u8]>) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in digest.as_ref() {
        use std::fmt::Write;
        write!(encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

/// Typed failure at the build-time Gerbil projection boundary.
#[derive(Debug)]
pub enum GrammarProjectionError {
    /// Workspace I/O failure.
    Io {
        /// Operation that failed.
        operation: &'static str,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// Generated projection failed admission.
    Drift(&'static str),
}

impl fmt::Display for GrammarProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { operation, source } => write!(formatter, "{operation}: {source}"),
            Self::Drift(reason) => write!(formatter, "grammar projection rejected: {reason}"),
        }
    }
}

impl Error for GrammarProjectionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Drift(_) => None,
        }
    }
}
