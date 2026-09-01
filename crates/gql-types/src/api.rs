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
    /// Text value.
    String,
    /// List value.
    List,
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
    /// String value.
    String(String),
    /// List value.
    List(Vec<Value>),
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
            Self::String(_) => ValueType::String,
            Self::List(_) => ValueType::List,
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
