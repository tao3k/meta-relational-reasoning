//! Private inert Scheme projection for the typed result transport. No evaluator.
//! Serde writes directly to a bounded sink; decoding uses a bounded data tree.
use std::collections::BTreeSet;

use ciborium::Value;
use serde::{Serialize, de::DeserializeOwned, ser};

use super::QueryResultTransportError as Error;

const MAX_DEPTH: usize = 64;
// Every serialized/parsed datum consumes at least one input/output byte. The
// caller's byte ceiling therefore also bounds nodes without a second, smaller
// fixed ceiling that rejects otherwise admitted large result sets.

impl ser::Error for Error {
    fn custom<T: std::fmt::Display>(message: T) -> Self {
        Self::Encoding(message.to_string())
    }
}

fn invalid() -> Error {
    Error::Encoding("invalid or unbounded Scheme result datum".into())
}

pub(super) fn encode<T: Serialize>(value: &T, limit: usize) -> Result<Vec<u8>, Error> {
    let mut writer = Writer {
        bytes: Vec::new(),
        size: 0,
        retain: true,
        limit,
        depth: 0,
        nodes: 0,
    };
    value.serialize(&mut writer)?;
    Ok(writer.bytes)
}

pub(super) fn check<T: Serialize>(value: &T, limit: usize) -> Result<(), Error> {
    let mut writer = Writer {
        bytes: Vec::new(),
        size: 0,
        retain: false,
        limit,
        depth: 0,
        nodes: 0,
    };
    value.serialize(&mut writer)
}

pub(super) fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, Error> {
    let input = std::str::from_utf8(bytes).map_err(|_| invalid())?;
    let mut reader = Reader {
        input,
        offset: 0,
        nodes: 0,
    };
    let datum = reader.datum(0)?;
    reader.space();
    if reader.offset != input.len() {
        return Err(invalid());
    }
    datum
        .deserialized()
        .map_err(|error| Error::Encoding(error.to_string()))
}

struct Writer {
    bytes: Vec<u8>,
    size: usize,
    retain: bool,
    limit: usize,
    depth: usize,
    nodes: usize,
}
impl Writer {
    fn put(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let observed = self.size.saturating_add(bytes.len());
        if observed > self.limit {
            return Err(Error::TooLarge {
                limit: self.limit,
                observed,
            });
        }
        self.size = observed;
        if self.retain {
            self.bytes.extend_from_slice(bytes);
        }
        Ok(())
    }
    fn node(&mut self) -> Result<(), Error> {
        self.nodes += 1;
        if self.depth > MAX_DEPTH || self.nodes > self.limit {
            return Err(invalid());
        }
        Ok(())
    }
    fn atom(&mut self, value: &str) -> Result<(), Error> {
        self.node()?;
        self.put(value.as_bytes())
    }
    fn string(&mut self, value: &str) -> Result<(), Error> {
        self.node()?;
        self.put(b"\"")?;
        for c in value.chars() {
            match c {
                '"' => self.put(b"\\\"")?,
                '\\' => self.put(b"\\\\")?,
                '\n' => self.put(b"\\n")?,
                '\r' => self.put(b"\\r")?,
                '\t' => self.put(b"\\t")?,
                c if c < ' ' => return Err(invalid()),
                c => {
                    self.put(c.encode_utf8(&mut [0; 4]).as_bytes())?;
                }
            }
        }
        self.put(b"\"")
    }
    fn start(&mut self, kind: &[u8]) -> Result<(), Error> {
        self.node()?;
        self.put(kind)?;
        self.depth += 1;
        Ok(())
    }
    fn end(&mut self) -> Result<(), Error> {
        self.depth -= 1;
        self.put(b")")
    }
    fn variant(&mut self, name: &str) -> Result<(), Error> {
        self.start(b"(object")?;
        self.put(b" (")?;
        self.string(name)?;
        self.put(b" ")
    }
}

struct Compound<'a> {
    writer: &'a mut Writer,
    variant: bool,
}
impl Compound<'_> {
    fn element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
        self.writer.put(b" ")?;
        value.serialize(&mut *self.writer)
    }
    fn field<T: ?Sized + Serialize>(&mut self, key: &str, value: &T) -> Result<(), Error> {
        self.writer.put(b" (")?;
        self.writer.string(key)?;
        self.writer.put(b" ")?;
        value.serialize(&mut *self.writer)?;
        self.writer.put(b")")
    }
    fn finish(self) -> Result<(), Error> {
        self.writer.end()?;
        if self.variant {
            self.writer.put(b")")?;
            self.writer.end()?;
        }
        Ok(())
    }
}

macro_rules! integer {
    ($($method:ident: $ty:ty),+) => {$(fn $method(self, value: $ty) -> Result<(), Error> { self.atom(&value.to_string()) })+};
}
impl<'a> ser::Serializer for &'a mut Writer {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Compound<'a>;
    type SerializeTuple = Compound<'a>;
    type SerializeTupleStruct = Compound<'a>;
    type SerializeTupleVariant = Compound<'a>;
    type SerializeMap = ser::Impossible<(), Error>;
    type SerializeStruct = Compound<'a>;
    type SerializeStructVariant = Compound<'a>;
    integer!(serialize_i8:i8, serialize_i16:i16, serialize_i32:i32, serialize_i64:i64,
             serialize_u8:u8, serialize_u16:u16, serialize_u32:u32, serialize_u64:u64);
    fn serialize_bool(self, v: bool) -> Result<(), Error> {
        self.atom(if v { "#t" } else { "#f" })
    }
    fn serialize_f32(self, _: f32) -> Result<(), Error> {
        Err(invalid())
    }
    fn serialize_f64(self, _: f64) -> Result<(), Error> {
        Err(invalid())
    }
    fn serialize_char(self, v: char) -> Result<(), Error> {
        self.string(v.encode_utf8(&mut [0; 4]))
    }
    fn serialize_str(self, v: &str) -> Result<(), Error> {
        self.string(v)
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<(), Error> {
        v.serialize(self)
    }
    fn serialize_none(self) -> Result<(), Error> {
        self.serialize_unit()
    }
    fn serialize_some<T: ?Sized + Serialize>(self, v: &T) -> Result<(), Error> {
        v.serialize(self)
    }
    fn serialize_unit(self) -> Result<(), Error> {
        self.atom("null")
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<(), Error> {
        self.serialize_unit()
    }
    fn serialize_unit_variant(self, _: &'static str, _: u32, v: &'static str) -> Result<(), Error> {
        self.string(v)
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        v: &T,
    ) -> Result<(), Error> {
        v.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        v: &T,
    ) -> Result<(), Error> {
        self.variant(variant)?;
        v.serialize(&mut *self)?;
        self.put(b")")?;
        self.end()
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Compound<'a>, Error> {
        self.start(b"(list")?;
        Ok(Compound {
            writer: self,
            variant: false,
        })
    }
    fn serialize_tuple(self, n: usize) -> Result<Compound<'a>, Error> {
        self.serialize_seq(Some(n))
    }
    fn serialize_tuple_struct(self, _: &'static str, n: usize) -> Result<Compound<'a>, Error> {
        self.serialize_seq(Some(n))
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        n: usize,
    ) -> Result<Compound<'a>, Error> {
        self.variant(variant)?;
        let mut compound = self.serialize_seq(Some(n))?;
        compound.variant = true;
        Ok(compound)
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Error> {
        Err(invalid())
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Compound<'a>, Error> {
        self.start(b"(object")?;
        Ok(Compound {
            writer: self,
            variant: false,
        })
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        n: usize,
    ) -> Result<Compound<'a>, Error> {
        self.variant(variant)?;
        let mut compound = self.serialize_struct("", n)?;
        compound.variant = true;
        Ok(compound)
    }
}
macro_rules! sequence {
    ($trait:ident, $method:ident) => {
        impl ser::$trait for Compound<'_> {
            type Ok = ();
            type Error = Error;
            fn $method<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), Error> {
                self.element(v)
            }
            fn end(self) -> Result<(), Error> {
                self.finish()
            }
        }
    };
}
sequence!(SerializeSeq, serialize_element);
sequence!(SerializeTuple, serialize_element);
sequence!(SerializeTupleStruct, serialize_field);
sequence!(SerializeTupleVariant, serialize_field);
macro_rules! fields {
    ($trait:ident) => {
        impl ser::$trait for Compound<'_> {
            type Ok = ();
            type Error = Error;
            fn serialize_field<T: ?Sized + Serialize>(
                &mut self,
                k: &'static str,
                v: &T,
            ) -> Result<(), Error> {
                self.field(k, v)
            }
            fn end(self) -> Result<(), Error> {
                self.finish()
            }
        }
    };
}
fields!(SerializeStruct);
fields!(SerializeStructVariant);

struct Reader<'a> {
    input: &'a str,
    offset: usize,
    nodes: usize,
}
impl Reader<'_> {
    fn space(&mut self) {
        while self.peek().is_some_and(|b| b" \n\r\t".contains(&b)) {
            self.offset += 1;
        }
    }
    fn peek(&self) -> Option<u8> {
        self.input.as_bytes().get(self.offset).copied()
    }
    fn expect(&mut self, c: u8) -> Result<(), Error> {
        self.space();
        if self.peek() != Some(c) {
            return Err(invalid());
        }
        self.offset += 1;
        Ok(())
    }
    fn string(&mut self) -> Result<String, Error> {
        self.expect(b'"')?;
        let mut out = String::new();
        while let Some(c) = self.input[self.offset..].chars().next() {
            self.offset += c.len_utf8();
            match c {
                '"' => return Ok(out),
                '\\' => {
                    let escaped = match self.peek() {
                        Some(b'n') => '\n',
                        Some(b'r') => '\r',
                        Some(b't') => '\t',
                        Some(b'"') => '"',
                        Some(b'\\') => '\\',
                        _ => return Err(invalid()),
                    };
                    self.offset += 1;
                    out.push(escaped);
                }
                c if c < ' ' => return Err(invalid()),
                c => out.push(c),
            }
        }
        Err(invalid())
    }
    fn atom(&mut self) -> &str {
        let start = self.offset;
        while self.peek().is_some_and(|b| !b" ()\n\r\t".contains(&b)) {
            self.offset += 1;
        }
        &self.input[start..self.offset]
    }
    fn datum(&mut self, depth: usize) -> Result<Value, Error> {
        self.nodes += 1;
        if depth > MAX_DEPTH || self.nodes > self.input.len() {
            return Err(invalid());
        }
        self.space();
        match self.peek() {
            Some(b'"') => Ok(Value::Text(self.string()?)),
            Some(b'(') => {
                self.offset += 1;
                let map = match self.atom() {
                    "object" => true,
                    "list" => false,
                    _ => return Err(invalid()),
                };
                let mut items = Vec::new();
                let mut fields = Vec::new();
                let mut keys = BTreeSet::new();
                loop {
                    self.space();
                    if self.peek() == Some(b')') {
                        self.offset += 1;
                        break;
                    }
                    if map {
                        self.expect(b'(')?;
                        let key = self.datum(depth + 1)?;
                        let Value::Text(ref text) = key else {
                            return Err(invalid());
                        };
                        if !keys.insert(text.clone()) {
                            return Err(invalid());
                        }
                        let value = self.datum(depth + 1)?;
                        self.expect(b')')?;
                        fields.push((key, value));
                    } else {
                        items.push(self.datum(depth + 1)?);
                    }
                }
                Ok(if map {
                    Value::Map(fields)
                } else {
                    Value::Array(items)
                })
            }
            Some(_) => {
                let atom = self.atom();
                match atom {
                    "null" => Ok(Value::Null),
                    "#t" => Ok(Value::Bool(true)),
                    "#f" => Ok(Value::Bool(false)),
                    _ => {
                        let digits = atom.strip_prefix('-').unwrap_or(atom);
                        if atom.len() > 21
                            || digits.is_empty()
                            || !digits.bytes().all(|b| b.is_ascii_digit())
                            || (digits.len() > 1 && digits.starts_with('0'))
                        {
                            return Err(invalid());
                        }
                        let number: i128 = atom.parse().map_err(|_| invalid())?;
                        Ok(Value::Integer(number.try_into().map_err(|_| invalid())?))
                    }
                }
            }
            None => Err(invalid()),
        }
    }
}
