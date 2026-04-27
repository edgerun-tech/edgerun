//! YAML parsing implementation - no_std + alloc.

use crate::{JsonValue, Map, Number};
#[cfg(all(feature = "std", not(target_os = "none")))]
use std::string::String;
#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::string::String;
#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::borrow::ToOwned;
#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::string::ToString;
#[cfg(all(feature = "alloc", any(not(feature = "std"), target_os = "none")))]
use alloc::vec::Vec;

#[derive(Debug)]
pub enum YamlError {
    UnexpectedChar(char),
    UnexpectedEnd,
    InvalidIndent,
    DuplicateKey,
}

pub fn from_yaml_str(s: &str) -> Result<JsonValue, YamlError> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(JsonValue::Null);
    }
    parse_yaml_value(trimmed)
}

pub fn parse_yaml_value(s: &str) -> Result<JsonValue, YamlError> {
    let lines: Vec<&str> = s.lines().collect();
    parse_yaml_lines(&lines, 0).map(|(v, _)| v)
}

fn parse_yaml_lines(lines: &[&str], _indent: usize) -> Result<(JsonValue, usize), YamlError> {
    let mut values = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        if line.trim().is_empty() {
            i += 1;
            continue;
        }

        let trimmed = line.trim();

        if trimmed.starts_with('-') {
            let item = trimmed.trim_start_matches('-').trim();
            let item_value = parse_yaml_item(item)?;
            values.push(item_value);
            i += 1;
            continue;
        }

        if let Some(colon_pos) = trimmed.find(':') {
            let key = trimmed[..colon_pos].trim();
            let value_str = trimmed[colon_pos + 1..].trim();

            if value_str.is_empty() {
                i += 1;
                let (nested, new_i) = parse_yaml_lines(lines, 2)?;
                let mut map = Map::new();
                map.insert(key.to_owned(), nested);
                values.push(JsonValue::Object(map));
                i = new_i;
            } else {
                let value = parse_yaml_simple(value_str)?;
                let mut map = Map::new();
                map.insert(key.to_owned(), value);
                values.push(JsonValue::Object(map));
                i += 1;
            }
        } else {
            return parse_yaml_simple(trimmed).map(|v| (v, i + 1));
        }
    }

    let mut result_map = Map::new();
    let mut all_objects = true;
    for v in &values {
        if let JsonValue::Object(m) = v {
            for (k, vv) in m.iter() {
                result_map.insert(k.clone(), vv.clone());
            }
        } else {
            all_objects = false;
            break;
        }
    }

    if result_map.is_empty() && !values.is_empty() {
        Ok((JsonValue::Array(values), i))
    } else {
        Ok((JsonValue::Object(result_map), i))
    }
}

fn parse_yaml_item(item: &str) -> Result<JsonValue, YamlError> {
    let trimmed = item.trim();

    if trimmed.is_empty() {
        return Ok(JsonValue::Null);
    }

    if trimmed.starts_with('-') {
        let inner = trimmed.trim_start_matches('-').trim();
        let (value, _) = parse_yaml_lines(&[inner], 0)?;
        Ok(value)
    } else if let Some(colon_pos) = trimmed.find(':') {
        let key = trimmed[..colon_pos].trim();
        let value_str = trimmed[colon_pos + 1..].trim();

        if value_str.is_empty() {
            let (nested, _) = parse_yaml_lines(&[trimmed], 2)?;
            let mut map = Map::new();
            map.insert(key.to_owned(), nested);
            Ok(JsonValue::Object(map))
        } else {
            let value = parse_yaml_simple(value_str)?;
            let mut map = Map::new();
            map.insert(key.to_owned(), value);
            Ok(JsonValue::Object(map))
        }
    } else {
        parse_yaml_simple(trimmed)
    }
}

fn parse_yaml_simple(s: &str) -> Result<JsonValue, YamlError> {
    let s = s.trim();

    if s == "null" || s == "~" || s.is_empty() {
        return Ok(JsonValue::Null);
    }

    if s == "true" || s == "yes" || s == "on" {
        return Ok(JsonValue::Bool(true));
    }
    if s == "false" || s == "no" || s == "off" {
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

    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        let inner = &s[1..s.len()-1];
        return Ok(JsonValue::String(unescape_yaml_string(inner)));
    }

    Ok(JsonValue::String(s.to_owned()))
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



pub fn to_yaml_string(value: &JsonValue) -> Result<String, YamlError> {
    let mut output = String::new();
    to_yaml_value(&mut output, value, 0)?;
    Ok(output)
}

fn to_yaml_value(output: &mut String, value: &JsonValue, indent: usize) -> Result<(), YamlError> {
    let indent_str = "  ".repeat(indent);
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
            if s.contains(':') || s.contains('#') || s.starts_with(' ') || s.ends_with(' ') || s.contains('\n') {
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
        JsonValue::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                if i > 0 {
                    output.push('\n');
                }
                output.push_str(&indent_str);
                output.push_str("- ");
                to_yaml_value(output, v, indent + 1)?;
            }
        }
        JsonValue::Object(map) => {
            for (i, (k, v)) in map.iter().enumerate() {
                if i > 0 {
                    output.push('\n');
                }
                output.push_str(&indent_str);
                match v {
                    JsonValue::Object(_) | JsonValue::Array(_) => {
                        output.push_str(k);
                        output.push_str(":\n");
                        to_yaml_value(output, v, indent + 1)?;
                    }
                    _ => {
                        output.push_str(k);
                        output.push_str(": ");
                        to_yaml_value(output, v, 0)?;
                    }
                }
            }
        }
    }
    Ok(())
}