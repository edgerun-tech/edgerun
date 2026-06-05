//! Fixed OAuth JSON field projection.
//!
//! This is owner-local glue until the Rust host has a direct invocation surface
//! for `json-tape.wat`, `json-scalar.wat`, and `json-emit.wat`.

use crate::prelude::*;
use core::fmt::Write;

#[derive(Debug, Clone)]
pub struct JsonObject<'a> {
    fields: Vec<JsonField<'a>>,
}

#[derive(Debug, Clone)]
struct JsonField<'a> {
    key: String,
    value: JsonAtom<'a>,
}

#[derive(Debug, Clone)]
enum JsonAtom<'a> {
    String(String),
    U64(u64),
    Bool(bool),
    Array(&'a str),
    Object(&'a str),
    Null,
    Other,
}

pub fn parse_object(input: &str) -> Result<JsonObject<'_>, String> {
    let bytes = input.as_bytes();
    let mut pos = skip_ws(bytes, 0);
    if bytes.get(pos) != Some(&b'{') {
        return Err("JSON parse: expected object".into());
    }
    pos += 1;

    let mut fields = Vec::new();
    loop {
        pos = skip_ws(bytes, pos);
        match bytes.get(pos) {
            Some(b'}') => {
                pos += 1;
                break;
            }
            Some(b'"') => {}
            Some(_) => return Err("JSON parse: expected object key".into()),
            None => return Err("JSON parse: unterminated object".into()),
        }

        let (key, next) = parse_string(bytes, pos)?;
        pos = skip_ws(bytes, next);
        if bytes.get(pos) != Some(&b':') {
            return Err("JSON parse: expected ':'".into());
        }
        pos = skip_ws(bytes, pos + 1);
        let (value, next) = parse_value(input, bytes, pos)?;
        fields.push(JsonField { key, value });
        pos = skip_ws(bytes, next);
        match bytes.get(pos) {
            Some(b',') => pos += 1,
            Some(b'}') => {
                pos += 1;
                break;
            }
            Some(_) => return Err("JSON parse: expected ',' or '}'".into()),
            None => return Err("JSON parse: unterminated object".into()),
        }
    }

    if skip_ws(bytes, pos) != bytes.len() {
        return Err("JSON parse: trailing data".into());
    }
    Ok(JsonObject { fields })
}

impl<'a> JsonObject<'a> {
    pub fn str_field(&self, key: &str) -> Option<String> {
        self.find(key).and_then(|value| match value {
            JsonAtom::String(value) => Some(value.clone()),
            _ => None,
        })
    }

    pub fn u64_field(&self, key: &str) -> Option<u64> {
        self.find(key).and_then(|value| match value {
            JsonAtom::U64(value) => Some(*value),
            _ => None,
        })
    }

    pub fn bool_field(&self, key: &str) -> Option<bool> {
        self.find(key).and_then(|value| match value {
            JsonAtom::Bool(value) => Some(*value),
            _ => None,
        })
    }

    pub fn string_array_field(&self, key: &str) -> Vec<String> {
        self.find(key)
            .and_then(|value| match value {
                JsonAtom::Array(span) => Some(parse_string_array(span).unwrap_or_default()),
                _ => None,
            })
            .unwrap_or_default()
    }

    pub fn string_or_array_field(&self, key: &str) -> Vec<String> {
        match self.find(key) {
            Some(JsonAtom::String(value)) => vec![value.clone()],
            Some(JsonAtom::Array(span)) => parse_string_array(span).unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    pub fn object_array_field(&self, key: &str) -> Vec<&'a str> {
        self.find(key)
            .and_then(|value| match value {
                JsonAtom::Array(span) => Some(parse_object_array(span).unwrap_or_default()),
                _ => None,
            })
            .unwrap_or_default()
    }

    fn find(&self, key: &str) -> Option<&JsonAtom<'a>> {
        self.fields
            .iter()
            .find(|field| field.key == key)
            .map(|field| &field.value)
    }
}

pub fn write_string_field(out: &mut String, first: &mut bool, key: &str, value: &str) {
    write_sep(out, first);
    write_json_string(out, key);
    out.push(':');
    write_json_string(out, value);
}

pub fn write_u64_field(out: &mut String, first: &mut bool, key: &str, value: u64) {
    write_sep(out, first);
    write_json_string(out, key);
    out.push(':');
    let _ = write!(out, "{value}");
}

pub fn write_string_array_field(out: &mut String, first: &mut bool, key: &str, values: &[String]) {
    write_sep(out, first);
    write_json_string(out, key);
    out.push_str(":[");
    for (idx, value) in values.iter().enumerate() {
        if idx != 0 {
            out.push(',');
        }
        write_json_string(out, value);
    }
    out.push(']');
}

pub fn write_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c <= '\u{1f}' => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn write_sep(out: &mut String, first: &mut bool) {
    if *first {
        *first = false;
    } else {
        out.push(',');
    }
}

fn parse_value<'a>(
    input: &'a str,
    bytes: &'a [u8],
    pos: usize,
) -> Result<(JsonAtom<'a>, usize), String> {
    match bytes.get(pos) {
        Some(b'"') => {
            let (value, next) = parse_string(bytes, pos)?;
            Ok((JsonAtom::String(value), next))
        }
        Some(b'0'..=b'9') => {
            parse_u64(bytes, pos).map(|(value, next)| (JsonAtom::U64(value), next))
        }
        Some(b't') if input[pos..].starts_with("true") => Ok((JsonAtom::Bool(true), pos + 4)),
        Some(b'f') if input[pos..].starts_with("false") => Ok((JsonAtom::Bool(false), pos + 5)),
        Some(b'n') if input[pos..].starts_with("null") => Ok((JsonAtom::Null, pos + 4)),
        Some(b'[') => {
            let next = skip_balanced(bytes, pos, b'[', b']')?;
            Ok((JsonAtom::Array(&input[pos..next]), next))
        }
        Some(b'{') => {
            let next = skip_balanced(bytes, pos, b'{', b'}')?;
            Ok((JsonAtom::Object(&input[pos..next]), next))
        }
        Some(_) => {
            let next = skip_atom(bytes, pos);
            Ok((JsonAtom::Other, next))
        }
        None => Err("JSON parse: expected value".into()),
    }
}

fn parse_string_array(input: &str) -> Result<Vec<String>, String> {
    let bytes = input.as_bytes();
    let mut pos = skip_ws(bytes, 0);
    if bytes.get(pos) != Some(&b'[') {
        return Err("JSON parse: expected string array".into());
    }
    pos += 1;
    let mut values = Vec::new();
    loop {
        pos = skip_ws(bytes, pos);
        match bytes.get(pos) {
            Some(b']') => return Ok(values),
            Some(b'"') => {
                let (value, next) = parse_string(bytes, pos)?;
                values.push(value);
                pos = skip_ws(bytes, next);
                match bytes.get(pos) {
                    Some(b',') => pos += 1,
                    Some(b']') => return Ok(values),
                    _ => return Err("JSON parse: expected array separator".into()),
                }
            }
            _ => return Err("JSON parse: expected string item".into()),
        }
    }
}

fn parse_object_array(input: &str) -> Result<Vec<&str>, String> {
    let bytes = input.as_bytes();
    let mut pos = skip_ws(bytes, 0);
    if bytes.get(pos) != Some(&b'[') {
        return Err("JSON parse: expected object array".into());
    }
    pos += 1;
    let mut values = Vec::new();
    loop {
        pos = skip_ws(bytes, pos);
        match bytes.get(pos) {
            Some(b']') => return Ok(values),
            Some(b'{') => {
                let next = skip_balanced(bytes, pos, b'{', b'}')?;
                values.push(&input[pos..next]);
                pos = skip_ws(bytes, next);
                match bytes.get(pos) {
                    Some(b',') => pos += 1,
                    Some(b']') => return Ok(values),
                    _ => return Err("JSON parse: expected array separator".into()),
                }
            }
            _ => return Err("JSON parse: expected object item".into()),
        }
    }
}

fn parse_string(bytes: &[u8], mut pos: usize) -> Result<(String, usize), String> {
    if bytes.get(pos) != Some(&b'"') {
        return Err("JSON parse: expected string".into());
    }
    pos += 1;
    let mut segment_start = pos;
    let mut out = String::new();
    while let Some(&byte) = bytes.get(pos) {
        match byte {
            b'"' => {
                push_utf8(&mut out, &bytes[segment_start..pos])?;
                return Ok((out, pos + 1));
            }
            b'\\' => {
                push_utf8(&mut out, &bytes[segment_start..pos])?;
                pos += 1;
                let escaped = *bytes
                    .get(pos)
                    .ok_or_else(|| "JSON parse: unterminated escape".to_string())?;
                match escaped {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{0008}'),
                    b'f' => out.push('\u{000c}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let code = parse_hex4(bytes, pos + 1)?;
                        if let Some(ch) = char::from_u32(code) {
                            out.push(ch);
                        }
                        pos += 4;
                    }
                    _ => return Err("JSON parse: invalid escape".into()),
                }
                segment_start = pos + 1;
            }
            b if b < 0x20 => return Err("JSON parse: control byte in string".into()),
            _ => {}
        }
        pos += 1;
    }
    Err("JSON parse: unterminated string".into())
}

fn push_utf8(out: &mut String, bytes: &[u8]) -> Result<(), String> {
    let value = core::str::from_utf8(bytes).map_err(|_| "JSON parse: invalid UTF-8".to_string())?;
    out.push_str(value);
    Ok(())
}

fn parse_u64(bytes: &[u8], mut pos: usize) -> Result<(u64, usize), String> {
    let mut value = 0u64;
    let start = pos;
    while let Some(b'0'..=b'9') = bytes.get(pos) {
        value = value
            .checked_mul(10)
            .and_then(|v| v.checked_add((bytes[pos] - b'0') as u64))
            .ok_or_else(|| "JSON parse: integer overflow".to_string())?;
        pos += 1;
    }
    if pos == start {
        return Err("JSON parse: expected integer".into());
    }
    Ok((value, pos))
}

fn parse_hex4(bytes: &[u8], pos: usize) -> Result<u32, String> {
    let mut value = 0u32;
    for offset in 0..4 {
        let byte = *bytes
            .get(pos + offset)
            .ok_or_else(|| "JSON parse: short unicode escape".to_string())?;
        value = (value << 4)
            | match byte {
                b'0'..=b'9' => (byte - b'0') as u32,
                b'a'..=b'f' => (byte - b'a' + 10) as u32,
                b'A'..=b'F' => (byte - b'A' + 10) as u32,
                _ => return Err("JSON parse: invalid unicode escape".into()),
            };
    }
    Ok(value)
}

fn skip_balanced(bytes: &[u8], mut pos: usize, open: u8, close: u8) -> Result<usize, String> {
    if bytes.get(pos) != Some(&open) {
        return Err("JSON parse: expected container".into());
    }
    let mut depth = 0usize;
    while let Some(&byte) = bytes.get(pos) {
        match byte {
            b'"' => pos = skip_string(bytes, pos)?,
            b if b == open => {
                depth += 1;
                pos += 1;
            }
            b if b == close => {
                depth -= 1;
                pos += 1;
                if depth == 0 {
                    return Ok(pos);
                }
            }
            _ => pos += 1,
        }
    }
    Err("JSON parse: unterminated container".into())
}

fn skip_string(bytes: &[u8], mut pos: usize) -> Result<usize, String> {
    pos += 1;
    while let Some(&byte) = bytes.get(pos) {
        match byte {
            b'"' => return Ok(pos + 1),
            b'\\' => pos += 2,
            _ => pos += 1,
        }
    }
    Err("JSON parse: unterminated string".into())
}

fn skip_atom(bytes: &[u8], mut pos: usize) -> usize {
    while let Some(byte) = bytes.get(pos) {
        if matches!(byte, b',' | b'}' | b']' | b' ' | b'\n' | b'\r' | b'\t') {
            break;
        }
        pos += 1;
    }
    pos
}

fn skip_ws(bytes: &[u8], mut pos: usize) -> usize {
    while matches!(bytes.get(pos), Some(b' ' | b'\n' | b'\r' | b'\t')) {
        pos += 1;
    }
    pos
}
