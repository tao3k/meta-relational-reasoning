//! Bounded inert Scheme v1 metadata. No reader extensions or evaluation.
use std::collections::BTreeMap;
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Value {
    String(String),
    List(Vec<Value>),
    Object(BTreeMap<String, Value>),
    Integer(usize),
    Bool(bool),
    Null,
}
impl Value {
    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Object(v) => v.get(key),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(v) => Some(v),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&Vec<Self>> {
        match self {
            Self::List(v) => Some(v),
            _ => None,
        }
    }
    pub fn integer(&self) -> Option<usize> {
        match self {
            Self::Integer(v) => Some(*v),
            _ => None,
        }
    }
    pub fn encode(&self) -> String {
        match self {
            Self::String(v) => quote(v),
            Self::Integer(v) => v.to_string(),
            Self::Bool(v) => if *v { "#t" } else { "#f" }.into(),
            Self::Null => "null".into(),
            Self::List(v) => format!(
                "(list{})",
                v.iter()
                    .map(|v| format!(" {}", v.encode()))
                    .collect::<String>()
            ),
            Self::Object(v) => format!(
                "(object{})",
                v.iter()
                    .map(|(k, v)| format!(" ({} {})", quote(k), v.encode()))
                    .collect::<String>()
            ),
        }
    }
}
pub(crate) fn quote(text: &str) -> String {
    let mut result = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            _ => result.push(c),
        }
    }
    result.push('"');
    result
}
pub(crate) fn decode(bytes: &[u8]) -> Option<Value> {
    if bytes.len() > crate::worker_wire::PAYLOAD_LIMIT {
        return None;
    }
    let mut reader = Reader {
        input: std::str::from_utf8(bytes).ok()?,
        nodes: 0,
    };
    let result = reader.value(0)?;
    reader.space();
    if reader.input.is_empty() {
        Some(result)
    } else {
        None
    }
}
struct Reader<'a> {
    input: &'a str,
    nodes: usize,
}
impl Reader<'_> {
    fn space(&mut self) {
        self.input = self.input.trim_start_matches([' ', '\n', '\r', '\t']);
    }
    fn consume(&mut self, token: &str) -> bool {
        if let Some(rest) = self.input.strip_prefix(token) {
            self.input = rest;
            true
        } else {
            false
        }
    }
    fn string(&mut self) -> Option<String> {
        if !self.consume("\"") {
            return None;
        }
        let mut result = String::new();
        loop {
            let c = self.input.chars().next()?;
            self.input = &self.input[c.len_utf8()..];
            match c {
                '"' => return Some(result),
                '\\' => {
                    let c = self.input.chars().next()?;
                    self.input = &self.input[c.len_utf8()..];
                    result.push(match c {
                        '"' => '"',
                        '\\' => '\\',
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        _ => return None,
                    });
                }
                c if !c.is_control() => result.push(c),
                _ => return None,
            }
        }
    }
    fn value(&mut self, depth: usize) -> Option<Value> {
        self.nodes += 1;
        if depth > 64 || self.nodes > 262144 {
            return None;
        }
        self.space();
        if self.input.starts_with('"') {
            return Some(Value::String(self.string()?));
        }
        if self.input.starts_with("(list")
            && self
                .input
                .get(5..)
                .is_some_and(|s| s.starts_with([' ', '\n', '\r', '\t', ')']))
            && self.consume("(list")
        {
            let mut rows = Vec::new();
            loop {
                self.space();
                if self.consume(")") {
                    return Some(Value::List(rows));
                }
                rows.push(self.value(depth + 1)?);
            }
        }
        if self.input.starts_with("(object")
            && self
                .input
                .get(7..)
                .is_some_and(|s| s.starts_with([' ', '\n', '\r', '\t', ')']))
            && self.consume("(object")
        {
            let mut rows = BTreeMap::new();
            loop {
                self.space();
                if self.consume(")") {
                    return Some(Value::Object(rows));
                }
                if !self.consume("(") {
                    return None;
                }
                self.space();
                let key = self.string()?;
                let value = self.value(depth + 1)?;
                self.space();
                if !self.consume(")") || rows.insert(key, value).is_some() {
                    return None;
                }
            }
        }
        let end = self
            .input
            .find([' ', '\n', '\r', '\t', ')'])
            .unwrap_or(self.input.len());
        let token = &self.input[..end];
        self.input = &self.input[end..];
        match token {
            "#t" => Some(Value::Bool(true)),
            "#f" => Some(Value::Bool(false)),
            "null" => Some(Value::Null),
            _ if !token.is_empty()
                && token.len() <= 20
                && token.bytes().all(|b| b.is_ascii_digit())
                && (token == "0" || !token.starts_with('0')) =>
            {
                token.parse().ok().map(Value::Integer)
            }
            _ => None,
        }
    }
}
