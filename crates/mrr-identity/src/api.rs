//! Versioned, domain-separated, content-derived identities shared by every MRR layer.
#![forbid(unsafe_code)]

use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};
use sha2::{Digest, Sha256};

/// Schema framing used before hashing every canonical semantic identity input.
pub const IDENTITY_SCHEMA: &str = "mrr.identity.v1";

const DIGEST_BYTES: usize = 32;
const ENCODED_DIGEST_BYTES: usize = DIGEST_BYTES * 2;

/// The semantic namespace that owns an identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum IdentityDomain {
    Entity,
    Relation,
    Fact,
    Rule,
    Derivation,
    Query,
    QueryOperator,
    State,
    Transition,
    Action,
    Generation,
    Revision,
    RulePack,
    LineageNode,
    LineageEdge,
    ReasoningBundle,
}

impl IdentityDomain {
    /// Returns the canonical lowercase domain label encoded into hashes and text forms.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Entity => "entity",
            Self::Relation => "relation",
            Self::Fact => "fact",
            Self::Rule => "rule",
            Self::Derivation => "derivation",
            Self::Query => "query",
            Self::QueryOperator => "query-operator",
            Self::State => "state",
            Self::Transition => "transition",
            Self::Action => "action",
            Self::Generation => "generation",
            Self::Revision => "revision",
            Self::RulePack => "rule-pack",
            Self::LineageNode => "lineage-node",
            Self::LineageEdge => "lineage-edge",
            Self::ReasoningBundle => "reasoning-bundle",
        }
    }
}

/// Fail-closed identity derivation and decoding errors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdentityError {
    EmptyCanonicalInput,
    MalformedEncoding,
    SchemaMismatch,
    DomainMismatch {
        expected: IdentityDomain,
        actual: String,
    },
    VersionMismatch(String),
    InvalidDigest,
}

impl fmt::Display for IdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for IdentityError {}

fn derive_digest(
    domain: IdentityDomain,
    canonical_input: &[u8],
) -> Result<[u8; DIGEST_BYTES], IdentityError> {
    if canonical_input.is_empty() {
        return Err(IdentityError::EmptyCanonicalInput);
    }
    let mut hasher = Sha256::new();
    hasher.update(IDENTITY_SCHEMA.as_bytes());
    hasher.update([0]);
    hasher.update(domain.as_str().as_bytes());
    hasher.update([0]);
    hasher.update(canonical_input);
    Ok(hasher.finalize().into())
}

fn encode_digest(digest: &[u8; DIGEST_BYTES], formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    for byte in digest {
        write!(formatter, "{byte:02x}")?;
    }
    Ok(())
}

fn decode_digest(encoded: &str) -> Result<[u8; DIGEST_BYTES], IdentityError> {
    if encoded.len() != ENCODED_DIGEST_BYTES {
        return Err(IdentityError::InvalidDigest);
    }
    let mut digest = [0_u8; DIGEST_BYTES];
    for (index, chunk) in encoded.as_bytes().chunks_exact(2).enumerate() {
        digest[index] = (decode_nibble(chunk[0])? << 4) | decode_nibble(chunk[1])?;
    }
    Ok(digest)
}

fn decode_nibble(byte: u8) -> Result<u8, IdentityError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(IdentityError::InvalidDigest),
    }
}

fn parse_typed_identity(
    encoded: &str,
    expected: IdentityDomain,
) -> Result<[u8; DIGEST_BYTES], IdentityError> {
    let mut fields = encoded.split(':');
    if fields.next() != Some("mrr") {
        return Err(IdentityError::SchemaMismatch);
    }
    let actual_domain = fields.next().ok_or(IdentityError::MalformedEncoding)?;
    if actual_domain != expected.as_str() {
        return Err(IdentityError::DomainMismatch {
            expected,
            actual: actual_domain.to_owned(),
        });
    }
    let version = fields.next().ok_or(IdentityError::MalformedEncoding)?;
    if version != "v1" {
        return Err(IdentityError::VersionMismatch(version.to_owned()));
    }
    let digest = fields.next().ok_or(IdentityError::MalformedEncoding)?;
    if fields.next().is_some() {
        return Err(IdentityError::MalformedEncoding);
    }
    decode_digest(digest)
}

macro_rules! identity_type {
    ($name:ident, $domain:ident) => {
        #[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name([u8; DIGEST_BYTES]);

        impl $name {
            /// The semantic namespace permanently bound to this Rust type.
            pub const DOMAIN: IdentityDomain = IdentityDomain::$domain;

            /// Deterministically derives an identity from the owner's canonical semantic bytes.
            pub fn from_canonical_bytes(
                canonical_input: impl AsRef<[u8]>,
            ) -> Result<Self, IdentityError> {
                derive_digest(Self::DOMAIN, canonical_input.as_ref()).map(Self)
            }

            /// Returns the immutable SHA-256 digest bytes without erasing the Rust type.
            #[must_use]
            pub const fn digest_bytes(&self) -> &[u8; DIGEST_BYTES] {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "mrr:{}:v1:", Self::DOMAIN.as_str())?;
                encode_digest(&self.0, formatter)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_tuple(stringify!($name))
                    .field(&self.to_string())
                    .finish()
            }
        }

        impl FromStr for $name {
            type Err = IdentityError;

            fn from_str(encoded: &str) -> Result<Self, Self::Err> {
                parse_typed_identity(encoded, Self::DOMAIN).map(Self)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.to_string())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let encoded = String::deserialize(deserializer)?;
                encoded.parse().map_err(D::Error::custom)
            }
        }
    };
}

identity_type!(EntityId, Entity);
identity_type!(RelationId, Relation);
identity_type!(FactId, Fact);
identity_type!(RuleId, Rule);
identity_type!(DerivationId, Derivation);
identity_type!(QueryId, Query);
identity_type!(QueryOperatorId, QueryOperator);
identity_type!(StateId, State);
identity_type!(TransitionId, Transition);
identity_type!(ActionId, Action);
identity_type!(GenerationId, Generation);
identity_type!(RevisionId, Revision);
identity_type!(RulePackId, RulePack);
identity_type!(LineageNodeId, LineageNode);
identity_type!(LineageEdgeId, LineageEdge);
identity_type!(ReasoningBundleId, ReasoningBundle);
