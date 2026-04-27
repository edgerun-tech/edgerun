//! TOML API compatible with toml crate.
//!
//! This module provides drop-in replacements for toml functionality.

#[cfg(not(feature = "std"))]
use alloc::borrow::ToOwned;
#[cfg(not(feature = "std"))]
use alloc::string::ToString;
#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec;
#[cfg(not(feature = "std"))]
use alloc::{format, string::String};
#[cfg(feature = "std")]
use std::{format, string::String};

use crate::{JsonValue, Map, Number};

#[derive(Debug, Clone)]
pub enum TomlValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Datetime(String),
    Array(Vec<TomlValue>),
    Table(Vec<(String, TomlValue)>),
}

impl TomlValue {
    pub fn is_string(&self) -> bool {
        matches!(self, TomlValue::String(_))
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, TomlValue::Integer(_))
    }

    pub fn is_float(&self) -> bool {
        matches!(self, TomlValue::Float(_))
    }

    pub fn is_boolean(&self) -> bool {
        matches!(self, TomlValue::Boolean(_))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, TomlValue::Array(_))
    }

    pub fn is_table(&self) -> bool {
        matches!(self, TomlValue::Table(_))
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            TomlValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            TomlValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            TomlValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            TomlValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<TomlValue>> {
        match self {
            TomlValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn as_table(&self) -> Option<&Vec<(String, TomlValue)>> {
        match self {
            TomlValue::Table(t) => Some(t),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&TomlValue> {
        self.as_table()?
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    pub fn get_index(&self, index: usize) -> Option<&TomlValue> {
        self.as_array()?.get(index)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TomlError {
    UnexpectedChar(char),
    UnexpectedEnd,
    InvalidKey,
    DuplicateKey,
    InvalidValue,
    IoError(String),
}

impl core::fmt::Display for TomlError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TomlError::UnexpectedChar(c) => write!(f, "unexpected character: {}", c),
            TomlError::UnexpectedEnd => write!(f, "unexpected end of input"),
            TomlError::InvalidKey => write!(f, "invalid key"),
            TomlError::DuplicateKey => write!(f, "duplicate key"),
            TomlError::InvalidValue => write!(f, "invalid value"),
            TomlError::IoError(s) => write!(f, "IO error: {}", s),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TomlError {}

pub fn from_toml_str(s: &str) -> Result<TomlValue, TomlError> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(TomlValue::Table(Vec::new()));
    }
    parse_toml_value(trimmed)
}

pub fn parse_toml_value(s: &str) -> Result<TomlValue, TomlError> {
    let mut result: Vec<(String, TomlValue)> = Vec::new();
    let mut current_table: Option<String> = None;
    let mut current_section: Vec<(String, TomlValue)> = Vec::new();

    for line in s.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') {
            if let Some(table_name) = current_table.take() {
                if !current_section.is_empty() {
                    result.push((table_name, TomlValue::Table(current_section.clone())));
                    current_section.clear();
                }
            }

            let closing = line.rfind(']').unwrap_or(line.len() - 1);
            let table_name = &line[1..closing];
            if table_name.starts_with('[') {
                let array_name = table_name.trim_end_matches(']');
                current_table = Some(array_name.to_string());
            } else {
                current_table = Some(table_name.to_string());
            }
            continue;
        }

        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim();
            let value_str = line[eq_pos + 1..].trim();

            let value = parse_toml_simple(value_str)?;

            if let Some(table) = current_section.iter_mut().find(|(k, _)| k == key) {
                table.1 = value;
            } else {
                current_section.push((key.to_string(), value));
            }
        }
    }

    if let Some(table_name) = current_table {
        if !current_section.is_empty() {
            result.push((table_name, TomlValue::Table(current_section.clone())));
        }
    } else if !current_section.is_empty() {
        for (k, v) in current_section {
            result.push((k, v));
        }
    }

    Ok(TomlValue::Table(result))
}

fn parse_toml_simple(s: &str) -> Result<TomlValue, TomlError> {
    let s = s.trim();

    if s.starts_with('"') && s.matches('"').count() >= 3 {
        let inner = &s[1..s.len() - 1];
        return Ok(TomlValue::String(unescape_toml_string(inner)));
    }

    if s.starts_with('\'') && s.matches('\'').count() >= 2 {
        let inner = &s[1..s.len() - 1];
        return Ok(TomlValue::String(inner.to_string()));
    }

    if s.starts_with('[') && s.ends_with(']') {
        let inner = &s[1..s.len() - 1];
        let items: Vec<&str> = inner.split(',').collect();
        let mut arr = Vec::new();
        for item in items {
            arr.push(parse_toml_simple(item.trim())?);
        }
        return Ok(TomlValue::Array(arr));
    }

    if s == "true" {
        return Ok(TomlValue::Boolean(true));
    }
    if s == "false" {
        return Ok(TomlValue::Boolean(false));
    }

    if let Ok(i) = s.parse::<i64>() {
        return Ok(TomlValue::Integer(i));
    }
    if let Ok(f) = s.parse::<f64>() {
        return Ok(TomlValue::Float(f));
    }

    Ok(TomlValue::String(s.to_owned()))
}

fn unescape_toml_string(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some('"') => result.push('"'),
                Some('\\') => result.push('\\'),
                Some(c) => {
                    result.push('\\');
                    result.push(c);
                }
                None => {}
            }
        } else {
            result.push(c);
        }
    }

    result
}

pub fn to_toml_string(value: &TomlValue) -> Result<String, TomlError> {
    let mut output = String::new();
    to_toml_value(&mut output, value, 0)?;
    Ok(output)
}

fn to_toml_value(
    output: &mut String,
    value: &TomlValue,
    indent: usize,
) -> Result<(), TomlError> {
    match value {
        TomlValue::String(s) => {
            if s.contains('"') || s.contains('\n') || s.contains('\\') || s.contains('#') {
                output.push('"');
                for c in s.chars() {
                    match c {
                        '"' => output.push_str("\\\""),
                        '\\' => output.push_str("\\\\"),
                        '\n' => output.push_str("\\n"),
                        '\r' => output.push_str("\\r"),
                        '\t' => output.push_str("\\t"),
                        c => output.push(c),
                    }
                }
                output.push('"');
            } else {
                output.push('"');
                output.push_str(s);
                output.push('"');
            }
        }
        TomlValue::Integer(i) => {
            output.push_str(&i.to_string());
        }
        TomlValue::Float(f) => {
            output.push_str(&f.to_string());
        }
        TomlValue::Boolean(b) => {
            output.push_str(if *b { "true" } else { "false" });
        }
        TomlValue::Datetime(s) => {
            output.push_str(s);
        }
        TomlValue::Array(arr) => {
            output.push('[');
            for (i, v) in arr.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                to_toml_value(output, v, indent)?;
            }
            output.push(']');
        }
        TomlValue::Table(table) => {
            for (i, (k, v)) in table.iter().enumerate() {
                if i > 0 {
                    output.push('\n');
                }
                match v {
                    TomlValue::Table(_) | TomlValue::Array(_) => {
                        output.push_str(&format!("[{}]\n", k));
                        to_toml_value(output, v, indent + 1)?;
                    }
                    _ => {
                        output.push_str(k);
                        output.push_str(" = ");
                        to_toml_value(output, v, indent)?;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Convert JsonValue to TomlValue
pub fn json_to_toml(value: JsonValue) -> TomlValue {
    match value {
        JsonValue::Null => TomlValue::String("null".to_string()),
        JsonValue::Bool(b) => TomlValue::Boolean(b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                TomlValue::Integer(i)
            } else if let Some(f) = n.as_f64() {
                TomlValue::Float(f)
            } else {
                TomlValue::String(n.to_string())
            }
        }
        JsonValue::String(s) => TomlValue::String(s),
        JsonValue::Array(arr) => TomlValue::Array(arr.into_iter().map(json_to_toml).collect()),
        JsonValue::Object(map) => {
            let mut table: Vec<(String, TomlValue)> = Vec::new();
            for (k, v) in map.0 {
                table.push((k, json_to_toml(v)));
            }
            TomlValue::Table(table)
        }
    }
}

/// Convert TomlValue to JsonValue
pub fn toml_to_json(value: TomlValue) -> JsonValue {
    match value {
        TomlValue::String(s) => JsonValue::String(s),
        TomlValue::Integer(i) => JsonValue::Number(Number::I64(i)),
        TomlValue::Float(ref f) => {
            if let Some(n) = Number::from_f64(*f) {
                JsonValue::Number(n)
            } else {
                JsonValue::Null
            }
        }
        TomlValue::Boolean(b) => JsonValue::Bool(b),
        TomlValue::Datetime(s) => JsonValue::String(s),
        TomlValue::Array(arr) => JsonValue::Array(arr.into_iter().map(toml_to_json).collect()),
        TomlValue::Table(table) => {
            let mut obj = Map::new();
            for (k, v) in table {
                obj.0.push((k, toml_to_json(v)));
            }
            JsonValue::Object(obj)
        }
    }
}

#[cfg(feature = "serde")]
pub fn to_value<T>(value: T) -> Result<TomlValue, crate::serde_error::Error>
where
    T: serde_crate::Serialize,
{
    let json_value = crate::to_value(value)?;
    Ok(json_to_toml(json_value))
}

#[cfg(feature = "serde")]
pub fn from_value<T>(value: TomlValue) -> Result<T, crate::serde_error::Error>
where
    T: serde_crate::de::DeserializeOwned,
{
    let json_value = toml_to_json(value);
    crate::from_value(json_value)
}

#[cfg(feature = "serde")]
pub fn from_toml_str_typed<T>(s: &str) -> Result<T, TomlError>
where
    T: serde_crate::de::DeserializeOwned,
{
    let toml = from_toml_str(s)?;
    let json = toml_to_json(toml);
    crate::from_value(json).map_err(|e| TomlError::IoError(e.to_string()))
}

#[cfg(feature = "serde")]
pub fn to_toml_string_typed<T>(value: &T) -> Result<String, TomlError>
where
    T: serde_crate::Serialize,
{
    let json_value = crate::to_value(value).map_err(|e| TomlError::IoError(e.to_string()))?;
    let toml_value = json_to_toml(json_value);
    to_toml_string(&toml_value)
}

#[cfg(feature = "serde")]
impl serde_crate::Serialize for TomlValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_crate::Serializer,
    {
        match self {
            TomlValue::String(s) => serializer.serialize_str(s),
            TomlValue::Integer(i) => serializer.serialize_i64(*i),
            TomlValue::Float(f) => serializer.serialize_f64(*f),
            TomlValue::Boolean(b) => serializer.serialize_bool(*b),
            TomlValue::Datetime(s) => serializer.serialize_str(s),
            TomlValue::Array(arr) => {
                use serde_crate::ser::SerializeSeq;
                let mut seq = serializer.serialize_seq(Some(arr.len()))?;
                for item in arr {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            TomlValue::Table(table) => {
                use serde_crate::ser::SerializeMap;
                let mut m = serializer.serialize_map(Some(table.len()))?;
                for (k, v) in table {
                    m.serialize_key(k)?;
                    m.serialize_value(v)?;
                }
                m.end()
            }
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde_crate::Deserialize<'de> for TomlValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde_crate::Deserializer<'de>,
    {
        let json = JsonValue::deserialize(deserializer)?;
        Ok(json_to_toml(json))
    }
}
