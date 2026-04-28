//! JSON value types and operations.
//!
//! # Types
//!
//! - [`JsonValue`] — the main owned JSON value type
//! - [`Value`] — alias for [`JsonValue`]
//! - [`JsonNumber`] — JSON number representation (see [`number`](crate::number))
//!
//! # Example
//!
//! ```
//! use edgerun_json::{json, JsonValue};
//!
//! // Using the json! macro
//! let value = json!({"name": "Alice", "age": 30, "active": true});
//! assert_eq!(value["name"].as_str(), Some("Alice"));
//!
//! // Building programmatically
//! let obj = JsonValue::object(vec![
//!     ("id", 1.into()),
//!     ("tags", JsonValue::array(vec!["a".into(), "b".into()])),
//! ]);
//! assert_eq!(obj["id"].as_u64(), Some(1));
//! ```

use crate::prelude::*;

#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::borrow::ToOwned;
#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::string::ToString;

#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::format;
#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::string::String;
#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::vec::Vec;

use crate::map::Map;
use crate::number::JsonNumber;
use crate::util;
use crate::ValueIndex;
use core::fmt;

/// Error type for fallible JSON value operations.
#[derive(Debug, Clone)]
pub enum JsonValueError {
    /// Operation was attempted on a JSON value of the wrong type.
    WrongType(String),
    /// The input was not valid JSON.
    Parse(crate::JsonParseError),
}

impl fmt::Display for JsonValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonValueError::WrongType(msg) => write!(f, "{}", msg),
            JsonValueError::Parse(error) => write!(f, "{}", error),
        }
    }
}

#[cfg(all(feature = "std", not(target_os = "none")))]
impl std::error::Error for JsonValueError {}

impl From<crate::JsonParseError> for JsonValueError {
    fn from(error: crate::JsonParseError) -> Self {
        Self::Parse(error)
    }
}

/// A JSON value that owns its data.
///
/// This is the main JSON value type in edgerun-json, supporting all standard
/// JSON types: null, boolean, number, string, array, and object.
///
/// # Construction
///
/// Use the [`json!`](crate::json) macro for literal-like syntax, or the [`JsonValue::object`]
/// and [`JsonValue::array`] constructors for programmatic building.
///
/// # Example
///
/// ```
/// use edgerun_json::{json, JsonValue};
///
/// let v = json!({"key": "value", "nums": [1, 2, 3]});
/// assert!(v.is_object());
/// assert_eq!(v["key"].as_str(), Some("value"));
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub enum JsonValue {
    #[default]
    Null,
    Bool(bool),
    Number(JsonNumber),
    String(String),
    Array(Vec<JsonValue>),
    Object(Map),
}

/// Type alias for [`JsonValue`].
pub type Value = JsonValue;
/// Type alias for [`JsonNumber`].
pub type Number = JsonNumber;

impl Eq for JsonValue {}

macro_rules! impl_try_from_json_number {
    ($ty:ty, $method:ident) => {
        impl TryFrom<JsonValue> for $ty {
            type Error = JsonValueError;

            fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
                (&value).try_into()
            }
        }

        impl TryFrom<&JsonValue> for $ty {
            type Error = JsonValueError;

            fn try_from(value: &JsonValue) -> Result<Self, Self::Error> {
                value.$method().ok_or_else(|| {
                    JsonValueError::WrongType(format!(
                        "expected {}, found {}",
                        stringify!($ty),
                        value.variant_name()
                    ))
                })
            }
        }
    };
}

macro_rules! impl_try_from_json_number_cast {
    ($ty:ty, $method:ident) => {
        impl TryFrom<JsonValue> for $ty {
            type Error = JsonValueError;

            fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
                (&value).try_into()
            }
        }

        impl TryFrom<&JsonValue> for $ty {
            type Error = JsonValueError;

            fn try_from(value: &JsonValue) -> Result<Self, Self::Error> {
                value.$method().ok_or_else(|| {
                    JsonValueError::WrongType(format!(
                        "expected {}, found {}",
                        stringify!($ty),
                        value.variant_name()
                    ))
                })
            }
        }
    };
}

impl JsonValue {
    /// Returns the name of this JSON value's variant (for error messages).
    pub(crate) fn variant_name(&self) -> &'static str {
        match self {
            JsonValue::Null => "null",
            JsonValue::Bool(_) => "boolean",
            JsonValue::Number(_) => "number",
            JsonValue::String(_) => "string",
            JsonValue::Array(_) => "array",
            JsonValue::Object(_) => "object",
        }
    }

    /// Creates a JSON object from key-value pairs.
    ///
    /// # Example
    ///
    /// ```
    /// use edgerun_json::JsonValue;
    ///
    /// let obj = JsonValue::object(vec![
    ///     ("name", "Alice".into()),
    ///     ("age", 30.into()),
    /// ]);
    /// assert_eq!(obj["name"].as_str(), Some("Alice"));
    /// ```
    #[must_use]
    pub fn object(entries: Vec<(impl Into<String>, JsonValue)>) -> Self {
        Self::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect::<Vec<_>>()
                .into(),
        )
    }

    /// Creates a JSON array from values.
    ///
    /// # Example
    ///
    /// ```
    /// use edgerun_json::JsonValue;
    ///
    /// let arr = JsonValue::array(vec![1.into(), 2.into(), 3.into()]);
    /// assert_eq!(arr.len(), 3);
    /// assert_eq!(arr[0].as_u64(), Some(1));
    /// ```
    #[must_use]
    pub fn array(values: Vec<JsonValue>) -> Self {
        Self::Array(values)
    }

    #[must_use]
    pub fn empty_object() -> Self {
        Self::Object(Map::new())
    }

    #[must_use]
    pub fn empty_array() -> Self {
        Self::Array(Vec::new())
    }

    #[must_use]
    pub fn object_from_iter<K, V, I>(entries: I) -> Self
    where
        K: Into<String>,
        V: Into<JsonValue>,
        I: IntoIterator<Item = (K, V)>,
    {
        Self::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        )
    }

    #[must_use]
    pub fn array_from_iter<V, I>(values: I) -> Self
    where
        V: Into<JsonValue>,
        I: IntoIterator<Item = V>,
    {
        Self::Array(values.into_iter().map(Into::into).collect())
    }

    /// Serializes this JSON value to a compact JSON string.
    ///
    /// # Example
    ///
    /// ```
    /// use edgerun_json::JsonValue;
    ///
    /// let value = JsonValue::object(vec![("ok", true.into())]);
    /// assert_eq!(value.to_json_string().unwrap(), r#"{"ok":true}"#);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`crate::JsonError::NonFiniteNumber`] if the value contains `NaN` or `Infinity`.
    pub fn to_json_string(&self) -> Result<String, crate::error::JsonError> {
        let mut out = Vec::with_capacity(util::initial_json_capacity(self));
        util::write_json_value(&mut out, self)?;
        Ok(String::from_utf8(out).expect("JSON serialization produced invalid UTF-8"))
    }

    pub fn push_field(&mut self, key: impl Into<String>, value: impl Into<JsonValue>) {
        match self {
            Self::Object(entries) => entries.push((key.into(), value.into())),
            _ => panic!("push_field called on non-object JSON value"),
        }
    }

    /// Non-panicking version of [`push_field`].
    /// Returns `Err` if called on a non-object value.
    pub fn try_push_field(
        &mut self,
        key: impl Into<String>,
        value: impl Into<JsonValue>,
    ) -> Result<(), JsonValueError> {
        match self {
            Self::Object(entries) => {
                entries.push((key.into(), value.into()));
                Ok(())
            }
            other => Err(JsonValueError::WrongType(format!(
                "push_field called on {:?} value, expected object",
                other.variant_name(),
            ))),
        }
    }

    pub fn push_item(&mut self, value: impl Into<JsonValue>) {
        match self {
            Self::Array(values) => values.push(value.into()),
            _ => panic!("push_item called on non-array JSON value"),
        }
    }

    /// Non-panicking version of [`push_item`].
    /// Returns `Err` if called on a non-array value.
    pub fn try_push_item(&mut self, value: impl Into<JsonValue>) -> Result<(), JsonValueError> {
        match self {
            Self::Array(values) => {
                values.push(value.into());
                Ok(())
            }
            other => Err(JsonValueError::WrongType(format!(
                "push_item called on {:?} value, expected array",
                other.variant_name(),
            ))),
        }
    }

    #[must_use]
    pub fn is_null(&self) -> bool {
        self.as_null().is_some()
    }

    #[must_use]
    pub fn as_null(&self) -> Option<()> {
        matches!(self, Self::Null).then_some(())
    }

    #[must_use]
    pub fn is_boolean(&self) -> bool {
        matches!(self, Self::Bool(_))
    }

    #[must_use]
    pub fn is_number(&self) -> bool {
        matches!(self, Self::Number(_))
    }

    #[must_use]
    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    #[must_use]
    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }

    #[must_use]
    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }

    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_number(&self) -> Option<&JsonNumber> {
        match self {
            Self::Number(number) => Some(number),
            _ => None,
        }
    }

    pub fn is_i64(&self) -> bool {
        self.as_number().is_some_and(JsonNumber::is_i64)
    }

    pub fn is_u64(&self) -> bool {
        self.as_number().is_some_and(JsonNumber::is_u64)
    }

    pub fn is_f64(&self) -> bool {
        self.as_number().is_some_and(JsonNumber::is_f64)
    }

    pub fn as_i64(&self) -> Option<i64> {
        self.as_number().and_then(JsonNumber::as_i64)
    }

    pub fn as_u64(&self) -> Option<u64> {
        self.as_number().and_then(JsonNumber::as_u64)
    }

    pub fn as_f64(&self) -> Option<f64> {
        self.as_number().and_then(JsonNumber::as_f64)
    }

    pub fn as_i32(&self) -> Option<i32> {
        self.as_i64().and_then(|value| i32::try_from(value).ok())
    }

    pub fn as_u32(&self) -> Option<u32> {
        self.as_u64().and_then(|value| u32::try_from(value).ok())
    }

    pub fn as_usize(&self) -> Option<usize> {
        self.as_u64().and_then(|value| usize::try_from(value).ok())
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value.as_str()),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_array(&self) -> Option<&Vec<JsonValue>> {
        match self {
            Self::Array(values) => Some(values),
            _ => None,
        }
    }

    pub fn as_array_mut(&mut self) -> Option<&mut Vec<JsonValue>> {
        match self {
            Self::Array(values) => Some(values),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_object(&self) -> Option<&Map> {
        match self {
            Self::Object(entries) => Some(entries),
            _ => None,
        }
    }

    pub fn into_object(self, name: &str) -> Result<Map, JsonValueError> {
        match self {
            Self::Object(entries) => Ok(entries),
            other => Err(JsonValueError::WrongType(format!(
                "{name} must be an object, found {}",
                other.variant_name()
            ))),
        }
    }

    pub fn as_object_mut(&mut self) -> Option<&mut Map> {
        match self {
            Self::Object(entries) => Some(entries),
            _ => None,
        }
    }

    pub fn get<I>(&self, index: I) -> Option<&JsonValue>
    where
        I: ValueIndex,
    {
        index.index_into(self)
    }

    pub fn get_mut<I>(&mut self, index: I) -> Option<&mut JsonValue>
    where
        I: ValueIndex,
    {
        index.index_into_mut(self)
    }

    #[must_use]
    pub fn object_entries(&self) -> Option<impl ExactSizeIterator<Item = (&str, &JsonValue)> + '_> {
        self.as_object().map(Map::fields)
    }

    #[must_use]
    pub fn array_items(&self) -> Option<impl ExactSizeIterator<Item = &JsonValue> + '_> {
        self.as_array().map(|values| values.iter())
    }

    pub fn required(&self, key: &str) -> Result<&JsonValue, JsonValueError> {
        self.as_object()
            .ok_or_else(|| {
                JsonValueError::WrongType(format!(
                    "field lookup expected object, found {}",
                    self.variant_name()
                ))
            })?
            .required(key)
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(JsonValue::as_str)
    }

    pub fn required_str(&self, key: &str) -> Result<&str, JsonValueError> {
        self.required(key)?
            .as_str()
            .ok_or_else(|| JsonValueError::WrongType(format!("field `{key}` expected string")))
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(JsonValue::as_bool)
    }

    pub fn required_bool(&self, key: &str) -> Result<bool, JsonValueError> {
        self.required(key)?
            .as_bool()
            .ok_or_else(|| JsonValueError::WrongType(format!("field `{key}` expected boolean")))
    }

    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(JsonValue::as_i64)
    }

    pub fn required_i64(&self, key: &str) -> Result<i64, JsonValueError> {
        self.required(key)?
            .as_i64()
            .ok_or_else(|| JsonValueError::WrongType(format!("field `{key}` expected i64")))
    }

    pub fn get_u64(&self, key: &str) -> Option<u64> {
        self.get(key).and_then(JsonValue::as_u64)
    }

    pub fn required_u64(&self, key: &str) -> Result<u64, JsonValueError> {
        self.required(key)?
            .as_u64()
            .ok_or_else(|| JsonValueError::WrongType(format!("field `{key}` expected u64")))
    }

    pub fn get_i32(&self, key: &str) -> Option<i32> {
        self.get(key).and_then(JsonValue::as_i32)
    }

    pub fn required_i32(&self, key: &str) -> Result<i32, JsonValueError> {
        self.required(key)?
            .as_i32()
            .ok_or_else(|| JsonValueError::WrongType(format!("field `{key}` expected i32")))
    }

    pub fn get_u32(&self, key: &str) -> Option<u32> {
        self.get(key).and_then(JsonValue::as_u32)
    }

    pub fn required_u32(&self, key: &str) -> Result<u32, JsonValueError> {
        self.required(key)?
            .as_u32()
            .ok_or_else(|| JsonValueError::WrongType(format!("field `{key}` expected u32")))
    }

    pub fn get_usize(&self, key: &str) -> Option<usize> {
        self.get(key).and_then(JsonValue::as_usize)
    }

    pub fn required_usize(&self, key: &str) -> Result<usize, JsonValueError> {
        self.required(key)?
            .as_usize()
            .ok_or_else(|| JsonValueError::WrongType(format!("field `{key}` expected usize")))
    }

    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(JsonValue::as_f64)
    }

    pub fn required_f64(&self, key: &str) -> Result<f64, JsonValueError> {
        self.required(key)?
            .as_f64()
            .ok_or_else(|| JsonValueError::WrongType(format!("field `{key}` expected f64")))
    }

    pub fn get_array(&self, key: &str) -> Option<&Vec<JsonValue>> {
        self.get(key).and_then(JsonValue::as_array)
    }

    pub fn required_array(&self, key: &str) -> Result<&Vec<JsonValue>, JsonValueError> {
        self.required(key)?
            .as_array()
            .ok_or_else(|| JsonValueError::WrongType(format!("field `{key}` expected array")))
    }

    pub fn get_object(&self, key: &str) -> Option<&Map> {
        self.get(key).and_then(JsonValue::as_object)
    }

    pub fn required_object(&self, key: &str) -> Result<&Map, JsonValueError> {
        self.required(key)?
            .as_object()
            .ok_or_else(|| JsonValueError::WrongType(format!("field `{key}` expected object")))
    }

    pub fn required_index(&self, index: usize) -> Result<&JsonValue, JsonValueError> {
        self.as_array()
            .ok_or_else(|| {
                JsonValueError::WrongType(format!(
                    "array index expected array, found {}",
                    self.variant_name()
                ))
            })?
            .get(index)
            .ok_or_else(|| JsonValueError::WrongType(format!("missing array index `{index}`")))
    }

    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Array(values) => values.len(),
            Self::Object(entries) => entries.len(),
            _ => 0,
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub fn as_i128(&self) -> Option<i128> {
        self.as_i64().map(i128::from)
    }

    #[must_use]
    pub fn as_u128(&self) -> Option<u128> {
        self.as_u64().map(u128::from)
    }

    #[must_use]
    pub fn as_f32(&self) -> Option<f32> {
        self.as_f64().map(|v| v as f32)
    }

    #[must_use]
    pub fn get_index(&self, index: usize) -> Option<&JsonValue> {
        match self {
            Self::Array(values) => values.get(index),
            _ => None,
        }
    }

    pub fn index_str(&self, index: usize) -> Option<&str> {
        self.get_index(index).and_then(JsonValue::as_str)
    }

    pub fn required_index_str(&self, index: usize) -> Result<&str, JsonValueError> {
        self.required_index(index)?.as_str().ok_or_else(|| {
            JsonValueError::WrongType(format!("array index `{index}` expected string"))
        })
    }

    pub fn index_bool(&self, index: usize) -> Option<bool> {
        self.get_index(index).and_then(JsonValue::as_bool)
    }

    pub fn required_index_bool(&self, index: usize) -> Result<bool, JsonValueError> {
        self.required_index(index)?.as_bool().ok_or_else(|| {
            JsonValueError::WrongType(format!("array index `{index}` expected boolean"))
        })
    }

    pub fn index_i64(&self, index: usize) -> Option<i64> {
        self.get_index(index).and_then(JsonValue::as_i64)
    }

    pub fn required_index_i64(&self, index: usize) -> Result<i64, JsonValueError> {
        self.required_index(index)?
            .as_i64()
            .ok_or_else(|| JsonValueError::WrongType(format!("array index `{index}` expected i64")))
    }

    pub fn index_u64(&self, index: usize) -> Option<u64> {
        self.get_index(index).and_then(JsonValue::as_u64)
    }

    pub fn required_index_u64(&self, index: usize) -> Result<u64, JsonValueError> {
        self.required_index(index)?
            .as_u64()
            .ok_or_else(|| JsonValueError::WrongType(format!("array index `{index}` expected u64")))
    }

    pub fn index_f64(&self, index: usize) -> Option<f64> {
        self.get_index(index).and_then(JsonValue::as_f64)
    }

    pub fn required_index_f64(&self, index: usize) -> Result<f64, JsonValueError> {
        self.required_index(index)?
            .as_f64()
            .ok_or_else(|| JsonValueError::WrongType(format!("array index `{index}` expected f64")))
    }

    pub fn get_index_mut(&mut self, index: usize) -> Option<&mut JsonValue> {
        match self {
            Self::Array(values) => values.get_mut(index),
            _ => None,
        }
    }

    pub fn take(&mut self) -> JsonValue {
        core::mem::replace(self, JsonValue::Null)
    }

    #[must_use]
    pub fn pointer(&self, pointer: &str) -> Option<&JsonValue> {
        if pointer.is_empty() {
            return Some(self);
        }
        if !pointer.starts_with('/') {
            return None;
        }
        let mut current = self;
        for segment in pointer.split('/').skip(1) {
            let token = crate::util::decode_pointer_segment(segment);
            current = match current {
                JsonValue::Object(entries) => entries
                    .iter()
                    .find(|(key, _)| key.as_str() == token)
                    .map(|(_, value)| value)?,
                JsonValue::Array(values) => values.get(token.parse::<usize>().ok()?)?,
                _ => return None,
            };
        }
        Some(current)
    }

    pub fn pointer_mut(&mut self, pointer: &str) -> Option<&mut JsonValue> {
        if pointer.is_empty() {
            return Some(self);
        }
        if !pointer.starts_with('/') {
            return None;
        }
        let mut current = self;
        for segment in pointer.split('/').skip(1) {
            let token = crate::util::decode_pointer_segment(segment);
            current = match current {
                JsonValue::Object(entries) => entries
                    .iter_mut()
                    .find(|(key, _)| key.as_str() == token)
                    .map(|(_, value)| value)?,
                JsonValue::Array(values) => values.get_mut(token.parse::<usize>().ok()?)?,
                _ => return None,
            };
        }
        Some(current)
    }

    pub fn sort_all_objects(&mut self) {
        match self {
            JsonValue::Object(entries) => {
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                for (_, value) in entries.iter_mut() {
                    value.sort_all_objects();
                }
            }
            JsonValue::Array(values) => {
                for value in values.iter_mut() {
                    value.sort_all_objects();
                }
            }
            _ => {}
        }
    }

    pub fn decode<T, E>(self) -> Result<T, JsonValueError>
    where
        T: TryFrom<JsonValue, Error = E>,
        E: fmt::Display,
    {
        T::try_from(self).map_err(|error| JsonValueError::WrongType(error.to_string()))
    }
}

impl fmt::Display for JsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.to_json_string() {
            Ok(json) => f.write_str(&json),
            Err(_) => Err(fmt::Error),
        }
    }
}

impl From<bool> for JsonValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl TryFrom<JsonValue> for bool {
    type Error = JsonValueError;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        (&value).try_into()
    }
}

impl TryFrom<&JsonValue> for bool {
    type Error = JsonValueError;

    fn try_from(value: &JsonValue) -> Result<Self, Self::Error> {
        value.as_bool().ok_or_else(|| {
            JsonValueError::WrongType(format!("expected bool, found {}", value.variant_name()))
        })
    }
}

impl TryFrom<JsonValue> for String {
    type Error = JsonValueError;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        match value {
            JsonValue::String(value) => Ok(value),
            other => Err(JsonValueError::WrongType(format!(
                "expected string, found {}",
                other.variant_name()
            ))),
        }
    }
}

impl<'a> TryFrom<&'a JsonValue> for &'a str {
    type Error = JsonValueError;

    fn try_from(value: &'a JsonValue) -> Result<Self, Self::Error> {
        value.as_str().ok_or_else(|| {
            JsonValueError::WrongType(format!("expected string, found {}", value.variant_name()))
        })
    }
}

impl TryFrom<JsonValue> for Vec<JsonValue> {
    type Error = JsonValueError;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        match value {
            JsonValue::Array(values) => Ok(values),
            other => Err(JsonValueError::WrongType(format!(
                "expected array, found {}",
                other.variant_name()
            ))),
        }
    }
}

impl TryFrom<JsonValue> for Map {
    type Error = JsonValueError;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        match value {
            JsonValue::Object(entries) => Ok(entries),
            other => Err(JsonValueError::WrongType(format!(
                "expected object, found {}",
                other.variant_name()
            ))),
        }
    }
}

impl_try_from_json_number!(i64, as_i64);
impl_try_from_json_number!(u64, as_u64);
impl_try_from_json_number!(f64, as_f64);
impl_try_from_json_number!(i128, as_i128);
impl_try_from_json_number!(u128, as_u128);
impl_try_from_json_number_cast!(i32, as_i32);
impl_try_from_json_number_cast!(u32, as_u32);
impl_try_from_json_number_cast!(usize, as_usize);

impl From<String> for JsonValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for JsonValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<&String> for JsonValue {
    fn from(value: &String) -> Self {
        Self::String(value.clone())
    }
}

impl From<i8> for JsonValue {
    fn from(value: i8) -> Self {
        Self::Number(JsonNumber::from(i64::from(value)))
    }
}

impl From<i16> for JsonValue {
    fn from(value: i16) -> Self {
        Self::Number(JsonNumber::from(i64::from(value)))
    }
}

impl From<i32> for JsonValue {
    fn from(value: i32) -> Self {
        Self::Number(JsonNumber::from(i64::from(value)))
    }
}

impl From<i64> for JsonValue {
    fn from(value: i64) -> Self {
        Self::Number(JsonNumber::from(value))
    }
}

impl From<isize> for JsonValue {
    fn from(value: isize) -> Self {
        Self::Number(JsonNumber::from(value as i64))
    }
}

impl From<u8> for JsonValue {
    fn from(value: u8) -> Self {
        Self::Number(JsonNumber::U64(u64::from(value)))
    }
}

impl From<u16> for JsonValue {
    fn from(value: u16) -> Self {
        Self::Number(JsonNumber::U64(u64::from(value)))
    }
}

impl From<u32> for JsonValue {
    fn from(value: u32) -> Self {
        Self::Number(JsonNumber::U64(u64::from(value)))
    }
}

impl From<u64> for JsonValue {
    fn from(value: u64) -> Self {
        Self::Number(JsonNumber::U64(value))
    }
}

impl From<usize> for JsonValue {
    fn from(value: usize) -> Self {
        Self::Number(JsonNumber::U64(value as u64))
    }
}

impl From<f32> for JsonValue {
    fn from(value: f32) -> Self {
        Self::Number(JsonNumber::F64(f64::from(value)))
    }
}

impl From<f64> for JsonValue {
    fn from(value: f64) -> Self {
        Self::Number(JsonNumber::F64(value))
    }
}

impl From<i128> for JsonValue {
    fn from(value: i128) -> Self {
        JsonNumber::from_i128(value).map_or_else(|| Self::String(value.to_string()), Self::Number)
    }
}

impl From<u128> for JsonValue {
    fn from(value: u128) -> Self {
        JsonNumber::from_u128(value).map_or_else(|| Self::String(value.to_string()), Self::Number)
    }
}

impl<T> From<Option<T>> for JsonValue
where
    T: Into<JsonValue>,
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => value.into(),
            None => Self::Null,
        }
    }
}

impl<T> From<Vec<T>> for JsonValue
where
    T: Into<JsonValue>,
{
    fn from(values: Vec<T>) -> Self {
        Self::Array(values.into_iter().map(Into::into).collect())
    }
}

impl From<Map> for JsonValue {
    fn from(value: Map) -> Self {
        Self::Object(value)
    }
}

#[cfg(all(feature = "std", not(target_os = "none")))]
impl<K, V> From<std::collections::HashMap<K, V>> for JsonValue
where
    K: Into<String> + Eq + std::hash::Hash,
    V: Into<JsonValue>,
{
    fn from(map: std::collections::HashMap<K, V>) -> Self {
        Self::Object(
            map.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect::<Vec<_>>()
                .into(),
        )
    }
}

#[cfg(all(feature = "std", not(target_os = "none")))]
impl<K, V> From<std::collections::BTreeMap<K, V>> for JsonValue
where
    K: Into<String> + Ord,
    V: Into<JsonValue>,
{
    fn from(map: std::collections::BTreeMap<K, V>) -> Self {
        Self::Object(
            map.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect::<Vec<_>>()
                .into(),
        )
    }
}

impl<K, V> core::iter::FromIterator<(K, V)> for JsonValue
where
    K: Into<String>,
    V: Into<JsonValue>,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self::Object(
            iter.into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect::<Vec<_>>()
                .into(),
        )
    }
}

impl<T> core::iter::FromIterator<T> for JsonValue
where
    T: Into<JsonValue>,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::Array(iter.into_iter().map(Into::into).collect())
    }
}
