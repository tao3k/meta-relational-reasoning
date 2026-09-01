use mrr_identity::{GenerationId, RevisionId};

const REVISION_BINDING_SCHEMA: &[u8] = b"mrr.external-revision.v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalRevisionIdentity {
    provider: String,
    logical_change: String,
    content_revision: String,
}

impl ExternalRevisionIdentity {
    pub fn new(
        provider: impl Into<String>,
        logical_change: impl Into<String>,
        content_revision: impl Into<String>,
    ) -> Result<Self, RevisionBindingError> {
        let identity = Self {
            provider: provider.into(),
            logical_change: logical_change.into(),
            content_revision: content_revision.into(),
        };
        for (field, value) in [
            ("provider", identity.provider.as_str()),
            ("logical_change", identity.logical_change.as_str()),
            ("content_revision", identity.content_revision.as_str()),
        ] {
            if value.is_empty() || value.trim() != value {
                return Err(RevisionBindingError::InvalidExternalField(field));
            }
        }
        Ok(identity)
    }

    #[must_use]
    pub fn provider(&self) -> &str {
        &self.provider
    }

    #[must_use]
    pub fn logical_change(&self) -> &str {
        &self.logical_change
    }

    #[must_use]
    pub fn content_revision(&self) -> &str {
        &self.content_revision
    }

    fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        push_field(&mut bytes, REVISION_BINDING_SCHEMA);
        push_field(&mut bytes, self.provider.as_bytes());
        push_field(&mut bytes, self.logical_change.as_bytes());
        push_field(&mut bytes, self.content_revision.as_bytes());
        bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionBinding {
    revision: RevisionId,
    generation: GenerationId,
    external: ExternalRevisionIdentity,
}

impl RevisionBinding {
    pub fn admit(
        external: ExternalRevisionIdentity,
        generation: GenerationId,
    ) -> Result<Self, RevisionBindingError> {
        let revision = RevisionId::from_canonical_bytes(external.canonical_bytes())
            .map_err(|_| RevisionBindingError::IdentityDerivation)?;
        Ok(Self {
            revision,
            generation,
            external,
        })
    }

    #[must_use]
    pub const fn revision(&self) -> RevisionId {
        self.revision
    }

    #[must_use]
    pub const fn generation(&self) -> GenerationId {
        self.generation
    }

    #[must_use]
    pub const fn external(&self) -> &ExternalRevisionIdentity {
        &self.external
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RevisionBindingError {
    InvalidExternalField(&'static str),
    IdentityDerivation,
}

fn push_field(output: &mut Vec<u8>, field: &[u8]) {
    output.extend_from_slice(&(field.len() as u64).to_be_bytes());
    output.extend_from_slice(field);
}
