#![forbid(unsafe_code)]

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ValueType {
    Null,
    Boolean,
    Integer,
    Decimal,
    String,
    Date,
    Node,
    Edge,
    Path,
    Any,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Value {
    Null,
    Boolean(bool),
    Integer(i64),
    Decimal(String),
    String(String),
}

impl Value {
    #[must_use]
    pub const fn value_type(&self) -> ValueType {
        match self {
            Self::Null => ValueType::Null,
            Self::Boolean(_) => ValueType::Boolean,
            Self::Integer(_) => ValueType::Integer,
            Self::Decimal(_) => ValueType::Decimal,
            Self::String(_) => ValueType::String,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TruthValue {
    True,
    False,
    Unknown,
}
