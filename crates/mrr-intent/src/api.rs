use mrr_identity::ReasoningBundleId;
use sha2::{Digest, Sha256};

const INTENT_SCHEMA: &[u8] = b"mrr.intent.v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntentSemanticModel {
    goals: Vec<String>,
    constraints: Vec<String>,
    questions: Vec<String>,
    invariants: Vec<String>,
    digest: [u8; 32],
}

impl IntentSemanticModel {
    pub fn project_org(source: &str) -> Result<Self, IntentProjectionError> {
        if !source.is_ascii() {
            return Err(IntentProjectionError::NonAsciiSource);
        }
        let mut root_seen = false;
        let mut section = None;
        let mut goals = Vec::new();
        let mut constraints = Vec::new();
        let mut questions = Vec::new();
        let mut invariants = Vec::new();
        let mut sections_seen = Vec::new();
        for line in source.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#+") {
                continue;
            }
            if let Some(heading) = line.strip_prefix("* ") {
                if heading != "Intent" || root_seen {
                    return Err(IntentProjectionError::InvalidRoot);
                }
                root_seen = true;
                section = None;
                continue;
            }
            if let Some(heading) = line.strip_prefix("** ") {
                if !root_seen || sections_seen.contains(&heading) {
                    return Err(IntentProjectionError::InvalidSection(heading.to_owned()));
                }
                section = Some(match heading {
                    "Goal" => IntentSection::Goal,
                    "Constraint" => IntentSection::Constraint,
                    "Question" => IntentSection::Question,
                    "Invariant" => IntentSection::Invariant,
                    _ => return Err(IntentProjectionError::InvalidSection(heading.to_owned())),
                });
                sections_seen.push(heading);
                continue;
            }
            if line.starts_with('*') {
                return Err(IntentProjectionError::UnsupportedHeading(line.to_owned()));
            }
            let value = line.strip_prefix("- ").unwrap_or(line).trim();
            if value.is_empty() {
                continue;
            }
            match section.ok_or(IntentProjectionError::ContentOutsideSection)? {
                IntentSection::Goal => goals.push(value.to_owned()),
                IntentSection::Constraint => constraints.push(value.to_owned()),
                IntentSection::Question => questions.push(value.to_owned()),
                IntentSection::Invariant => invariants.push(value.to_owned()),
            }
        }
        if !root_seen {
            return Err(IntentProjectionError::InvalidRoot);
        }
        if [
            goals.as_slice(),
            constraints.as_slice(),
            questions.as_slice(),
            invariants.as_slice(),
        ]
        .iter()
        .any(|values| values.is_empty())
        {
            return Err(IntentProjectionError::MissingRequiredContent);
        }
        let digest = intent_digest(&goals, &constraints, &questions, &invariants);
        Ok(Self {
            goals,
            constraints,
            questions,
            invariants,
            digest,
        })
    }

    #[must_use]
    pub fn goals(&self) -> &[String] {
        &self.goals
    }

    #[must_use]
    pub fn constraints(&self) -> &[String] {
        &self.constraints
    }

    #[must_use]
    pub fn questions(&self) -> &[String] {
        &self.questions
    }

    #[must_use]
    pub fn invariants(&self) -> &[String] {
        &self.invariants
    }

    #[must_use]
    pub const fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
}

#[derive(Clone, Copy)]
enum IntentSection {
    Goal,
    Constraint,
    Question,
    Invariant,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntentBundleBinding {
    intent_digest: [u8; 32],
    bundle: ReasoningBundleId,
}

impl IntentBundleBinding {
    #[must_use]
    pub const fn select(model: &IntentSemanticModel, bundle: ReasoningBundleId) -> Self {
        Self {
            intent_digest: model.digest,
            bundle,
        }
    }

    #[must_use]
    pub const fn bundle(&self) -> ReasoningBundleId {
        self.bundle
    }

    #[must_use]
    pub fn status(&self, model: &IntentSemanticModel) -> IntentBindingStatus {
        if self.intent_digest == model.digest {
            IntentBindingStatus::Current
        } else {
            IntentBindingStatus::Stale
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntentBindingStatus {
    Current,
    Stale,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntentProjectionError {
    NonAsciiSource,
    InvalidRoot,
    InvalidSection(String),
    UnsupportedHeading(String),
    ContentOutsideSection,
    MissingRequiredContent,
}

fn intent_digest(
    goals: &[String],
    constraints: &[String],
    questions: &[String],
    invariants: &[String],
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, INTENT_SCHEMA);
    for (label, values) in [
        (b"goal".as_slice(), goals),
        (b"constraint".as_slice(), constraints),
        (b"question".as_slice(), questions),
        (b"invariant".as_slice(), invariants),
    ] {
        hash_field(&mut hasher, label);
        hash_len(&mut hasher, values.len());
        for value in values {
            hash_field(&mut hasher, value.as_bytes());
        }
    }
    hasher.finalize().into()
}

fn hash_len(hasher: &mut Sha256, len: usize) {
    hasher.update((len as u64).to_be_bytes());
}

fn hash_field(hasher: &mut Sha256, bytes: &[u8]) {
    hash_len(hasher, bytes.len());
    hasher.update(bytes);
}
