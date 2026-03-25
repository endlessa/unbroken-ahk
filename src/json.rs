//! Hand-rolled JSON serialization and deserialization.
//!
//! Zero dependencies. Produces readable, indented JSON for storage
//! and debugging. Parses JSON back into a simple value tree.

use core::fmt;

// ---------------------------------------------------------------------------
// Value tree
// ---------------------------------------------------------------------------

/// A JSON value. This is the intermediate representation used for
/// serialization and deserialization of all platform types.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    Str(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            JsonValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        self.as_f64().map(|n| n as u64)
    }

    pub fn as_u32(&self) -> Option<u32> {
        self.as_f64().map(|n| n as u32)
    }

    pub fn as_array(&self) -> Option<&Vec<JsonValue>> {
        match self {
            JsonValue::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&Vec<(String, JsonValue)>> {
        match self {
            JsonValue::Object(o) => Some(o),
            _ => None,
        }
    }

    /// Look up a field in an object by key.
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(pairs) => {
                pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v)
            }
            _ => None,
        }
    }

    /// Convenience: get a string field from an object.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(|v| v.as_str())
    }

    /// Convenience: get a bool field from an object.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(|v| v.as_bool())
    }

    /// Convenience: get a u64 field from an object.
    pub fn get_u64(&self, key: &str) -> Option<u64> {
        self.get(key).and_then(|v| v.as_u64())
    }

    /// Convenience: get a u32 field from an object.
    pub fn get_u32(&self, key: &str) -> Option<u32> {
        self.get(key).and_then(|v| v.as_u32())
    }
}

// ---------------------------------------------------------------------------
// Serialization (JsonValue -> String)
// ---------------------------------------------------------------------------

impl fmt::Display for JsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_value(f, self, 0, true)
    }
}

/// Serialize a JsonValue to a compact JSON string (no extra whitespace).
pub fn to_json_compact(value: &JsonValue) -> String {
    let mut out = String::new();
    write_value_to_string(&mut out, value, 0, false);
    out
}

/// Serialize a JsonValue to a pretty-printed JSON string.
pub fn to_json_pretty(value: &JsonValue) -> String {
    format!("{}", value)
}

fn write_value(f: &mut fmt::Formatter<'_>, val: &JsonValue, indent: usize, pretty: bool) -> fmt::Result {
    match val {
        JsonValue::Null => write!(f, "null"),
        JsonValue::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
        JsonValue::Number(n) => {
            if *n == (*n as i64) as f64 {
                write!(f, "{}", *n as i64)
            } else {
                write!(f, "{}", n)
            }
        }
        JsonValue::Str(s) => write!(f, "\"{}\"", escape_json_string(s)),
        JsonValue::Array(items) => {
            if items.is_empty() {
                return write!(f, "[]");
            }
            write!(f, "[")?;
            for (i, item) in items.iter().enumerate() {
                if pretty {
                    write!(f, "\n")?;
                    write_indent(f, indent + 1)?;
                }
                write_value(f, item, indent + 1, pretty)?;
                if i + 1 < items.len() {
                    write!(f, ",")?;
                    if !pretty {
                        write!(f, " ")?;
                    }
                }
            }
            if pretty {
                write!(f, "\n")?;
                write_indent(f, indent)?;
            }
            write!(f, "]")
        }
        JsonValue::Object(pairs) => {
            if pairs.is_empty() {
                return write!(f, "{{}}");
            }
            write!(f, "{{")?;
            for (i, (key, val)) in pairs.iter().enumerate() {
                if pretty {
                    write!(f, "\n")?;
                    write_indent(f, indent + 1)?;
                }
                write!(f, "\"{}\":", escape_json_string(key))?;
                if pretty {
                    write!(f, " ")?;
                }
                write_value(f, val, indent + 1, pretty)?;
                if i + 1 < pairs.len() {
                    write!(f, ",")?;
                }
            }
            if pretty {
                write!(f, "\n")?;
                write_indent(f, indent)?;
            }
            write!(f, "}}")
        }
    }
}

fn write_value_to_string(out: &mut String, val: &JsonValue, indent: usize, pretty: bool) {
    match val {
        JsonValue::Null => out.push_str("null"),
        JsonValue::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        JsonValue::Number(n) => {
            if *n == (*n as i64) as f64 {
                out.push_str(&format!("{}", *n as i64));
            } else {
                out.push_str(&format!("{}", n));
            }
        }
        JsonValue::Str(s) => {
            out.push('"');
            out.push_str(&escape_json_string(s));
            out.push('"');
        }
        JsonValue::Array(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if pretty {
                    out.push('\n');
                    push_indent(out, indent + 1);
                }
                write_value_to_string(out, item, indent + 1, pretty);
                if i + 1 < items.len() {
                    out.push(',');
                    if !pretty {
                        out.push(' ');
                    }
                }
            }
            if pretty {
                out.push('\n');
                push_indent(out, indent);
            }
            out.push(']');
        }
        JsonValue::Object(pairs) => {
            if pairs.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push('{');
            for (i, (key, val)) in pairs.iter().enumerate() {
                if pretty {
                    out.push('\n');
                    push_indent(out, indent + 1);
                }
                out.push('"');
                out.push_str(&escape_json_string(key));
                out.push_str("\":");
                if pretty {
                    out.push(' ');
                }
                write_value_to_string(out, val, indent + 1, pretty);
                if i + 1 < pairs.len() {
                    out.push(',');
                }
            }
            if pretty {
                out.push('\n');
                push_indent(out, indent);
            }
            out.push('}');
        }
    }
}

fn write_indent(f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
    for _ in 0..level {
        write!(f, "  ")?;
    }
    Ok(())
}

fn push_indent(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

fn escape_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Parsing (String -> JsonValue)
// ---------------------------------------------------------------------------

/// Parse a JSON string into a JsonValue.
pub fn parse_json(input: &str) -> Result<JsonValue, JsonError> {
    let mut parser = Parser::new(input);
    let value = parser.parse_value()?;
    parser.skip_whitespace();
    if parser.pos < parser.input.len() {
        return Err(JsonError::TrailingData(parser.pos));
    }
    Ok(value)
}

#[derive(Debug, Clone)]
pub enum JsonError {
    UnexpectedEnd,
    UnexpectedChar(usize, char),
    InvalidNumber(usize),
    InvalidEscape(usize),
    TrailingData(usize),
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonError::UnexpectedEnd => write!(f, "unexpected end of input"),
            JsonError::UnexpectedChar(pos, ch) => {
                write!(f, "unexpected character '{}' at position {}", ch, pos)
            }
            JsonError::InvalidNumber(pos) => write!(f, "invalid number at position {}", pos),
            JsonError::InvalidEscape(pos) => write!(f, "invalid escape at position {}", pos),
            JsonError::TrailingData(pos) => write!(f, "trailing data at position {}", pos),
        }
    }
}

struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn next_byte(&mut self) -> Option<u8> {
        let b = self.input.get(self.pos).copied();
        if b.is_some() {
            self.pos += 1;
        }
        b
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            match self.input[self.pos] {
                b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn expect(&mut self, expected: u8) -> Result<(), JsonError> {
        match self.next_byte() {
            Some(b) if b == expected => Ok(()),
            Some(b) => Err(JsonError::UnexpectedChar(self.pos - 1, b as char)),
            None => Err(JsonError::UnexpectedEnd),
        }
    }

    fn parse_value(&mut self) -> Result<JsonValue, JsonError> {
        self.skip_whitespace();
        match self.peek() {
            None => Err(JsonError::UnexpectedEnd),
            Some(b'"') => self.parse_string().map(JsonValue::Str),
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b't') => self.parse_literal("true", JsonValue::Bool(true)),
            Some(b'f') => self.parse_literal("false", JsonValue::Bool(false)),
            Some(b'n') => self.parse_literal("null", JsonValue::Null),
            Some(b'-') | Some(b'0'..=b'9') => self.parse_number(),
            Some(b) => Err(JsonError::UnexpectedChar(self.pos, b as char)),
        }
    }

    fn parse_string(&mut self) -> Result<String, JsonError> {
        self.expect(b'"')?;
        let mut s = String::new();
        loop {
            match self.next_byte() {
                None => return Err(JsonError::UnexpectedEnd),
                Some(b'"') => return Ok(s),
                Some(b'\\') => {
                    match self.next_byte() {
                        Some(b'"') => s.push('"'),
                        Some(b'\\') => s.push('\\'),
                        Some(b'/') => s.push('/'),
                        Some(b'n') => s.push('\n'),
                        Some(b'r') => s.push('\r'),
                        Some(b't') => s.push('\t'),
                        Some(b'u') => {
                            let cp = self.parse_hex4()?;
                            if let Some(ch) = char::from_u32(cp) {
                                s.push(ch);
                            }
                        }
                        _ => return Err(JsonError::InvalidEscape(self.pos)),
                    }
                }
                Some(b) => s.push(b as char),
            }
        }
    }

    fn parse_hex4(&mut self) -> Result<u32, JsonError> {
        let mut val = 0u32;
        for _ in 0..4 {
            let b = self.next_byte().ok_or(JsonError::UnexpectedEnd)?;
            let digit = match b {
                b'0'..=b'9' => (b - b'0') as u32,
                b'a'..=b'f' => (b - b'a' + 10) as u32,
                b'A'..=b'F' => (b - b'A' + 10) as u32,
                _ => return Err(JsonError::InvalidEscape(self.pos)),
            };
            val = val * 16 + digit;
        }
        Ok(val)
    }

    fn parse_number(&mut self) -> Result<JsonValue, JsonError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        // Integer part
        match self.peek() {
            Some(b'0') => self.pos += 1,
            Some(b'1'..=b'9') => {
                self.pos += 1;
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.pos += 1;
                }
            }
            _ => return Err(JsonError::InvalidNumber(start)),
        }
        // Fraction
        if self.peek() == Some(b'.') {
            self.pos += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(JsonError::InvalidNumber(self.pos));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        // Exponent
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.pos += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(JsonError::InvalidNumber(self.pos));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        let slice = core::str::from_utf8(&self.input[start..self.pos])
            .map_err(|_| JsonError::InvalidNumber(start))?;
        let num: f64 = slice.parse().map_err(|_| JsonError::InvalidNumber(start))?;
        Ok(JsonValue::Number(num))
    }

    fn parse_array(&mut self) -> Result<JsonValue, JsonError> {
        self.expect(b'[')?;
        self.skip_whitespace();
        let mut items = Vec::new();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(JsonValue::Array(items));
        }
        loop {
            items.push(self.parse_value()?);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    return Ok(JsonValue::Array(items));
                }
                Some(b) => return Err(JsonError::UnexpectedChar(self.pos, b as char)),
                None => return Err(JsonError::UnexpectedEnd),
            }
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, JsonError> {
        self.expect(b'{')?;
        self.skip_whitespace();
        let mut pairs = Vec::new();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(JsonValue::Object(pairs));
        }
        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect(b':')?;
            let val = self.parse_value()?;
            pairs.push((key, val));
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(JsonValue::Object(pairs));
                }
                Some(b) => return Err(JsonError::UnexpectedChar(self.pos, b as char)),
                None => return Err(JsonError::UnexpectedEnd),
            }
        }
    }

    fn parse_literal(&mut self, expected: &str, value: JsonValue) -> Result<JsonValue, JsonError> {
        for byte in expected.bytes() {
            match self.next_byte() {
                Some(b) if b == byte => {}
                Some(b) => return Err(JsonError::UnexpectedChar(self.pos - 1, b as char)),
                None => return Err(JsonError::UnexpectedEnd),
            }
        }
        Ok(value)
    }
}

// ---------------------------------------------------------------------------
// ToJson / FromJson traits for our domain types
// ---------------------------------------------------------------------------

/// Convert a type to a JsonValue for serialization.
pub trait ToJson {
    fn to_json(&self) -> JsonValue;
}

/// Parse a type from a JsonValue.
pub trait FromJson: Sized {
    fn from_json(value: &JsonValue) -> Result<Self, JsonError>;
}

// Helper to build objects ergonomically
pub fn obj(pairs: Vec<(&str, JsonValue)>) -> JsonValue {
    JsonValue::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

pub fn str_val(s: &str) -> JsonValue {
    JsonValue::Str(s.to_string())
}

pub fn opt_str(s: &Option<String>) -> JsonValue {
    match s {
        Some(s) => JsonValue::Str(s.clone()),
        None => JsonValue::Null,
    }
}

pub fn str_array(items: &[String]) -> JsonValue {
    JsonValue::Array(items.iter().map(|s| JsonValue::Str(s.clone())).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_simple() {
        let val = obj(vec![
            ("name", str_val("hello")),
            ("count", JsonValue::Number(42.0)),
            ("active", JsonValue::Bool(true)),
            ("tags", str_array(&["a".into(), "b".into()])),
        ]);
        let json_str = to_json_pretty(&val);
        let parsed = parse_json(&json_str).unwrap();
        assert_eq!(val, parsed);
    }

    #[test]
    fn parse_empty_object() {
        let val = parse_json("{}").unwrap();
        assert_eq!(val, JsonValue::Object(vec![]));
    }

    #[test]
    fn parse_null() {
        let val = parse_json("null").unwrap();
        assert_eq!(val, JsonValue::Null);
    }

    #[test]
    fn escape_round_trip() {
        let val = JsonValue::Str("line1\nline2\ttab\"quote".into());
        let s = to_json_compact(&val);
        let parsed = parse_json(&s).unwrap();
        assert_eq!(val, parsed);
    }
}
