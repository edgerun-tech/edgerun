//! High-level JSON parsing and serialization API.
//!
//! This module provides the main entry points for working with JSON data.
//! The primary entry points work with owned [`JsonValue`] values. Typed models
//! should implement [`crate::FromJson`] and [`crate::ToJson`].
//!
//! # Parsing
//!
//! - [`parse_json`] — parse into an owned [`JsonValue`]
//! - [`parse_json_borrowed`] — parse with zero-copy string borrowing
//! - [`parse_json_tape`] — parse into a token tape for fast indexed access
//! - [`from_str`] / [`from_slice`] — parse into [`JsonValue`]
//!
//! # Serialization
//!
//! - [`to_string`] / [`to_vec`] — compact serialization
//! - [`to_string_pretty`] / [`to_vec_pretty`] — pretty (indented) serialization
//! - [`to_writer`] / [`to_writer_pretty`] — write to any [`Write`] implementation
//! - [`from_reader`] — read from any [`Read`] implementation
//!
#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::string::{String, ToString};
#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::vec::Vec;

use crate::error::JsonError;
use crate::error::JsonParseError;
use crate::io::{Read, Write};
use crate::prelude::*;
use crate::util;
use crate::{JsonValue, JsonValueError};
use core::fmt;

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Escapes a string as a JSON string literal (including surrounding quotes).
///
/// # Example
///
/// ```
/// use edgerun_json::escape_json_string;
///
/// let escaped = escape_json_string("hello\tworld");
/// assert_eq!(escaped, r#""hello\tworld""#);
/// ```
pub fn escape_json_string(input: &str) -> String {
    let mut out = Vec::with_capacity(input.len() + 2);
    util::write_escaped_json_string(&mut out, input);
    String::from_utf8(out).expect("JSON escape produced invalid UTF-8")
}

/// Parses a JSON string into an owned [`JsonValue`].
///
/// # Example
///
/// ```
/// use edgerun_json::parse_json;
///
/// let value = parse_json(r#"{"name":"Alice","scores":[95,87]}"#).unwrap();
/// assert_eq!(value["name"].as_str(), Some("Alice"));
/// ```
///
/// # Errors
///
/// Returns a [`JsonParseError`] if the input is not valid JSON.
pub fn parse_json(input: &str) -> Result<JsonValue, JsonParseError> {
    let mut parser = crate::parse::Parser::new(input);
    let value = parser.parse_value()?;
    parser.skip_whitespace();
    if parser.is_eof() {
        Ok(value)
    } else {
        Err(JsonParseError::UnexpectedTrailingCharacters(parser.index()))
    }
}

/// Parses a JSON string with zero-copy borrowing of plain strings and keys.
///
/// This avoids allocating [`String`]s for string values and object keys
/// when no unescaping is needed. The result borrows from the input.
///
/// # Example
///
/// ```
/// use edgerun_json::parse_json_borrowed;
///
/// let input = r#"{"name":"Alice","active":true}"#;
/// let value = parse_json_borrowed(input).unwrap();
/// // Convert to owned for easy inspection
/// let owned = value.into_owned();
/// assert_eq!(owned["name"].as_str(), Some("Alice"));
/// ```
///
/// # Errors
///
/// Returns a [`JsonParseError`] if the input is not valid JSON.
pub fn parse_json_borrowed(
    input: &str,
) -> Result<crate::borrowed_value::BorrowedJsonValue<'_>, JsonParseError> {
    let mut parser = crate::parse::Parser::new(input);
    let value = parser.parse_value_borrowed()?;
    parser.skip_whitespace();
    if parser.is_eof() {
        Ok(value)
    } else {
        Err(JsonParseError::UnexpectedTrailingCharacters(parser.index()))
    }
}

/// Parses a JSON string into a token tape for fast indexed access.
///
/// The tape is a flat array of tokens with parent pointers, enabling
/// efficient random access without recursive traversal.
///
/// # Example
///
/// ```
/// use edgerun_json::parse_json_tape;
///
/// let tape = parse_json_tape(r#"{"users":[{"name":"Alice"},{"name":"Bob"}]}"#).unwrap();
/// let root = tape.root(r#"{"users":[{"name":"Alice"},{"name":"Bob"}]}"#).unwrap();
/// let users = root.get("users").unwrap();
/// assert_eq!(users.kind(), edgerun_json::TapeTokenKind::Array);
/// ```
///
/// # Errors
///
/// Returns a [`JsonParseError`] if the input is not valid JSON.
pub fn parse_json_tape(input: &str) -> Result<crate::tape::JsonTape, JsonParseError> {
    let mut parser = crate::parse::Parser::new(input);
    let mut tokens = Vec::new();
    parser.parse_tape_value(&mut tokens, None)?;
    parser.skip_whitespace();
    if parser.is_eof() {
        Ok(crate::tape::JsonTape { tokens })
    } else {
        Err(JsonParseError::UnexpectedTrailingCharacters(parser.index()))
    }
}

/// Converts an owned [`JsonValue`] into a caller-defined type.
///
/// This uses [`TryFrom<JsonValue>`], so crates can implement small explicit
/// mappers for their protocol structs without adding a reflection dependency.
pub fn from_value_as<T, E>(value: JsonValue) -> Result<T, JsonValueError>
where
    T: TryFrom<JsonValue, Error = E>,
    E: fmt::Display,
{
    T::try_from(value).map_err(|error| JsonValueError::WrongType(error.to_string()))
}

/// Parses JSON and converts the owned value into a caller-defined type.
pub fn from_str_as<T, E>(input: &str) -> Result<T, JsonValueError>
where
    T: TryFrom<JsonValue, Error = E>,
    E: fmt::Display,
{
    let tape = parse_json_tape(input)?;
    let value = tape
        .root(input)
        .and_then(|root| root.to_json_value())
        .ok_or_else(|| JsonValueError::WrongType("missing JSON root value".to_string()))?;
    from_value_as(value)
}

/// Parses a UTF-8 JSON byte slice and converts it into a caller-defined type.
pub fn from_slice_as<T, E>(input: &[u8]) -> Result<T, JsonValueError>
where
    T: TryFrom<JsonValue, Error = E>,
    E: fmt::Display,
{
    let input = core::str::from_utf8(input).map_err(|_| JsonParseError::InvalidUtf8)?;
    from_str_as(input)
}

/// Parses a JSON string.
///
/// # Example
///
/// ```
/// use edgerun_json::{from_str, JsonValue};
///
/// let value: JsonValue = from_str(r#"{"name":"Alice"}"#).unwrap();
/// assert_eq!(value["name"].as_str(), Some("Alice"));
/// ```
pub fn from_str(input: &str) -> Result<JsonValue, JsonParseError> {
    parse_json(input)
}

/// Parses a JSON byte slice.
///
/// # Errors
///
/// Returns [`JsonParseError::InvalidUtf8`] if the input is not valid UTF-8.
pub fn from_slice(input: &[u8]) -> Result<JsonValue, JsonParseError> {
    let input = core::str::from_utf8(input).map_err(|_| JsonParseError::InvalidUtf8)?;
    parse_json(input)
}

/// Reads JSON from a reader and parses it.
pub fn from_reader<R: Read>(mut reader: R) -> Result<JsonValue, JsonParseError> {
    let mut input = String::new();
    reader
        .read_to_string(&mut input)
        .map_err(|_| JsonParseError::InvalidUtf8)?;
    parse_json(&input)
}

/// Serializes a value to a compact JSON string.
///
/// # Example
///
/// ```
/// use edgerun_json::{json, to_string};
///
/// let value = json!({"name": "Alice", "age": 30});
/// assert_eq!(to_string(&value).unwrap(), r#"{"name":"Alice","age":30}"#);
/// ```
/// Serializes a value to a compact JSON string through EdgeRun's native JSON value.
pub fn to_string<T: crate::ToJson + ?Sized>(value: &T) -> Result<String, JsonError> {
    value.to_json().to_json_string()
}

/// Serializes a value to compact JSON bytes through EdgeRun's native JSON value.
pub fn to_vec<T: crate::ToJson + ?Sized>(value: &T) -> Result<Vec<u8>, JsonError> {
    let value = value.to_json();
    let mut out = Vec::with_capacity(util::initial_json_capacity(&value));
    util::write_json_value(&mut out, &value)?;
    Ok(out)
}

/// Writes JSON to a writer.
pub fn to_writer<W: Write>(mut writer: W, value: &JsonValue) -> Result<(), JsonError> {
    let bytes = to_vec(value)?;
    writer.write_all(&bytes).map_err(|_| JsonError::Io)
}

/// Serializes a value to a pretty-printed JSON string.
///
/// # Example
///
/// ```
/// use edgerun_json::{json, to_string_pretty};
///
/// let value = json!({"name": "Alice", "age": 30});
/// let pretty = to_string_pretty(&value).unwrap();
/// assert!(pretty.contains("\n"));
/// ```
/// Serializes a value to a pretty-printed JSON string.
pub fn to_string_pretty<T: crate::ToJson + ?Sized>(value: &T) -> Result<String, JsonError> {
    let value = value.to_json();
    let mut out = Vec::with_capacity(util::initial_json_capacity(&value) + 16);
    util::write_json_value_pretty(&mut out, &value, 0)?;
    Ok(String::from_utf8(out).expect("JSON serialization produced invalid UTF-8"))
}

/// Serializes a value to pretty-printed JSON bytes.
pub fn to_vec_pretty<T: crate::ToJson + ?Sized>(value: &T) -> Result<Vec<u8>, JsonError> {
    let value = value.to_json();
    let mut out = Vec::with_capacity(util::initial_json_capacity(&value) + 16);
    util::write_json_value_pretty(&mut out, &value, 0)?;
    Ok(out)
}

/// Writes pretty-printed JSON to a writer.
pub fn to_writer_pretty<W: Write>(mut writer: W, value: &JsonValue) -> Result<(), JsonError> {
    let bytes = to_vec_pretty(value)?;
    writer.write_all(&bytes).map_err(|_| JsonError::Io)
}
