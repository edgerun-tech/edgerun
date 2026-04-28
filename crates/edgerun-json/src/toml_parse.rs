//! TOML parsing implementation - no_std + alloc compatible.

use crate::prelude::*;
use crate::{JsonValue, Map, Number};
#[cfg(all(feature = "std", not(target_os = "none")))]
use std::string::String;
#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::string::String;
#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::borrow::ToOwned;
#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::string::ToString;
#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::vec::Vec;

#[derive(Debug)]
pub enum TomlError {
    UnexpectedChar(char),
    UnexpectedEnd,
    InvalidKey,
    DuplicateKey,
    InvalidValue,
}

pub fn from_toml_str(s: &str) -> Result<JsonValue, TomlError> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(JsonValue::empty_object());
    }
    parse_toml_value(trimmed)
}

pub fn parse_toml_value(s: &str) -> Result<JsonValue, TomlError> {
    let mut result = Map::new();
    let mut current_table: Option<String> = None;
    let mut current_section: Option<Map> = None;

    for line in s.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') {
            if let Some(table_name) = current_table.take() {
                if let Some(section) = current_section.take() {
                    if !section.is_empty() {
                        result.insert(table_name, section.into());
                    }
                }
            }

            let closing = line.rfind(']').unwrap_or(line.len() - 1);
            let table_name = &line[1..closing];
            if table_name.starts_with('[') {
                let array_name = table_name.trim_end_matches(']');
                let table_name = array_name.trim_end_matches(|c| c == ']').to_owned();
                current_table = Some(table_name);
            } else {
                current_table = Some(table_name.to_string());
            }
            current_section = Some(Map::new());
            continue;
        }

        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim();
            let value_str = line[eq_pos + 1..].trim();

            let value = parse_toml_value_simple(value_str)?;

            if let Some(table) = current_section.as_mut() {
                table.insert(key.to_owned(), value);
            } else {
                result.insert(key.to_owned(), value);
            }
        }
    }

    if let Some(table_name) = current_table {
        if let Some(section) = current_section {
            if !section.is_empty() {
                result.insert(table_name, section.into());
            }
        }
    }

    Ok(result.into())
}

fn parse_toml_value_simple(s: &str) -> Result<JsonValue, TomlError> {
    let s = s.trim();

    if s.starts_with('"') {
        if s.matches('"').count() >= 3 {
            let inner = &s[1..s.len()-1];
            return Ok(JsonValue::String(unescape_toml_string(inner)));
        }
    }

    if s.starts_with('\'') {
        if s.matches('\'').count() >= 2 {
            let inner = &s[1..s.len()-1];
            return Ok(JsonValue::String(inner.to_string()));
        }
    }

    if s.starts_with('[') && s.ends_with(']') {
        let inner = &s[1..s.len()-1];
        let items: Vec<&str> = inner.split(',').collect();
        let mut arr = Vec::new();
        for item in items {
            arr.push(parse_toml_value_simple(item.trim())?);
        }
        return Ok(arr.into());
    }

    if s == "true" {
        return Ok(JsonValue::Bool(true));
    }
    if s == "false" {
        return Ok(JsonValue::Bool(false));
    }

    if let Ok(i) = s.parse::<i64>() {
        return Ok(JsonValue::Number(Number::I64(i)));
    }
    if let Ok(u) = s.parse::<u64>() {
        return Ok(JsonValue::Number(Number::U64(u)));
    }
    if let Ok(f) = s.parse::<f64>() {
        if let Some(n) = Number::from_f64(f) {
            return Ok(JsonValue::Number(n));
        }
    }

    if s.starts_with("0x") || s.starts_with("0o") || s.starts_with("0b") {
        if let Ok(i) = i64::from_str_radix(&s[2..], match &s[0..2] {
            "0x" => 16,
            "0o" => 8,
            "0b" => 2,
            _ => 10,
        }) {
            return Ok(JsonValue::Number(Number::I64(i)));
        }
    }

    Ok(JsonValue::String(s.to_owned()))
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

pub fn to_toml_string(value: &JsonValue) -> Result<String, TomlError> {
    let mut output = String::new();
    to_toml_value(&mut output, value, 0)?;
    Ok(output)
}

fn to_toml_value(output: &mut String, value: &JsonValue, indent: usize) -> Result<(), TomlError> {
    match value {
        JsonValue::Null => {
            output.push_str("null");
        }
        JsonValue::Bool(b) => {
            output.push_str(if *b { "true" } else { "false" });
        }
        JsonValue::Number(n) => {
            output.push_str(&n.to_string());
        }
        JsonValue::String(s) => {
            if s.contains('"') || s.contains('\n') || s.contains('\\') {
                output.push('"');
                for c in s.chars() {
                    match c {
                        '"' => output.push_str("#quot;"),
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
        JsonValue::Array(arr) => {
            output.push('[');
            for (i, v) in arr.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                to_toml_value(output, v, indent)?;
            }
            output.push(']');
        }
        JsonValue::Object(map) => {
            for (i, (k, v)) in map.iter().enumerate() {
                if i > 0 {
                    output.push('\n');
                }
                match v {
                    JsonValue::Object(_) => {
                        output.push_str(k);
                        output.push_str(" = ");
                        to_toml_value(output, v, indent)?;
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
