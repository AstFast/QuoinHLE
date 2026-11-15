//! JSON, read and written here rather than by a crate.
//!
//! The machine's own numbers — screen, keypad, identity strings — live in a file a person edits (see
//! [`crate::device_file`]), so this host needs to read JSON and write it back. That is the whole job:
//! one value type, one parser, one writer. No schema, no derive, no reflection.
//!
//! # Why this is hand-written
//!
//! `serde_json` is the obvious answer and using it would not be wrong; `Cargo.toml`'s rule is that a
//! crate gets added when the function needs one. What decides against it here is the *writer*: the
//! file it produces is meant to be read next to the struct it describes, so its keys come out in the
//! order that struct declares them. `serde_json::Map` is a `BTreeMap` unless `preserve_order` is on,
//! and that feature swaps the map for `indexmap` — a second crate to make the output alphabetical or
//! insertion-ordered, where the ordering is the only thing that was wanted. The parser side is a
//! hundred lines of a grammar that has been stable since 2006 and is written out below.
//!
//! Three properties matter more than speed here, and each one has a test:
//!
//! - **A number keeps the digits it was written with.** [`Value::Number`] holds the literal, not an
//!   `f64`: `storage_total` is a `u64` and 16 MiB of RAM must not come back as `16777216.0` or as
//!   something rounded past 2^53.
//! - **An error says where.** [`Error`] carries a line and a column, because the file is edited by a
//!   person and "expected `,` or `}`" without a position is a search, not a message.
//! - **Nothing is silently the same.** A duplicate key is refused rather than resolved, since a file
//!   with two `"width"` entries means its author believed something about it that is not true.

use std::fmt;

/// How many objects and arrays a document may nest before the parser refuses it. `value()` recurses,
/// so this is a guard against a file that is 100 000 `[` bytes long taking the stack with it, not a
/// limit on anything a device profile needs — that one is three deep at its deepest.
pub const MAX_DEPTH: usize = 64;

/// A JSON value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    /// The number **as written**, so `1e3`, `0.5` and `18446744073709551615` all survive a round trip
    /// exactly. [`Value::as_u64`] is what turns it into a machine number, and it refuses a literal
    /// that is not a whole number rather than rounding one.
    Number(String),
    String(String),
    Array(Vec<Value>),
    Object(Map),
}

/// An object, keeping its keys in the order they were inserted.
///
/// Not a `HashMap`: the output is a file whose order is the reading order of the struct behind it,
/// and a re-save that shuffles the keys would make every diff of that file meaningless.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Map(Vec<(String, Value)>);

impl Map {
    pub fn new() -> Self {
        Map(Vec::new())
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Insert at the end, or replace in place if the key is already there. The replaced value is
    /// returned, which is how the parser notices a duplicate.
    pub fn insert(&mut self, key: impl Into<String>, value: Value) -> Option<Value> {
        let key = key.into();
        match self.0.iter_mut().find(|(k, _)| *k == key) {
            Some((_, slot)) => Some(std::mem::replace(slot, value)),
            None => {
                self.0.push((key, value));
                None
            }
        }
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, (String, Value)> {
        self.0.iter()
    }
}

impl FromIterator<(String, Value)> for Map {
    fn from_iter<I: IntoIterator<Item = (String, Value)>>(iter: I) -> Self {
        let mut map = Map::new();
        for (k, v) in iter {
            map.insert(k, v);
        }
        map
    }
}

impl Value {
    /// What this value *is*, in words, for an error that has to say what it found instead.
    pub fn kind(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "a boolean",
            Value::Number(_) => "a number",
            Value::String(_) => "a string",
            Value::Array(_) => "an array",
            Value::Object(_) => "an object",
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&Map> {
        match self {
            Value::Object(map) => Some(map),
            _ => None,
        }
    }

    /// The literal this number was written as, or `None` for anything that is not a number.
    pub fn number(&self) -> Option<&str> {
        match self {
            Value::Number(text) => Some(text),
            _ => None,
        }
    }

    /// A non-negative whole number. `1.0` and `1e3` are *not* one: the first is a decimal and the
    /// second is a float exponent, and a field that means "how many pixels" should be written the way
    /// an integer is written. Refusing them costs one retry and catches a file where someone wrote
    /// `0.5` for a width.
    pub fn as_u64(&self) -> Option<u64> {
        let text = self.number()?;
        if !text.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        text.parse().ok()
    }

    /// A whole number that may be negative.
    pub fn as_i64(&self) -> Option<i64> {
        let text = self.number()?;
        let digits = text.strip_prefix('-').unwrap_or(text);
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        text.parse().ok()
    }

    /// Any number, including `1e3`, as a float. Used by nothing in the device file — it is here
    /// because a value type that cannot answer "and as a float?" invites the next reader to add one.
    pub fn as_f64(&self) -> Option<f64> {
        self.number()?.parse().ok()
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::String(v.to_string())
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

impl From<Vec<Value>> for Value {
    fn from(v: Vec<Value>) -> Self {
        Value::Array(v)
    }
}

impl From<Map> for Value {
    fn from(v: Map) -> Self {
        Value::Object(v)
    }
}

/// The integer constructors. Each one writes decimal digits, which is what [`Value::as_u64`] reads
/// back, so `Value::from(u32::MAX)` and the literal `4294967295` are the same value.
macro_rules! from_uint {
    ($($t:ty),*) => {$(
        impl From<$t> for Value {
            fn from(v: $t) -> Self {
                Value::Number(v.to_string())
            }
        }
    )*};
}

from_uint!(u8, u16, u32, u64, i32, i64);

/// Where a parse failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for Error {}

/// Parse one document. Trailing whitespace is fine; trailing *anything else* is an error, because a
/// file with two documents in it is a file whose second half nobody will notice is being ignored.
pub fn parse(text: &str) -> Result<Value, Error> {
    let mut p = Parser {
        text,
        i: 0,
        line: 1,
        column: 1,
    };
    p.ws();
    let value = p.value(0)?;
    p.ws();
    if p.peek().is_some() {
        return Err(p.error("the document ends here, and more text follows"));
    }
    Ok(value)
}

struct Parser<'a> {
    text: &'a str,
    /// Byte offset. Always on a `char` boundary: every advance goes through [`Parser::bump`].
    i: usize,
    line: usize,
    column: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<char> {
        self.text[self.i..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.i += c.len_utf8();
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }

    fn error(&self, message: impl Into<String>) -> Error {
        Error {
            line: self.line,
            column: self.column,
            message: message.into(),
        }
    }

    fn ws(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
            self.bump();
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, Error> {
        match self.peek() {
            Some('{') => self.object(depth),
            Some('[') => self.array(depth),
            Some('"') => Ok(Value::String(self.string()?)),
            Some('t') => self.literal("true", Value::Bool(true)),
            Some('f') => self.literal("false", Value::Bool(false)),
            Some('n') => self.literal("null", Value::Null),
            Some(c) if c == '-' || c.is_ascii_digit() => self.number(),
            Some(c) => Err(self.error(format!("expected a value, found {c:?}"))),
            None => Err(self.error("expected a value, found the end of the file")),
        }
    }

    /// `true`, `false` and `null`. Consumed character by character so the position in the error is
    /// the character that disagreed, not the start of the word — which means looking before
    /// stepping: `bump` moves the position past what it returns.
    fn literal(&mut self, word: &str, value: Value) -> Result<Value, Error> {
        for want in word.chars() {
            if self.peek() != Some(want) {
                return Err(self.error(format!("expected `{word}`")));
            }
            self.bump();
        }
        Ok(value)
    }

    /// The guard against a file that is 100 000 `[` bytes long. It counts **containers**, not values,
    /// so where the limit falls does not depend on whether the innermost one happens to be empty:
    /// `[` × `MAX_DEPTH` parses, `[` × `MAX_DEPTH + 1` does not.
    fn too_deep(&self, depth: usize) -> Option<Error> {
        (depth >= MAX_DEPTH).then(|| self.error(format!("nested deeper than {MAX_DEPTH} levels")))
    }

    fn object(&mut self, depth: usize) -> Result<Value, Error> {
        if let Some(e) = self.too_deep(depth) {
            return Err(e);
        }
        self.bump();
        let mut map = Map::new();
        self.ws();
        if self.peek() == Some('}') {
            self.bump();
            return Ok(Value::Object(map));
        }
        loop {
            self.ws();
            if self.peek() != Some('"') {
                return Err(self.error("expected a quoted key"));
            }
            let key = self.string()?;
            self.ws();
            if self.peek() != Some(':') {
                return Err(self.error(format!("expected `:` after the key \"{key}\"")));
            }
            self.bump();
            self.ws();
            let value = self.value(depth + 1)?;
            if map.insert(key.clone(), value).is_some() {
                return Err(self.error(format!("the key \"{key}\" is given twice")));
            }
            self.ws();
            match self.peek() {
                Some(',') => {
                    self.bump();
                }
                Some('}') => {
                    self.bump();
                    return Ok(Value::Object(map));
                }
                Some(c) => return Err(self.error(format!("expected `,` or `}}`, found {c:?}"))),
                None => return Err(self.error("the object never closes: expected `}`")),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, Error> {
        if let Some(e) = self.too_deep(depth) {
            return Err(e);
        }
        self.bump();
        let mut items = Vec::new();
        self.ws();
        if self.peek() == Some(']') {
            self.bump();
            return Ok(Value::Array(items));
        }
        loop {
            self.ws();
            items.push(self.value(depth + 1)?);
            self.ws();
            match self.peek() {
                Some(',') => {
                    self.bump();
                }
                Some(']') => {
                    self.bump();
                    return Ok(Value::Array(items));
                }
                Some(c) => return Err(self.error(format!("expected `,` or `]`, found {c:?}"))),
                None => return Err(self.error("the array never closes: expected `]`")),
            }
        }
    }

    fn string(&mut self) -> Result<String, Error> {
        self.bump();
        let mut out = String::new();
        loop {
            // Peek first and step only on the paths that accept the character: an error has to point
            // at what disagreed, and `bump` leaves the position on the next character.
            let Some(c) = self.peek() else {
                return Err(self.error("the string never closes: expected `\"`"));
            };
            match c {
                '"' => {
                    self.bump();
                    return Ok(out);
                }
                '\\' => {
                    self.bump();
                    out.push(self.escape()?);
                }
                c if (c as u32) < 0x20 => {
                    return Err(self.error(format!(
                        "a raw control character (U+{:04X}) is not allowed in a string; write `\\u{:04x}`",
                        c as u32, c as u32
                    )))
                }
                c => {
                    self.bump();
                    out.push(c);
                }
            }
        }
    }

    fn escape(&mut self) -> Result<char, Error> {
        let Some(c) = self.peek() else {
            return Err(self.error("the file ends in a backslash"));
        };
        self.bump();
        Ok(match c {
            '"' => '"',
            '\\' => '\\',
            '/' => '/',
            'b' => '\u{8}',
            'f' => '\u{c}',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'u' => return self.unicode_escape(),
            other => return Err(self.error(format!("`\\{other}` is not a JSON escape"))),
        })
    }

    /// `\uXXXX`, and the surrogate pair a character outside the basic plane is written as.
    fn unicode_escape(&mut self) -> Result<char, Error> {
        let first = self.hex4()?;
        if (0xd800..0xdc00).contains(&first) {
            for want in ['\\', 'u'] {
                if self.peek() != Some(want) {
                    return Err(self.error(
                        "a high surrogate has to be followed by `\\u` and a low surrogate",
                    ));
                }
                self.bump();
            }
            let second = self.hex4()?;
            if !(0xdc00..0xe000).contains(&second) {
                return Err(self.error("a high surrogate has to be followed by a low surrogate"));
            }
            let point = 0x10000 + ((first - 0xd800) << 10) + (second - 0xdc00);
            return char::from_u32(point)
                .ok_or_else(|| self.error("that surrogate pair is not a character"));
        }
        if (0xdc00..0xe000).contains(&first) {
            return Err(self.error("a low surrogate on its own is not a character"));
        }
        char::from_u32(first).ok_or_else(|| self.error(format!("U+{first:04X} is not a character")))
    }

    fn hex4(&mut self) -> Result<u32, Error> {
        let mut value = 0u32;
        for _ in 0..4 {
            let Some(c) = self.peek() else {
                return Err(self.error("`\\u` needs four hexadecimal digits"));
            };
            let Some(digit) = c.to_digit(16) else {
                return Err(self.error(format!("`{c}` is not a hexadecimal digit")));
            };
            self.bump();
            value = value * 16 + digit;
        }
        Ok(value)
    }

    fn number(&mut self) -> Result<Value, Error> {
        let start = self.i;
        if self.peek() == Some('-') {
            self.bump();
        }
        match self.peek() {
            Some('0') => {
                self.bump();
                // `01` is not a number: a leading zero means the literal ends there.
                if matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                    return Err(self.error("a number may not have a leading zero"));
                }
            }
            Some(c) if c.is_ascii_digit() => {
                while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                    self.bump();
                }
            }
            _ => return Err(self.error("expected a digit")),
        }
        if self.peek() == Some('.') {
            self.bump();
            if !matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                return Err(self.error("a `.` needs at least one digit after it"));
            }
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.bump();
            }
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            self.bump();
            if matches!(self.peek(), Some('+' | '-')) {
                self.bump();
            }
            if !matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                return Err(self.error("an exponent needs at least one digit"));
            }
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.bump();
            }
        }
        Ok(Value::Number(self.text[start..self.i].to_string()))
    }
}

/// Write a document: two-space indent, one key per line, keys in insertion order, and a final
/// newline so the file is a text file rather than a blob that a diff tool treats as one line.
pub fn write(value: &Value) -> String {
    let mut out = String::new();
    write_value(&mut out, value, 0);
    out.push('\n');
    out
}

fn write_value(out: &mut String, value: &Value, depth: usize) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(text) => out.push_str(text),
        Value::String(text) => write_string(out, text),
        Value::Array(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push_str("[\n");
            for (i, item) in items.iter().enumerate() {
                indent(out, depth + 1);
                write_value(out, item, depth + 1);
                if i + 1 < items.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            indent(out, depth);
            out.push(']');
        }
        Value::Object(map) => {
            if map.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push_str("{\n");
            for (i, (key, item)) in map.iter().enumerate() {
                indent(out, depth + 1);
                write_string(out, key);
                out.push_str(": ");
                write_value(out, item, depth + 1);
                if i + 1 < map.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            indent(out, depth);
            out.push('}');
        }
    }
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

/// Quote and escape. `/` is left alone — escaping it is legal and unreadable — and everything above
/// U+001F is written as itself, because the file is UTF-8 and a device name is allowed to be.
fn write_string(out: &mut String, text: &str) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
#[path = "test/json.rs"]
mod tests;
