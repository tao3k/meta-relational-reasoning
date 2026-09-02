//! Public value model primitives for the GQL toolchain.

/// ISO-aligned runtime type for GQL scalar and structural values.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ValueType {
    /// Null value.
    Null,
    /// Boolean value.
    Boolean,
    /// Integer value.
    Integer,
    /// Decimal value represented as string form.
    Decimal,
    /// Approximate numeric value represented in canonical lexical form.
    Float,
    /// Text value.
    String,
    /// Byte-string value.
    ByteString,
    /// Calendar date value.
    Date,
    /// Wall-clock time value.
    Time,
    /// Combined date and time value.
    Timestamp,
    /// ISO duration value.
    Duration,
    /// List value.
    List,
    /// Ordered record value.
    Record,
    /// Node-typed value.
    Node,
    /// Edge-typed value.
    Edge,
    /// Path-typed value.
    Path,
    /// Catch-all unknown value type.
    Any,
}

/// Runtime value container used across the frontend and semantic passes.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Value {
    /// Null value.
    Null,
    /// Boolean value.
    Boolean(bool),
    /// Signed integer value.
    Integer(i64),
    /// Decimal value.
    Decimal(String),
    /// Approximate numeric value.
    Float(String),
    /// String value.
    String(String),
    /// Byte-string value.
    ByteString(Vec<u8>),
    /// Calendar date value in canonical lexical form.
    Date(String),
    /// Wall-clock time value in canonical lexical form.
    Time(String),
    /// Combined date and time value in canonical lexical form.
    Timestamp(String),
    /// ISO duration value in canonical lexical form.
    Duration(String),
    /// List value.
    List(Vec<Value>),
    /// Ordered record value.
    Record(Vec<(String, Value)>),
}

impl Value {
    /// Returns the static runtime [`ValueType`] of this value.
    #[must_use]
    pub const fn value_type(&self) -> ValueType {
        match self {
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

/// Three-valued boolean-like result used by semantic checks.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TruthValue {
    /// True.
    True,
    /// False.
    False,
    /// Unknown / undecidable.
    Unknown,
}
