//! YAML API compatible with serde_yaml.
//!
//! This module provides drop-in replacements for serde_yaml functionality.

#[cfg(not(feature = "std"))]
use alloc::borrow::ToOwned;
#[cfg(not(feature = "std"))]
use alloc::string::ToString;
#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec;
#[cfg(not(feature = "std"))]
use alloc::{format, string::String, vec};
#[cfg(feature = "std")]
use std::{format, string::String, vec};

use crate::{JsonValue, Map, Number};

#[derive(Debug, Clone)]
pub enum YamlValue {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<YamlValue>),
    Mapping(Vec<(String, YamlValue)>),
    Tagged(TaggedYamlValue),
}

impl PartialEq for YamlValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (YamlValue::Null, YamlValue::Null) => true,
            (YamlValue::Bool(a), YamlValue::Bool(b)) => a == b,
            (YamlValue::Number(a), YamlValue::Number(b)) => a == b,
            (YamlValue::String(a), YamlValue::String(b)) => a == b,
            (YamlValue::Array(a), YamlValue::Array(b)) => a == b,
            (YamlValue::Mapping(a), YamlValue::Mapping(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                for (k, v) in a.iter() {
                    let found = b.iter().find(|(k2, _)| k == k2);
                    match found {
                        Some((_, v2)) if v == v2 => {}
                        _ => return false,
                    }
                }
                true
            }
            (YamlValue::Tagged(a), YamlValue::Tagged(b)) => a == b,
            _ => false,
        }
    }
}

impl core::ops::Index<usize> for YamlValue {
    type Output = YamlValue;
    fn index(&self, index: usize) -> &YamlValue {
        self.as_sequence()
            .and_then(|arr| arr.get(index))
            .unwrap_or(&YamlValue::Null)
    }
}

impl core::ops::Index<&str> for YamlValue {
    type Output = YamlValue;
    fn index(&self, key: &str) -> &YamlValue {
        self.get(key).unwrap_or(&YamlValue::Null)
    }
}

pub struct TaggedYamlValue {
    pub tag: String,
    pub value: Box<YamlValue>,
}

impl Clone for TaggedYamlValue {
    fn clone(&self) -> Self {
        TaggedYamlValue {
            tag: self.tag.clone(),
            value: self.value.clone(),
        }
    }
}

impl PartialEq for TaggedYamlValue {
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag && self.value == other.value
    }
}

impl core::fmt::Debug for TaggedYamlValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TaggedYamlValue")
            .field("tag", &self.tag)
            .field("value", &self.value)
            .finish()
    }
}

impl YamlValue {
    pub fn is_null(&self) -> bool {
        matches!(self, YamlValue::Null)
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, YamlValue::Bool(_))
    }

    pub fn is_number(&self) -> bool {
        matches!(self, YamlValue::Number(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, YamlValue::String(_))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, YamlValue::Array(_))
    }

    pub fn is_mapping(&self) -> bool {
        matches!(self, YamlValue::Mapping(_))
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            YamlValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            YamlValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            YamlValue::Number(n) => n.as_i64(),
            _ => None,
        }
    }

    pub fn as_sequence(&self) -> Option<&Vec<YamlValue>> {
        match self {
            YamlValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn as_mapping(&self) -> Option<&Vec<(String, YamlValue)>> {
        match self {
            YamlValue::Mapping(m) => Some(m),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&YamlValue> {
        let map = self.as_mapping()?;
        for (k, v) in map.iter() {
            if k == key {
                return Some(v);
            }
        }
        None
    }

    pub fn get_index(&self, index: usize) -> Option<&YamlValue> {
        self.as_sequence()?.get(index)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum YamlError {
    UnexpectedChar(char),
    UnexpectedEnd,
    InvalidIndent,
    DuplicateKey,
    IoError(String),
}

impl core::fmt::Display for YamlError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            YamlError::UnexpectedChar(c) => write!(f, "unexpected character: {}", c),
            YamlError::UnexpectedEnd => write!(f, "unexpected end of input"),
            YamlError::InvalidIndent => write!(f, "invalid indentation"),
            YamlError::DuplicateKey => write!(f, "duplicate key"),
            YamlError::IoError(s) => write!(f, "IO error: {}", s),
        }
    }
}

pub fn from_yaml_str(s: &str) -> Result<YamlValue, YamlError> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(YamlValue::Null);
    }
    parse_yaml_value(trimmed)
}

pub fn parse_yaml_value(s: &str) -> Result<YamlValue, YamlError> {
    let lines: Vec<&str> = s.lines().collect();
    parse_yaml_at(&lines, 0, 0).map(|(v, _)| v)
}

fn parse_yaml_at(
    lines: &[&str],
    start: usize,
    min_indent: usize,
) -> Result<(YamlValue, usize), YamlError> {
    let mut values = Vec::new();
    let mut i = start;
    let mut in_list = false;
    let mut seen_list_key: Option<String> = None;

    while i < lines.len() {
        let line = lines[i];
        let leading = line.len() - line.trim_start().len();

        if line.trim().is_empty() {
            i += 1;
            continue;
        }

        if leading < min_indent && i > start {
            break;
        }

        let trimmed = line.trim();

        if trimmed.starts_with('-') {
            in_list = true;
            let item = trimmed.trim_start_matches('-').trim();
            // Parse list item - could be simple value or nested object
            let item_value = if item.contains(':') {
                // Multi-line nested object - parse as mapping directly
                parse_yaml_at(&[item], 0, 0)
                    .map(|(v, _)| v)
                    .unwrap_or(YamlValue::Null)
            } else {
                parse_yaml_simple(item).unwrap_or(YamlValue::Null)
            };
            values.push(item_value);
            i += 1;
            continue;
        }

        if let Some(colon_pos) = trimmed.find(':') {
            let mut key = trimmed[..colon_pos].trim().to_string();
            // Strip quotes from keys
            if (key.starts_with('"') && key.ends_with('"'))
                || (key.starts_with('\'') && key.ends_with('\''))
            {
                key = key[1..key.len() - 1].to_string();
            }
            let value_str = trimmed[colon_pos + 1..].trim();

            if value_str.is_empty() {
                // Nested block - parse lines at higher indentation
                i += 1;
                let (nested, new_i) = parse_yaml_at(lines, i, leading + 2)?;
                values.push(YamlValue::Mapping(vec![(key, nested)]));
                i = new_i;
            } else {
                let value = parse_yaml_simple(value_str)?;
                values.push(YamlValue::Mapping(vec![(key, value)]));
                i += 1;
            }
        } else {
            return Ok((parse_yaml_simple(trimmed)?, i + 1));
        }
    }

    // Build the result - handle arrays and mappings properly
    let mut result_map: Vec<(String, YamlValue)> = Vec::new();
    let mut array_items: Vec<YamlValue> = Vec::new();

    for v in &values {
        match v {
            YamlValue::Mapping(m) => {
                // Check if this mapping came from a list item (single key like "issuer")
                if in_list && m.len() == 1 {
                    if let Some((k, _)) = m.first() {
                        // Single-field mapping from list item - treat as array element
                        array_items.push(v.clone());
                        continue;
                    }
                }
                // Add mapping entries
                result_map.extend(m.iter().cloned());
            }
            YamlValue::Array(arr) => {
                array_items.extend(arr.iter().cloned());
            }
            _ => {
                array_items.push(v.clone());
            }
        }
    }

    if in_list && array_items.is_empty() && !result_map.is_empty() {
        // List produced mappings - convert to array of maps
        Ok((YamlValue::Array(vec![YamlValue::Mapping(result_map)]), i))
    } else if result_map.is_empty() && !array_items.is_empty() {
        Ok((YamlValue::Array(array_items), i))
    } else {
        Ok((YamlValue::Mapping(result_map), i))
    }
}

fn parse_yaml_item(item: &str) -> Result<YamlValue, YamlError> {
    let trimmed = item.trim();

    if trimmed.is_empty() {
        return Ok(YamlValue::Null);
    }

    if trimmed.starts_with('-') {
        let inner = trimmed.trim_start_matches('-').trim();
        let (value, _) = parse_yaml_at(&[inner], 0, 0)?;
        Ok(value)
    } else if let Some(colon_pos) = trimmed.find(':') {
        let key = trimmed[..colon_pos].trim();
        let value_str = trimmed[colon_pos + 1..].trim();

        if value_str.is_empty() {
            let (nested, _) = parse_yaml_at(&[trimmed], 0, 2)?;
            Ok(YamlValue::Mapping(vec![(key.to_owned(), nested)]))
        } else {
            let value = parse_yaml_simple(value_str)?;
            Ok(YamlValue::Mapping(vec![(key.to_owned(), value)]))
        }
    } else {
        parse_yaml_simple(trimmed)
    }
}

fn parse_yaml_simple(s: &str) -> Result<YamlValue, YamlError> {
    let s = s.trim();

    if s == "null" || s == "~" || s.is_empty() {
        return Ok(YamlValue::Null);
    }

    if s == "true" || s == "yes" || s == "on" {
        return Ok(YamlValue::Bool(true));
    }
    if s == "false" || s == "no" || s == "off" {
        return Ok(YamlValue::Bool(false));
    }

    if let Ok(i) = s.parse::<i64>() {
        return Ok(YamlValue::Number(Number::I64(i)));
    }
    if let Ok(u) = s.parse::<u64>() {
        return Ok(YamlValue::Number(Number::U64(u)));
    }
    if let Ok(f) = s.parse::<f64>() {
        if let Some(n) = Number::from_f64(f) {
            return Ok(YamlValue::Number(n));
        }
    }

    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        let inner = &s[1..s.len() - 1];
        return Ok(YamlValue::String(unescape_yaml_string(inner)));
    }

    Ok(YamlValue::String(s.to_owned()))
}

fn unescape_yaml_string(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some('"') => result.push('"'),
                Some('\'') => result.push('\''),
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

pub fn to_yaml_string(value: &YamlValue) -> Result<String, YamlError> {
    let mut output = String::new();
    to_yaml_string_impl(&mut output, value, 0)?;
    Ok(output)
}

fn to_yaml_string_impl(
    output: &mut String,
    value: &YamlValue,
    indent: usize,
) -> Result<(), YamlError> {
    let indent_str = "  ".repeat(indent);
    match value {
        YamlValue::Null => {
            output.push_str("null");
        }
        YamlValue::Bool(b) => {
            output.push_str(if *b { "true" } else { "false" });
        }
        YamlValue::Number(n) => {
            output.push_str(&n.to_string());
        }
        YamlValue::String(s) => {
            if s.contains(':')
                || s.contains('#')
                || s.starts_with(' ')
                || s.ends_with(' ')
                || s.contains('\n')
            {
                output.push('"');
                for c in s.chars() {
                    match c {
                        '"' => output.push_str("\\\""),
                        '\n' => output.push_str("\\n"),
                        '\r' => output.push_str("\\r"),
                        '\t' => output.push_str("\\t"),
                        c => output.push(c),
                    }
                }
                output.push('"');
            } else {
                output.push_str(s);
            }
        }
        YamlValue::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                if i > 0 {
                    output.push('\n');
                }
                output.push_str(&indent_str);
                output.push_str("- ");
                to_yaml_string_impl(output, v, indent + 1)?;
            }
        }
        YamlValue::Mapping(map) => {
            for (i, (k, v)) in map.iter().enumerate() {
                if i > 0 {
                    output.push('\n');
                }
                output.push_str(&indent_str);
                match v {
                    YamlValue::Mapping(_) | YamlValue::Array(_) => {
                        output.push_str(k);
                        output.push_str(":\n");
                        to_yaml_string_impl(output, v, indent + 1)?;
                    }
                    _ => {
                        output.push_str(k);
                        output.push_str(": ");
                        to_yaml_string_impl(output, v, 0)?;
                    }
                }
            }
        }
        YamlValue::Tagged(tagged) => {
            output.push_str(&format!("!{} ", tagged.tag));
            to_yaml_string_impl(output, &tagged.value, 0)?;
        }
    }
    Ok(())
}

#[cfg(feature = "serde")]
pub fn to_value<T>(value: T) -> Result<YamlValue, crate::serde_error::Error>
where
    T: serde_crate::Serialize,
{
    let json_value = crate::to_value(value)?;
    Ok(json_to_yaml(json_value))
}

#[cfg(feature = "serde")]
pub fn from_value<T>(value: YamlValue) -> Result<T, crate::serde_error::Error>
where
    T: serde_crate::de::DeserializeOwned,
{
    let json_value = yaml_to_json(value);
    crate::from_value(json_value)
}

/// Convert JsonValue to YamlValue
pub fn json_to_yaml(value: JsonValue) -> YamlValue {
    match value {
        JsonValue::Null => YamlValue::Null,
        JsonValue::Bool(b) => YamlValue::Bool(b),
        JsonValue::Number(n) => YamlValue::Number(n),
        JsonValue::String(s) => YamlValue::String(s),
        JsonValue::Array(arr) => YamlValue::Array(arr.into_iter().map(json_to_yaml).collect()),
        JsonValue::Object(map) => {
            let mut mapping: Vec<(String, YamlValue)> = Vec::new();
            for (k, v) in map.0.iter() {
                mapping.push((k.clone(), json_to_yaml(v.clone())));
            }
            YamlValue::Mapping(mapping)
        }
    }
}

/// Convert YamlValue to JsonValue
pub fn yaml_to_json(value: YamlValue) -> JsonValue {
    match value {
        YamlValue::Null => JsonValue::Null,
        YamlValue::Bool(b) => JsonValue::Bool(b),
        YamlValue::Number(n) => JsonValue::Number(n),
        YamlValue::String(s) => JsonValue::String(s),
        YamlValue::Array(arr) => JsonValue::Array(arr.into_iter().map(yaml_to_json).collect()),
        YamlValue::Mapping(map) => {
            let mut obj = crate::Map::new();
            for (k, v) in map {
                obj.0.push((k.clone(), yaml_to_json(v)));
            }
            JsonValue::Object(obj)
        }
        YamlValue::Tagged(tagged) => yaml_to_json(*tagged.value),
    }
}

#[cfg(feature = "serde")]
pub fn from_yaml_str_typed<T>(s: &str) -> Result<T, YamlError>
where
    T: serde_crate::de::DeserializeOwned,
{
    let yaml = from_yaml_str(s)?;
    let json = yaml_to_json(yaml);
    crate::from_value(json).map_err(|e| YamlError::IoError(e.to_string()))
}

#[cfg(feature = "serde")]
pub fn to_yaml_string_typed<T>(value: &T) -> Result<String, YamlError>
where
    T: serde_crate::Serialize,
{
    let json_value = crate::to_value(value).map_err(|e| YamlError::IoError(e.to_string()))?;
    let yaml_value = json_to_yaml(json_value);
    to_yaml_string(&yaml_value)
}

pub struct YamlDeserializer<'a> {
    input: &'a str,
    offset: usize,
    done: bool,
}

impl<'a> YamlDeserializer<'a> {
    pub fn parse(input: &'a str) -> Self {
        YamlDeserializer {
            input,
            offset: 0,
            done: false,
        }
    }

    fn next_doc(&mut self) -> Option<Result<YamlValue, YamlError>> {
        if self.done {
            return None;
        }

        let docs: Vec<&str> = self.input.split("\n---").collect();
        if self.offset >= docs.len() {
            self.done = true;
            return None;
        }

        let doc = docs[self.offset].trim();
        self.offset += 1;

        if doc.is_empty() || doc == "null" {
            return Some(Ok(YamlValue::Null));
        }

        Some(parse_yaml_value(doc))
    }
}

impl<'a> Iterator for YamlDeserializer<'a> {
    type Item = Result<YamlValue, YamlError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_doc()
    }
}

#[cfg(feature = "serde")]
impl serde_crate::Serialize for YamlValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_crate::Serializer,
    {
        match self {
            YamlValue::Null => serializer.serialize_unit(),
            YamlValue::Bool(b) => serializer.serialize_bool(*b),
            YamlValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    serializer.serialize_i64(i)
                } else if let Some(u) = n.as_u64() {
                    serializer.serialize_u64(u)
                } else if let Some(f) = n.as_f64() {
                    serializer.serialize_f64(f)
                } else {
                    serializer.serialize_str(&n.to_string())
                }
            }
            YamlValue::String(s) => serializer.serialize_str(s),
            YamlValue::Array(arr) => {
                use serde_crate::ser::SerializeSeq;
                let mut seq = serializer.serialize_seq(Some(arr.len()))?;
                for item in arr {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            YamlValue::Mapping(map) => {
                use serde_crate::ser::SerializeMap;
                let mut m = serializer.serialize_map(Some(map.len()))?;
                for (k, v) in map {
                    m.serialize_key(k)?;
                    m.serialize_value(v)?;
                }
                m.end()
            }
            YamlValue::Tagged(tagged) => tagged.value.serialize(serializer),
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde_crate::Deserialize<'de> for YamlValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde_crate::Deserializer<'de>,
    {
        let json = JsonValue::deserialize(deserializer)?;
        Ok(json_to_yaml(json))
    }
}

#[cfg(feature = "serde")]
impl serde_crate::Serialize for YamlError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_crate::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
