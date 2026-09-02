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

    // There are deliberately NO as_u64/as_u32 helpers: `n as u64` on an
    // f64 silently saturates negative, fractional, and overflowing
    // values — every integer read goes through json_types' strict
    // helpers, which reject those instead.

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
}

// ---------------------------------------------------------------------------
// Serialization (JsonValue -> String)
// ---------------------------------------------------------------------------

impl fmt::Display for JsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Single serializer: Display delegates to the same writer the
        // compact path uses, so pretty and compact output can never diverge.
        let mut out = String::new();
        write_value_to_string(&mut out, self, 0, true);
        f.write_str(&out)
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
    // Write straight into the result buffer — going through Display would
    // build the whole output twice for large values.
    let mut out = String::new();
    write_value_to_string(&mut out, value, 0, true);
    out
}

fn write_value_to_string(out: &mut String, val: &JsonValue, indent: usize, pretty: bool) {
    match val {
        JsonValue::Null => out.push_str("null"),
        JsonValue::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        JsonValue::Number(n) => {
            if !n.is_finite() {
                // JSON has no NaN/Infinity — emit null rather than invalid output.
                out.push_str("null");
            } else if *n == (*n as i64) as f64
                && *n >= -9_223_372_036_854_775_808.0
                && *n < 9_223_372_036_854_775_808.0
            {
                // Integer formatting only inside i64's exact range: at 2^63
                // the cast saturates to i64::MAX and would print off by one.
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
    MissingField(String),
    InvalidField(String, String),
    UnknownField(String, String, String),
    TooDeep(usize),
    DuplicateKey(String),
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
            JsonError::MissingField(name) => {
                write!(f, "missing or invalid required field '{}'", name)
            }
            JsonError::InvalidField(name, expected) => {
                write!(f, "invalid field '{}': expected {}", name, expected)
            }
            JsonError::UnknownField(what, name, valid) => {
                write!(f, "unknown {} field '{}'; valid keys: {}", what, name, valid)
            }
            JsonError::TooDeep(pos) => {
                write!(f, "nesting deeper than {} levels at position {}", MAX_DEPTH, pos)
            }
            JsonError::DuplicateKey(key) => {
                write!(f, "duplicate object key '{}'", key)
            }
        }
    }
}

/// Maximum container nesting depth. Recursion in the parser is bounded by
/// this, so hostile or malformed input returns an error instead of
/// overflowing the stack and killing the process.
const MAX_DEPTH: usize = 128;

struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
    depth: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            pos: 0,
            depth: 0,
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
        if self.depth >= MAX_DEPTH {
            return Err(JsonError::TooDeep(self.pos));
        }
        self.depth += 1;
        let result = match self.peek() {
            None => Err(JsonError::UnexpectedEnd),
            Some(b'"') => self.parse_string().map(JsonValue::Str),
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b't') => self.parse_literal("true", JsonValue::Bool(true)),
            Some(b'f') => self.parse_literal("false", JsonValue::Bool(false)),
            Some(b'n') => self.parse_literal("null", JsonValue::Null),
            Some(b'-') | Some(b'0'..=b'9') => self.parse_number(),
            Some(b) => Err(JsonError::UnexpectedChar(self.pos, b as char)),
        };
        self.depth -= 1;
        result
    }

    fn parse_string(&mut self) -> Result<String, JsonError> {
        self.expect(b'"')?;
        let mut s = String::new();
        // Start of the current run of unescaped bytes. The input came from a
        // &str, so any run delimited by ASCII bytes ('"' or '\\') is valid
        // UTF-8 and can be copied wholesale — multi-byte sequences are never
        // split because their continuation bytes are all >= 0x80.
        let mut run_start = self.pos;
        loop {
            match self.next_byte() {
                None => return Err(JsonError::UnexpectedEnd),
                Some(b'"') => {
                    self.push_run(&mut s, run_start, self.pos - 1);
                    return Ok(s);
                }
                Some(b'\\') => {
                    self.push_run(&mut s, run_start, self.pos - 1);
                    match self.next_byte() {
                        Some(b'"') => s.push('"'),
                        Some(b'\\') => s.push('\\'),
                        Some(b'/') => s.push('/'),
                        Some(b'b') => s.push('\u{0008}'),
                        Some(b'f') => s.push('\u{000C}'),
                        Some(b'n') => s.push('\n'),
                        Some(b'r') => s.push('\r'),
                        Some(b't') => s.push('\t'),
                        Some(b'u') => {
                            let cp = self.parse_hex4()?;
                            let ch = match cp {
                                // High surrogate: must be followed by \u + low
                                // surrogate; combine into one code point.
                                0xD800..=0xDBFF => {
                                    if self.next_byte() != Some(b'\\')
                                        || self.next_byte() != Some(b'u')
                                    {
                                        return Err(JsonError::InvalidEscape(self.pos));
                                    }
                                    let lo = self.parse_hex4()?;
                                    if !(0xDC00..=0xDFFF).contains(&lo) {
                                        return Err(JsonError::InvalidEscape(self.pos));
                                    }
                                    let combined =
                                        0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
                                    char::from_u32(combined)
                                        .ok_or(JsonError::InvalidEscape(self.pos))?
                                }
                                // Lone low surrogate is invalid.
                                0xDC00..=0xDFFF => {
                                    return Err(JsonError::InvalidEscape(self.pos))
                                }
                                _ => char::from_u32(cp)
                                    .ok_or(JsonError::InvalidEscape(self.pos))?,
                            };
                            s.push(ch);
                        }
                        _ => return Err(JsonError::InvalidEscape(self.pos)),
                    }
                    run_start = self.pos;
                }
                // RFC 8259 §7: control characters MUST be escaped inside
                // strings — a raw newline/tab here is input every
                // conforming parser rejects, and the strict loaders must
                // not quietly accept what standard tools would refuse.
                // (Our own serializer escapes all of these, so accepted
                // documents still round-trip.)
                Some(b) if b < 0x20 => {
                    return Err(JsonError::UnexpectedChar(self.pos - 1, b as char));
                }
                Some(_) => {}
            }
        }
    }

    /// Append input[start..end] (a run of unescaped bytes) to the output
    /// string. Valid UTF-8 by construction — see parse_string. If a future
    /// refactor ever feeds the parser non-&str bytes, fail loudly rather
    /// than silently dropping content.
    fn push_run(&self, s: &mut String, start: usize, end: usize) {
        if end > start {
            s.push_str(
                core::str::from_utf8(&self.input[start..end])
                    .expect("parser input must be valid UTF-8 (constructed from &str)"),
            );
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
        // f64's FromStr saturates overflow to infinity (1e999 -> inf), a
        // value the serializer can only emit as null — accepting it would
        // make parse-then-reserialize silently change the value. Reject,
        // like strict parsers do ("number out of range").
        if !num.is_finite() {
            return Err(JsonError::InvalidNumber(start));
        }
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
        let mut keys = std::collections::HashSet::new();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(JsonValue::Object(pairs));
        }
        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            // RFC 8259 leaves duplicate keys implementation-defined;
            // silently keeping one of them is exactly the kind of quiet
            // data drop this platform's strict loaders exist to prevent.
            // (Set-based check — a linear scan would hand large flat
            // objects a quadratic parse-time DoS.)
            if !keys.insert(key.clone()) {
                return Err(JsonError::DuplicateKey(key));
            }
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

/// The RFC 8259 whitespace set — the SAME set parse_json skips. The
/// splitter must not accept separators (form feed, vertical tab, NBSP)
/// that the strict parser rejects, or load_registry would call a file
/// healthy that parse_json calls corrupt.
fn is_json_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

/// Split a JSON array document into raw per-element slices WITHOUT parsing
/// the elements, so a parse-level defect inside one element (a duplicate
/// object key, a malformed number) can be confined to that element instead
/// of failing the whole document. Used by the registry loader, whose
/// contract is per-entry tolerance: one damaged entry must not discard the
/// healthy entries around it.
///
/// Only the array skeleton is validated here — the opening/closing
/// brackets, commas, and string boundaries. Every returned slice still
/// needs `parse_json` to be trusted.
pub fn split_top_level_array(input: &str) -> Result<Vec<&str>, JsonError> {
    let bytes = input.as_bytes();
    let mut pos = 0;
    while pos < bytes.len() && is_json_whitespace(bytes[pos]) {
        pos += 1;
    }
    match bytes.get(pos) {
        None => return Err(JsonError::UnexpectedEnd),
        Some(b'[') => pos += 1,
        Some(&b) => return Err(JsonError::UnexpectedChar(pos, b as char)),
    }
    let mut elements = Vec::new();
    let mut lookahead = pos;
    while lookahead < bytes.len() && is_json_whitespace(bytes[lookahead]) {
        lookahead += 1;
    }
    if bytes.get(lookahead) == Some(&b']') {
        pos = lookahead + 1;
    } else {
        loop {
            // Every element must BEGIN like a JSON value begins. Anything
            // else at this position (a stray comma, a form feed, '@') is
            // skeleton damage that parse_json would reject file-level —
            // classifying it as one damaged "entry" would let the two
            // loaders disagree about the same file.
            while pos < bytes.len() && is_json_whitespace(bytes[pos]) {
                pos += 1;
            }
            match bytes.get(pos) {
                None => return Err(JsonError::UnexpectedEnd),
                Some(b'"' | b'{' | b'[' | b'-' | b'0'..=b'9' | b't' | b'f' | b'n') => {}
                Some(&b) => return Err(JsonError::UnexpectedChar(pos, b as char)),
            }
            let start = pos;
            // Scan one element: a bracket STACK plus string state finds
            // the ',' or ']' that ends it — brackets inside strings don't
            // count, escaped quotes don't end strings, and the stack
            // (not a bare depth counter) catches a closer of the wrong
            // KIND (']' terminating '{'), which parse_json rejects
            // file-level and the splitter must too. The stack is also
            // capped so nesting parse_json would refuse as TooDeep is
            // refused here, never classified as a healthy file.
            // (The whole document sits one level inside the outer array:
            // parse_json allows MAX_DEPTH nested values total, so an
            // element's own bracket nesting may reach MAX_DEPTH - 1.)
            let mut stack: Vec<u8> = Vec::new();
            let mut in_string = false;
            let mut escaped = false;
            // Once the element's value has closed at stack depth 0 — a
            // string's closing quote, a bracket returning to depth 0, or
            // a scalar ending at whitespace — only whitespace may follow
            // before the ',' or ']'. Anything else means the comma
            // BETWEEN elements is missing: skeleton damage, which must
            // not merge two healthy entries into one unparseable "entry"
            // that gets dropped.
            let mut value_closed = false;
            let mut in_scalar = false;
            let end;
            loop {
                let b = match bytes.get(pos) {
                    Some(&b) => b,
                    None => return Err(JsonError::UnexpectedEnd),
                };
                if in_string {
                    if escaped {
                        escaped = false;
                    } else if b == b'\\' {
                        escaped = true;
                    } else if b == b'"' {
                        in_string = false;
                        if stack.is_empty() {
                            value_closed = true;
                        }
                    }
                } else if stack.is_empty() && value_closed {
                    match b {
                        b',' | b']' => {
                            end = pos;
                            break;
                        }
                        _ if is_json_whitespace(b) => {}
                        _ => return Err(JsonError::UnexpectedChar(pos, b as char)),
                    }
                } else if stack.is_empty() && in_scalar {
                    match b {
                        b',' | b']' => {
                            end = pos;
                            break;
                        }
                        _ if is_json_whitespace(b) => {
                            in_scalar = false;
                            value_closed = true;
                        }
                        // A new value starting right after a scalar is
                        // the same missing-comma damage.
                        b'"' | b'{' | b'[' => {
                            return Err(JsonError::UnexpectedChar(pos, b as char))
                        }
                        _ => {}
                    }
                } else if stack.is_empty() {
                    match b {
                        b'"' => in_string = true,
                        b'[' | b'{' => stack.push(b),
                        b',' | b']' => {
                            end = pos;
                            break;
                        }
                        _ if is_json_whitespace(b) => {}
                        // Start of a scalar (number, true/false/null, or
                        // garbage the per-element parse will reject).
                        _ => in_scalar = true,
                    }
                } else {
                    match b {
                        b'"' => in_string = true,
                        b'[' | b'{' => {
                            // MAX_DEPTH - 2: the outer array is one
                            // parse_value level and the innermost VALUE
                            // inside the deepest bracket is another, so
                            // an element's bracket chain must stop two
                            // short of the parser's limit. Conservative
                            // in the safe direction: a content-free
                            // bracket chain at the exact boundary is
                            // refused here even though parse_json would
                            // squeak it through — file-level TooDeep for
                            // a pathological file beats calling healthy
                            // a file the parser rejects.
                            if stack.len() >= MAX_DEPTH - 2 {
                                return Err(JsonError::TooDeep(pos));
                            }
                            stack.push(b);
                        }
                        b']' | b'}' => {
                            let opener = stack.pop().unwrap_or(0);
                            let kind_matches = (opener == b'[' && b == b']')
                                || (opener == b'{' && b == b'}');
                            if !kind_matches {
                                return Err(JsonError::UnexpectedChar(pos, b as char));
                            }
                            if stack.is_empty() {
                                value_closed = true;
                            }
                        }
                        _ => {}
                    }
                }
                pos += 1;
            }
            // Never empty: start points at a verified value-starter byte
            // (trailing/doubled commas already errored at the check
            // above), so only trailing whitespace needs trimming.
            let piece = input[start..end]
                .trim_matches(|c: char| c.is_ascii() && is_json_whitespace(c as u8));
            elements.push(piece);
            let closed = bytes[end] == b']';
            pos = end + 1;
            if closed {
                break;
            }
        }
    }
    while pos < bytes.len() && is_json_whitespace(bytes[pos]) {
        pos += 1;
    }
    if pos < bytes.len() {
        return Err(JsonError::TrailingData(pos));
    }
    Ok(elements)
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
    fn split_top_level_array_basic() {
        assert_eq!(split_top_level_array("[]").unwrap(), Vec::<&str>::new());
        assert_eq!(split_top_level_array(" [ ] ").unwrap(), Vec::<&str>::new());
        assert_eq!(split_top_level_array("[1, 2, 3]").unwrap(), vec!["1", "2", "3"]);
        assert_eq!(
            split_top_level_array(r#"[{"a": [1, 2]}, {"b": {"c": 3}}]"#).unwrap(),
            vec![r#"{"a": [1, 2]}"#, r#"{"b": {"c": 3}}"#]
        );
    }

    #[test]
    fn split_top_level_array_ignores_brackets_in_strings() {
        // Brackets, commas, and escaped quotes inside strings must not
        // end an element.
        let input = r#"[{"name": "a,b]c"}, {"name": "d\"e[f"}]"#;
        let parts = split_top_level_array(input).unwrap();
        assert_eq!(parts.len(), 2);
        assert!(parse_json(parts[0]).is_ok());
        assert!(parse_json(parts[1]).is_ok());
    }

    #[test]
    fn split_top_level_array_confines_element_defects() {
        // The whole point: an element the strict parser rejects (duplicate
        // key) is still isolated as its own slice, leaving the others
        // parseable.
        let input = r#"[{"id": "a"}, {"k": 1, "k": 2}, {"id": "b"}]"#;
        let parts = split_top_level_array(input).unwrap();
        assert_eq!(parts.len(), 3);
        assert!(parse_json(parts[0]).is_ok());
        assert!(matches!(parse_json(parts[1]), Err(JsonError::DuplicateKey(_))));
        assert!(parse_json(parts[2]).is_ok());
    }

    #[test]
    fn split_top_level_array_rejects_structural_damage() {
        assert!(matches!(split_top_level_array(""), Err(JsonError::UnexpectedEnd)));
        assert!(matches!(split_top_level_array("{}"), Err(JsonError::UnexpectedChar(_, _))));
        assert!(matches!(split_top_level_array("[1, 2"), Err(JsonError::UnexpectedEnd)));
        assert!(matches!(split_top_level_array("[1] junk"), Err(JsonError::TrailingData(_))));
        // Comma damage is skeleton damage, never a phantom empty "entry".
        assert!(matches!(split_top_level_array("[1, 2,]"), Err(JsonError::UnexpectedChar(_, _))));
        assert!(matches!(split_top_level_array("[1,, 2]"), Err(JsonError::UnexpectedChar(_, _))));
        assert!(matches!(split_top_level_array("[,]"), Err(JsonError::UnexpectedChar(_, _))));
        // A MISSING comma between two elements is the same class: it must
        // error file-level, never merge two healthy entries into one
        // unparseable slice that both get dropped.
        assert!(matches!(
            split_top_level_array(r#"[{"id": "a"} {"id": "b"}]"#),
            Err(JsonError::UnexpectedChar(_, _))
        ));
        assert!(matches!(
            split_top_level_array(r#"["a" "b"]"#),
            Err(JsonError::UnexpectedChar(_, _))
        ));
        // The splitter accepts EXACTLY the parser's whitespace set: a
        // form feed between elements is rejected by parse_json, so the
        // splitter must not call that file healthy.
        assert!(matches!(
            split_top_level_array("[1,\u{0C}2]"),
            Err(JsonError::UnexpectedChar(_, _))
        ));
        // A closer of the wrong KIND (']' terminating '{') is the same
        // file-level damage parse_json reports — never a healthy file
        // with one bad entry.
        assert!(matches!(
            split_top_level_array("[{]]"),
            Err(JsonError::UnexpectedChar(_, _))
        ));
        assert!(matches!(
            split_top_level_array(r#"[{"a": [1}]"#),
            Err(JsonError::UnexpectedChar(_, _))
        ));
        // Nesting parse_json refuses as TooDeep is refused here too —
        // including exactly at the boundary the parser enforces.
        let deep = format!("[{}1{}]", "[".repeat(200), "]".repeat(200));
        assert!(parse_json(&deep).is_err());
        assert!(matches!(split_top_level_array(&deep), Err(JsonError::TooDeep(_))));
        let boundary = format!("[{}1{}]", "[".repeat(127), "]".repeat(127));
        assert!(parse_json(&boundary).is_err(), "parser should reject depth-127 element");
        assert!(split_top_level_array(&boundary).is_err());
        // Comfortably inside the limit, both accept.
        let fine = format!("[{}1{}]", "[".repeat(100), "]".repeat(100));
        assert!(parse_json(&fine).is_ok());
        assert!(split_top_level_array(&fine).is_ok());
        // Scalar elements get the same missing-comma detection as
        // strings and objects.
        assert!(matches!(split_top_level_array("[1 2]"), Err(JsonError::UnexpectedChar(_, _))));
        assert!(matches!(
            split_top_level_array(r#"[1 {"a": 2}]"#),
            Err(JsonError::UnexpectedChar(_, _))
        ));
        assert!(matches!(
            split_top_level_array("[true false]"),
            Err(JsonError::UnexpectedChar(_, _))
        ));
        // Scalars themselves still split fine.
        assert_eq!(split_top_level_array("[1 , true, null]").unwrap(), vec!["1", "true", "null"]);
    }

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

    #[test]
    fn utf8_round_trip() {
        let original = "héllo wörld — 你好 🎉";
        let val = JsonValue::Str(original.into());
        let s = to_json_compact(&val);
        let parsed = parse_json(&s).unwrap();
        assert_eq!(parsed.as_str(), Some(original));
        // A second cycle must be stable (no compounding corruption).
        let s2 = to_json_compact(&parsed);
        assert_eq!(s, s2);
    }

    #[test]
    fn utf8_raw_parse() {
        let parsed = parse_json("\"héllo — 你好\"").unwrap();
        assert_eq!(parsed.as_str(), Some("héllo — 你好"));
    }

    #[test]
    fn surrogate_pair_escape() {
        // Build the 12-char escape sequence 😀 — the standard
        // JSON encoding of U+1F600 (grinning face) as a surrogate pair.
        let input = format!("\"{}ud83d{}ude00\"", '\\', '\\');
        let parsed = parse_json(&input).unwrap();
        assert_eq!(parsed.as_str(), Some("\u{1F600}"));
    }

    #[test]
    fn lone_surrogate_is_error() {
        assert!(parse_json(r#""\ud83d""#).is_err());
        assert!(parse_json(r#""\ude00""#).is_err());
        assert!(parse_json(r#""\ud83dA""#).is_err());
    }

    #[test]
    fn overflowing_numbers_are_rejected_not_infinity() {
        // f64 FromStr saturates 1e999 to infinity — a value the
        // serializer can only emit as null. Reject like strict parsers.
        assert!(matches!(parse_json("1e999"), Err(JsonError::InvalidNumber(_))));
        assert!(matches!(parse_json("-1e999"), Err(JsonError::InvalidNumber(_))));
        // The extremes of the finite range still parse.
        assert!(parse_json("1e308").is_ok());
        assert!(parse_json("-1.7976931348623157e308").is_ok());
    }

    #[test]
    fn raw_control_characters_in_strings_are_rejected() {
        // RFC 8259 §7: control characters must be escaped. A literal
        // newline/tab inside a string is input every conforming parser
        // rejects — accepting it would quietly bless damage that other
        // tools refuse.
        assert!(parse_json("\"a\nb\"").is_err());
        assert!(parse_json("\"a\tb\"").is_err());
        assert!(parse_json("\"a\u{0001}b\"").is_err());
        // Their escaped spellings are fine and round-trip.
        assert_eq!(parse_json(r#""a\nb""#).unwrap().as_str(), Some("a\nb"));
        let val = JsonValue::Str("a\nb\tc\u{0001}d".into());
        let s = to_json_compact(&val);
        assert_eq!(parse_json(&s).unwrap().as_str(), Some("a\nb\tc\u{0001}d"));
    }

    #[test]
    fn backspace_formfeed_escapes() {
        let parsed = parse_json(r#""a\bb\fc""#).unwrap();
        assert_eq!(parsed.as_str(), Some("a\u{0008}b\u{000C}c"));
        // Round-trip through the serializer's \uXXXX form.
        let val = JsonValue::Str("a\u{0008}b\u{000C}c".into());
        let s = to_json_compact(&val);
        assert_eq!(parse_json(&s).unwrap(), val);
    }

    #[test]
    fn deep_nesting_errors_instead_of_overflowing() {
        // Hostile/malformed input must return an error, not crash the
        // process with a stack overflow.
        let deep = "[".repeat(200_000);
        assert!(matches!(parse_json(&deep), Err(JsonError::TooDeep(_))));
        let deep_obj = r#"{"a":"#.repeat(100_000) + "1";
        assert!(matches!(parse_json(&deep_obj), Err(JsonError::TooDeep(_))));
        // Reasonable nesting still parses.
        let ok = "[".repeat(50) + &"]".repeat(50);
        assert!(parse_json(&ok).is_ok());
    }

    #[test]
    fn duplicate_object_keys_error() {
        // Silently keeping one of two duplicate keys is a quiet data
        // drop — reject at the parser so every loader is covered.
        assert!(matches!(
            parse_json(r#"{"a": 1, "a": 2}"#),
            Err(JsonError::DuplicateKey(_))
        ));
        assert!(parse_json(r#"{"a": 1, "b": 2}"#).is_ok());
        // Nested objects each get their own key space.
        assert!(parse_json(r#"{"a": {"x": 1}, "b": {"x": 2}}"#).is_ok());
    }

    #[test]
    fn non_finite_numbers_serialize_as_null() {
        assert_eq!(to_json_compact(&JsonValue::Number(f64::NAN)), "null");
        assert_eq!(to_json_compact(&JsonValue::Number(f64::INFINITY)), "null");
    }

    #[test]
    fn i64_boundary_numbers_round_trip() {
        // 2^63 saturates an i64 cast; it must not print off by one.
        let two_63 = 9_223_372_036_854_775_808.0f64;
        let s = to_json_compact(&JsonValue::Number(two_63));
        assert_ne!(s, "9223372036854775807");
        let parsed = parse_json(&s).unwrap();
        assert_eq!(parsed.as_f64(), Some(two_63));
        // i64::MIN is exactly representable and stays integer-formatted.
        let min = -9_223_372_036_854_775_808.0f64;
        assert_eq!(to_json_compact(&JsonValue::Number(min)), "-9223372036854775808");
    }

    #[test]
    fn top_level_trailing_data_is_rejected() {
        // INVARIANT: parse_json accepts a document only when NOTHING but
        // whitespace follows the first value — a healthy-looking prefix of
        // a damaged registry/run file must never be returned as a valid
        // value, and parse_json must agree with split_top_level_array's
        // own trailing-data check about the same file.
        assert!(matches!(parse_json("{} x"), Err(JsonError::TrailingData(_))));
        assert!(matches!(parse_json("[1] 2"), Err(JsonError::TrailingData(_))));
        // Leading-zero numbers are rejected ONLY via this check: "01"
        // parses as the value 0 with '1' left over.
        assert!(matches!(parse_json("01"), Err(JsonError::TrailingData(_))));
        // Trailing whitespace alone stays accepted.
        assert!(parse_json("{} ").is_ok());
        assert!(parse_json("[1]\n").is_ok());
    }

    #[test]
    fn strict_number_grammar_rejects_nonconforming_literals() {
        // INVARIANT: the parser enforces RFC 8259's number grammar itself
        // rather than deferring to f64::FromStr — FromStr happily parses
        // "1." as 1.0, so a regression here would silently accept
        // non-JSON input and re-serialize it as a DIFFERENT literal,
        // diverging from every conforming parser.
        for bad in ["1.", "1e", "1e+", "-", ".5", "+1"] {
            assert!(
                matches!(
                    parse_json(bad),
                    Err(JsonError::InvalidNumber(_)) | Err(JsonError::UnexpectedChar(_, _))
                ),
                "{:?} must be rejected",
                bad
            );
        }
        // The conforming spellings still parse to their exact values.
        assert_eq!(parse_json("1.5").unwrap().as_f64(), Some(1.5));
        assert_eq!(parse_json("1e5").unwrap().as_f64(), Some(100_000.0));
        assert_eq!(parse_json("-0.5").unwrap().as_f64(), Some(-0.5));
    }

    #[test]
    fn unknown_and_malformed_escapes_are_rejected() {
        // INVARIANT: an unrecognized escape letter and a non-hex digit
        // inside \uXXXX are errors, never leniently decoded into some
        // other character — the strict loaders must not read string
        // content that every conforming parser refuses.
        assert!(matches!(parse_json(r#""\q""#), Err(JsonError::InvalidEscape(_))));
        assert!(matches!(parse_json(r#""\u00gz""#), Err(JsonError::InvalidEscape(_))));
    }

    #[test]
    fn object_keys_are_escaped_like_values() {
        // INVARIANT: serializer escaping applies to object KEYS, not just
        // string values — metadata keys are user-controlled, and an
        // unescaped quote or newline in a key would write a registry file
        // the parser itself calls corrupt (data loss on the next load).
        let val = obj(vec![("we\"ird\nkey", str_val("v"))]);
        let compact = to_json_compact(&val);
        assert_eq!(parse_json(&compact).unwrap(), val);
        let pretty = to_json_pretty(&val);
        assert_eq!(parse_json(&pretty).unwrap(), val);
        // The escaped spellings actually appear in the output.
        assert_eq!(compact, r#"{"we\"ird\nkey":"v"}"#);
    }

    #[test]
    fn pretty_and_compact_golden_shapes() {
        // INVARIANT: the exact layout of emitted JSON — two-space
        // indentation growing one level per nesting depth, pretty newline
        // placement, compact's ", " array separator and no-space object
        // colon — is the module's stated purpose (readable, indented JSON
        // for storage); every consumer test only re-parses the output, so
        // the shape is pinned here.
        let val = obj(vec![
            ("a", JsonValue::Number(1.0)),
            ("b", JsonValue::Array(vec![JsonValue::Number(2.0), str_val("x")])),
            ("c", obj(vec![("d", JsonValue::Bool(true))])),
        ]);
        assert_eq!(to_json_compact(&val), r#"{"a":1,"b":[2, "x"],"c":{"d":true}}"#);
        let expected_pretty = "{\n  \"a\": 1,\n  \"b\": [\n    2,\n    \"x\"\n  ],\n  \"c\": {\n    \"d\": true\n  }\n}";
        assert_eq!(to_json_pretty(&val), expected_pretty);
        // Display delegates to the same writer as pretty — the two must
        // never diverge.
        assert_eq!(format!("{}", val), expected_pretty);
        // Empty containers stay inline even in pretty mode.
        assert_eq!(
            to_json_pretty(&obj(vec![("e", JsonValue::Array(vec![]))])),
            "{\n  \"e\": []\n}"
        );
    }
}
