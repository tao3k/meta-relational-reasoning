//! Language-neutral typed relation schemas and context-bearing facts.
#![forbid(unsafe_code)]

pub use mrr_identity::{
    DerivationId, EntityId, FactId, GenerationId, RelationId, RuleId, RulePackId,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Value {
    Entity(EntityId),
    Null,
    Boolean(bool),
    Integer(i64),
    Decimal(String),
    Float(String),
    String(String),
    ByteString(Vec<u8>),
    Date(String),
    Time(String),
    Timestamp(String),
    Duration(String),
    List(Vec<Value>),
    Record(Vec<(String, Value)>),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ValueType {
    Entity,
    Null,
    Boolean,
    Integer,
    Decimal,
    Float,
    String,
    ByteString,
    Date,
    Time,
    Timestamp,
    Duration,
    List,
    Record,
}

impl Value {
    #[must_use]
    pub const fn value_type(&self) -> ValueType {
        match self {
            Self::Entity(_) => ValueType::Entity,
            Self::Null => ValueType::Null,
            Self::Boolean(_) => ValueType::Boolean,
            Self::Integer(_) => ValueType::Integer,
            Self::Decimal(_) => ValueType::Decimal,
            Self::Float(_) => ValueType::Float,
            Self::String(_) => ValueType::String,
            Self::ByteString(_) => ValueType::ByteString,
            Self::Date(_) => ValueType::Date,
            Self::Time(_) => ValueType::Time,
            Self::Timestamp(_) => ValueType::Timestamp,
            Self::Duration(_) => ValueType::Duration,
            Self::List(_) => ValueType::List,
            Self::Record(_) => ValueType::Record,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct RelationField {
    name: String,
    value_type: ValueType,
}

impl RelationField {
    pub fn new(name: impl Into<String>, value_type: ValueType) -> Result<Self, RelationError> {
        let name = name.into();
        if name.is_empty() || name.trim() != name {
            return Err(RelationError::EmptyFieldName);
        }
        Ok(Self { name, value_type })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn value_type(&self) -> ValueType {
        self.value_type
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum RelationCardinality {
    OneToOne,
    OneToMany,
    ManyToOne,
    ManyToMany,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RelationSchema {
    id: RelationId,
    predicate: String,
    fields: Vec<RelationField>,
    cardinality: RelationCardinality,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum RelationAuthority {
    Entity(EntityId),
    Rule(RuleId),
    RulePack(RulePackId),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum FactProvenance {
    Source(EntityId),
    Derivation(DerivationId),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum EvidenceCompleteness {
    Complete,
    Partial,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum FactValidity {
    Valid,
    InvalidatedBy(FactId),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct RelationContext {
    generation: GenerationId,
    authority: RelationAuthority,
    provenance: FactProvenance,
    completeness: EvidenceCompleteness,
    validity: FactValidity,
}

impl RelationContext {
    #[must_use]
    pub const fn new(
        generation: GenerationId,
        authority: RelationAuthority,
        provenance: FactProvenance,
        completeness: EvidenceCompleteness,
        validity: FactValidity,
    ) -> Self {
        Self {
            generation,
            authority,
            provenance,
            completeness,
            validity,
        }
    }

    #[must_use]
    pub const fn generation(&self) -> GenerationId {
        self.generation
    }

    #[must_use]
    pub const fn authority(&self) -> RelationAuthority {
        self.authority
    }

    #[must_use]
    pub const fn provenance(&self) -> FactProvenance {
        self.provenance
    }

    #[must_use]
    pub const fn completeness(&self) -> EvidenceCompleteness {
        self.completeness
    }

    #[must_use]
    pub const fn validity(&self) -> FactValidity {
        self.validity
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Fact {
    id: FactId,
    relation: RelationId,
    values: Vec<Value>,
    context: RelationContext,
}

impl Fact {
    #[must_use]
    pub fn new(
        id: FactId,
        relation: RelationId,
        values: Vec<Value>,
        context: RelationContext,
    ) -> Self {
        Self {
            id,
            relation,
            values,
            context,
        }
    }

    #[must_use]
    pub const fn id(&self) -> FactId {
        self.id
    }

    #[must_use]
    pub const fn relation(&self) -> RelationId {
        self.relation
    }

    #[must_use]
    pub fn values(&self) -> &[Value] {
        &self.values
    }

    #[must_use]
    pub const fn context(&self) -> &RelationContext {
        &self.context
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RelationError {
    EmptyPredicate,
    EmptyFields,
    EmptyFieldName,
    DuplicateFieldName(String),
    ArityMismatch {
        expected: usize,
        actual: usize,
    },
    TypeMismatch {
        field: String,
        expected: ValueType,
        actual: ValueType,
    },
    WrongRelation,
}

impl RelationSchema {
    pub fn new(
        id: RelationId,
        predicate: impl Into<String>,
        fields: Vec<RelationField>,
        cardinality: RelationCardinality,
    ) -> Result<Self, RelationError> {
        let predicate = predicate.into();
        if predicate.is_empty() || predicate.trim() != predicate {
            return Err(RelationError::EmptyPredicate);
        }
        if fields.is_empty() {
            return Err(RelationError::EmptyFields);
        }
        let mut names = std::collections::BTreeSet::new();
        for field in &fields {
            if !names.insert(field.name.clone()) {
                return Err(RelationError::DuplicateFieldName(field.name.clone()));
            }
        }
        Ok(Self {
            id,
            predicate,
            fields,
            cardinality,
        })
    }

    pub fn validate_fact(&self, fact: &Fact) -> Result<(), RelationError> {
        if fact.relation != self.id {
            return Err(RelationError::WrongRelation);
        }
        if fact.values.len() != self.fields.len() {
            return Err(RelationError::ArityMismatch {
                expected: self.fields.len(),
                actual: fact.values.len(),
            });
        }
        for (field, value) in self.fields.iter().zip(&fact.values) {
            let actual = value.value_type();
            if actual != field.value_type {
                return Err(RelationError::TypeMismatch {
                    field: field.name.clone(),
                    expected: field.value_type,
                    actual,
                });
            }
        }
        Ok(())
    }

    #[must_use]
    pub const fn id(&self) -> RelationId {
        self.id
    }

    #[must_use]
    pub fn predicate(&self) -> &str {
        &self.predicate
    }

    #[must_use]
    pub fn fields(&self) -> &[RelationField] {
        &self.fields
    }

    #[must_use]
    pub const fn cardinality(&self) -> RelationCardinality {
        self.cardinality
    }
}
