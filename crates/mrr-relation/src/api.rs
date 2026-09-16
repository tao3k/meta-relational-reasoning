//! Language-neutral typed relation schemas and context-bearing facts.
#![forbid(unsafe_code)]

use crate::validation::{
    validate_constraints, validate_field_value, validate_fields, validate_property_fields,
};
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
pub enum ValueKind {
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
    pub const fn kind(&self) -> ValueKind {
        match self {
            Self::Entity(_) => ValueKind::Entity,
            Self::Null => ValueKind::Null,
            Self::Boolean(_) => ValueKind::Boolean,
            Self::Integer(_) => ValueKind::Integer,
            Self::Decimal(_) => ValueKind::Decimal,
            Self::Float(_) => ValueKind::Float,
            Self::String(_) => ValueKind::String,
            Self::ByteString(_) => ValueKind::ByteString,
            Self::Date(_) => ValueKind::Date,
            Self::Time(_) => ValueKind::Time,
            Self::Timestamp(_) => ValueKind::Timestamp,
            Self::Duration(_) => ValueKind::Duration,
            Self::List(_) => ValueKind::List,
            Self::Record(_) => ValueKind::Record,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum FloatWidth {
    Binary32,
    Binary64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum TemporalUnit {
    Second,
    Millisecond,
    Microsecond,
    Nanosecond,
}

impl TemporalUnit {
    pub(crate) const fn fractional_digits(self) -> usize {
        match self {
            Self::Second => 0,
            Self::Millisecond => 3,
            Self::Microsecond => 6,
            Self::Nanosecond => 9,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum TimezonePolicy {
    Naive,
    Utc,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ValueSchema {
    Entity,
    Boolean,
    Integer,
    Decimal {
        precision: u8,
        scale: u8,
    },
    Float {
        width: FloatWidth,
    },
    String,
    ByteString,
    Date,
    Time {
        unit: TemporalUnit,
        timezone: TimezonePolicy,
    },
    Timestamp {
        unit: TemporalUnit,
        timezone: TimezonePolicy,
    },
    Duration,
    List {
        element: Box<ValueSchema>,
        element_nullable: bool,
    },
    Record {
        fields: Vec<RelationField>,
    },
}

impl ValueSchema {
    /// Returns the runtime value kind admitted by this schema.
    #[must_use]
    pub const fn kind(&self) -> ValueKind {
        match self {
            Self::Entity => ValueKind::Entity,
            Self::Boolean => ValueKind::Boolean,
            Self::Integer => ValueKind::Integer,
            Self::Decimal { .. } => ValueKind::Decimal,
            Self::Float { .. } => ValueKind::Float,
            Self::String => ValueKind::String,
            Self::ByteString => ValueKind::ByteString,
            Self::Date => ValueKind::Date,
            Self::Time { .. } => ValueKind::Time,
            Self::Timestamp { .. } => ValueKind::Timestamp,
            Self::Duration => ValueKind::Duration,
            Self::List { .. } => ValueKind::List,
            Self::Record { .. } => ValueKind::Record,
        }
    }

    pub fn validate(&self) -> Result<(), RelationError> {
        match self {
            Self::Decimal { precision, scale }
                if *precision == 0 || *precision > 76 || *scale > *precision =>
            {
                return Err(RelationError::InvalidValueSchema(
                    "decimal requires 1 <= precision <= 76 and scale <= precision",
                ));
            }
            Self::List { element, .. } => element.validate()?,
            Self::Record { fields } => validate_fields(fields)?,
            _ => {}
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct RelationField {
    name: String,
    schema: ValueSchema,
    nullable: bool,
}

impl RelationField {
    pub fn new(
        name: impl Into<String>,
        schema: ValueSchema,
        nullable: bool,
    ) -> Result<Self, RelationError> {
        let name = name.into();
        if name.is_empty() || name.trim() != name {
            return Err(RelationError::EmptyFieldName);
        }
        schema.validate()?;
        Ok(Self {
            name,
            schema,
            nullable,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn schema(&self) -> &ValueSchema {
        &self.schema
    }

    #[must_use]
    pub const fn nullable(&self) -> bool {
        self.nullable
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum RelationConstraint {
    Key(Vec<String>),
    Unique(Vec<String>),
    FunctionalDependency {
        determinant: Vec<String>,
        dependent: Vec<String>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RelationSchema {
    id: RelationId,
    predicate: String,
    fields: Vec<RelationField>,
    constraints: Vec<RelationConstraint>,
}

/// A language-neutral entity type and its declared property shape.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EntitySchema {
    id: EntityId,
    name: String,
    properties: Vec<RelationField>,
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum RelationContextError {
    AuthorityProvenanceMismatch,
}

impl RelationContext {
    pub fn new(
        generation: GenerationId,
        authority: RelationAuthority,
        provenance: FactProvenance,
        completeness: EvidenceCompleteness,
        validity: FactValidity,
    ) -> Result<Self, RelationContextError> {
        let context = Self {
            generation,
            authority,
            provenance,
            completeness,
            validity,
        };
        context.validate()?;
        Ok(context)
    }

    pub fn validate(&self) -> Result<(), RelationContextError> {
        let coherent = matches!(
            (self.authority, self.provenance),
            (RelationAuthority::Entity(authority), FactProvenance::Source(source))
                if authority == source
        ) || matches!(
            (self.authority, self.provenance),
            (
                RelationAuthority::Rule(_) | RelationAuthority::RulePack(_),
                FactProvenance::Derivation(_)
            )
        );
        if coherent {
            Ok(())
        } else {
            Err(RelationContextError::AuthorityProvenanceMismatch)
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
    EmptyEntityName,
    EmptyFields,
    EmptyFieldName,
    DuplicateFieldName(String),
    InvalidValueSchema(&'static str),
    InvalidConstraint(String),
    ArityMismatch {
        expected: usize,
        actual: usize,
    },
    TypeMismatch {
        field: String,
        expected: ValueSchema,
        actual: ValueKind,
    },
    NullNotAllowed(String),
    InvalidValue {
        field: String,
        reason: &'static str,
    },
    InvalidContext(RelationContextError),
    SelfInvalidation(FactId),
    WrongRelation,
}

impl EntitySchema {
    pub fn new(
        id: EntityId,
        name: impl Into<String>,
        properties: Vec<RelationField>,
    ) -> Result<Self, RelationError> {
        let schema = Self {
            id,
            name: name.into(),
            properties,
        }
        .normalized();
        schema.validate()?;
        Ok(schema)
    }

    /// Revalidates an entity schema after external decoding.
    pub fn validate(&self) -> Result<(), RelationError> {
        if self.name.is_empty() || self.name.trim() != self.name {
            return Err(RelationError::EmptyEntityName);
        }
        validate_property_fields(&self.properties)
    }

    #[must_use]
    pub const fn id(&self) -> EntityId {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn properties(&self) -> &[RelationField] {
        &self.properties
    }

    #[must_use]
    pub fn property(&self, name: &str) -> Option<&RelationField> {
        self.properties
            .binary_search_by(|property| property.name().cmp(name))
            .ok()
            .map(|index| &self.properties[index])
    }

    #[must_use]
    pub fn normalized(mut self) -> Self {
        self.properties
            .sort_by(|left, right| left.name().cmp(right.name()));
        self
    }
}

impl RelationSchema {
    pub fn new(
        id: RelationId,
        predicate: impl Into<String>,
        fields: Vec<RelationField>,
        constraints: Vec<RelationConstraint>,
    ) -> Result<Self, RelationError> {
        let schema = Self {
            id,
            predicate: predicate.into(),
            fields,
            constraints,
        };
        schema.validate()?;
        Ok(schema)
    }

    /// Revalidates a schema after decoding it from an external representation.
    ///
    /// Constructors enforce these invariants for native callers, while bundle
    /// admission calls this method to ensure serde cannot bypass them.
    pub fn validate(&self) -> Result<(), RelationError> {
        if self.predicate.is_empty() || self.predicate.trim() != self.predicate {
            return Err(RelationError::EmptyPredicate);
        }
        validate_fields(&self.fields)?;
        validate_constraints(&self.fields, &self.constraints)
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
        fact.context
            .validate()
            .map_err(RelationError::InvalidContext)?;
        if fact.context.validity == FactValidity::InvalidatedBy(fact.id) {
            return Err(RelationError::SelfInvalidation(fact.id));
        }
        for (field, value) in self.fields.iter().zip(&fact.values) {
            validate_field_value(field, value, &field.name)?;
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
    pub fn constraints(&self) -> &[RelationConstraint] {
        &self.constraints
    }
}
