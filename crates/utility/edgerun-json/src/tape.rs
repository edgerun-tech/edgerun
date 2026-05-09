use crate::prelude::*;

#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::vec;

#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::borrow::ToOwned;

#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::string::String;
#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::vec::Vec;

use crate::JsonValue;
use crate::JsonValueError;
use crate::error::JsonError;
use crate::util;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledObjectSchema {
    fields: Vec<CompiledField>,
    capacity_hint: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledRowSchema {
    object: CompiledObjectSchema,
    row_capacity_hint: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonTape {
    pub tokens: Vec<TapeToken>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TapeToken {
    pub kind: TapeTokenKind,
    pub start: usize,
    pub end: usize,
    pub parent: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TapeTokenKind {
    Null,
    Bool,
    Number,
    String,
    Key,
    Array,
    Object,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TapeValue<'a> {
    tape: &'a JsonTape,
    input: &'a str,
    index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TapeObjectIndex {
    buckets: Vec<Vec<(u64, usize, usize)>>,
}

#[derive(Clone, Copy, Debug)]
pub struct IndexedTapeObject<'a> {
    object: TapeValue<'a>,
    index: &'a TapeObjectIndex,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledTapeKey {
    key: String,
    hash: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledTapeKeys {
    keys: Vec<CompiledTapeKey>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CompiledField {
    key: String,
    rendered_prefix: Vec<u8>,
}

impl JsonTape {
    #[must_use]
    pub fn root<'a>(&'a self, input: &'a str) -> Option<TapeValue<'a>> {
        (!self.tokens.is_empty()).then_some(TapeValue {
            tape: self,
            input,
            index: 0,
        })
    }
}

impl<'a> TapeValue<'a> {
    fn token(&self) -> &TapeToken {
        &self.tape.tokens[self.index]
    }

    fn raw(&self) -> &'a str {
        let token = self.token();
        &self.input[token.start..token.end]
    }

    #[must_use]
    pub fn kind(&self) -> TapeTokenKind {
        self.tape.tokens[self.index].kind
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&'a str> {
        let token = &self.tape.tokens[self.index];
        match token.kind {
            TapeTokenKind::String | TapeTokenKind::Key => {
                if self.input.as_bytes()[token.start] == b'"'
                    && self.input.as_bytes()[token.end - 1] == b'"'
                {
                    Some(&self.input[token.start + 1..token.end - 1])
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match (self.kind(), self.raw()) {
            (TapeTokenKind::Bool, "true") => Some(true),
            (TapeTokenKind::Bool, "false") => Some(false),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        if self.kind() != TapeTokenKind::Number {
            return None;
        }
        let raw = self.raw();
        if raw.contains(['.', 'e', 'E']) {
            return None;
        }
        raw.parse().ok()
    }

    #[must_use]
    pub fn as_u64(&self) -> Option<u64> {
        if self.kind() != TapeTokenKind::Number {
            return None;
        }
        let raw = self.raw();
        if raw.starts_with('-') || raw.contains(['.', 'e', 'E']) {
            return None;
        }
        raw.parse().ok()
    }

    #[must_use]
    pub fn as_i32(&self) -> Option<i32> {
        self.as_i64().and_then(|value| i32::try_from(value).ok())
    }

    #[must_use]
    pub fn as_u32(&self) -> Option<u32> {
        self.as_u64().and_then(|value| u32::try_from(value).ok())
    }

    #[must_use]
    pub fn as_usize(&self) -> Option<usize> {
        self.as_u64().and_then(|value| usize::try_from(value).ok())
    }

    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        (self.kind() == TapeTokenKind::Number)
            .then(|| self.raw().parse().ok())
            .flatten()
    }

    #[must_use]
    pub fn is_null(&self) -> bool {
        self.kind() == TapeTokenKind::Null
    }

    #[must_use]
    pub fn to_json_value(&self) -> Option<JsonValue> {
        match self.kind() {
            TapeTokenKind::Null => Some(JsonValue::Null),
            TapeTokenKind::Bool => self.as_bool().map(JsonValue::Bool),
            TapeTokenKind::Number => {
                if let Some(value) = self.as_i64() {
                    Some(JsonValue::from(value))
                } else if let Some(value) = self.as_u64() {
                    Some(JsonValue::from(value))
                } else {
                    self.as_f64().map(JsonValue::from)
                }
            }
            TapeTokenKind::String | TapeTokenKind::Key => self.as_str().map(JsonValue::from),
            TapeTokenKind::Array => self.array_items().map(|items| {
                JsonValue::Array(
                    items
                        .into_iter()
                        .filter_map(|v| v.to_json_value())
                        .collect(),
                )
            }),
            TapeTokenKind::Object => self.object_fields().map(|fields| {
                JsonValue::Object(
                    fields
                        .into_iter()
                        .filter_map(|(key, value)| {
                            value.to_json_value().map(|value| (key.to_owned(), value))
                        })
                        .collect(),
                )
            }),
        }
    }

    #[must_use]
    pub fn array_items(&self) -> Option<Vec<TapeValue<'a>>> {
        if self.kind() != TapeTokenKind::Array {
            return None;
        }
        Some(
            self.tape
                .tokens
                .iter()
                .enumerate()
                .filter_map(|(index, token)| {
                    (token.parent == Some(self.index)).then_some(TapeValue {
                        tape: self.tape,
                        input: self.input,
                        index,
                    })
                })
                .collect(),
        )
    }

    #[must_use]
    pub fn object_fields(&self) -> Option<Vec<(&'a str, TapeValue<'a>)>> {
        if self.kind() != TapeTokenKind::Object {
            return None;
        }
        let mut fields = Vec::new();
        let tokens = &self.tape.tokens;
        let mut i = self.index + 1;
        while i + 1 < tokens.len() {
            if tokens[i].parent != Some(self.index) || tokens[i].kind != TapeTokenKind::Key {
                i += 1;
                continue;
            }
            let key = TapeValue {
                tape: self.tape,
                input: self.input,
                index: i,
            };
            let value_index = i + 1;
            if tokens[value_index].parent == Some(self.index) {
                if let Some(key) = key.as_str() {
                    fields.push((
                        key,
                        TapeValue {
                            tape: self.tape,
                            input: self.input,
                            index: value_index,
                        },
                    ));
                }
            }
            i += 2;
        }
        Some(fields)
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<TapeValue<'a>> {
        if self.kind() != TapeTokenKind::Object {
            return None;
        }
        self.get_linear(key)
    }

    pub fn required(&self, key: &str) -> Result<TapeValue<'a>, JsonValueError> {
        if self.kind() != TapeTokenKind::Object {
            return Err(JsonValueError::WrongType(format!(
                "field lookup expected object, found {:?}",
                self.kind()
            )));
        }
        self.get(key)
            .ok_or_else(|| JsonValueError::WrongType(format!("missing required field `{key}`")))
    }

    fn optional_field<T>(
        &self,
        key: &str,
        expected: &str,
        convert: impl FnOnce(TapeValue<'a>) -> Option<T>,
    ) -> Result<Option<T>, JsonValueError> {
        self.get(key)
            .map(|value| {
                convert(value).ok_or_else(|| {
                    JsonValueError::WrongType(format!("field `{key}` expected {expected}"))
                })
            })
            .transpose()
    }

    #[must_use]
    pub fn get_str(&self, key: &str) -> Option<&'a str> {
        self.get(key).and_then(|value| value.as_str())
    }

    pub fn optional_str(&self, key: &str) -> Result<Option<&'a str>, JsonValueError> {
        self.optional_field(key, "string", |value| value.as_str())
    }

    pub fn required_str(&self, key: &str) -> Result<&'a str, JsonValueError> {
        self.required(key)?
            .as_str()
            .ok_or_else(|| JsonValueError::WrongType(format!("field `{key}` expected string")))
    }

    #[must_use]
    pub fn get_string(&self, key: &str) -> Option<String> {
        self.get_str(key).map(ToOwned::to_owned)
    }

    pub fn optional_string(&self, key: &str) -> Result<Option<String>, JsonValueError> {
        self.optional_str(key)
            .map(|value| value.map(ToOwned::to_owned))
    }

    pub fn required_string(&self, key: &str) -> Result<String, JsonValueError> {
        self.required_str(key).map(ToOwned::to_owned)
    }

    #[must_use]
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(|value| value.as_bool())
    }

    pub fn optional_bool(&self, key: &str) -> Result<Option<bool>, JsonValueError> {
        self.optional_field(key, "boolean", |value| value.as_bool())
    }

    pub fn required_bool(&self, key: &str) -> Result<bool, JsonValueError> {
        self.required(key)?
            .as_bool()
            .ok_or_else(|| JsonValueError::WrongType(format!("field `{key}` expected boolean")))
    }

    #[must_use]
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|value| value.as_i64())
    }

    pub fn optional_i64(&self, key: &str) -> Result<Option<i64>, JsonValueError> {
        self.optional_field(key, "i64", |value| value.as_i64())
    }

    pub fn required_i64(&self, key: &str) -> Result<i64, JsonValueError> {
        self.required(key)?
            .as_i64()
            .ok_or_else(|| JsonValueError::WrongType(format!("field `{key}` expected i64")))
    }

    #[must_use]
    pub fn get_i32(&self, key: &str) -> Option<i32> {
        self.get(key).and_then(|value| value.as_i32())
    }

    pub fn optional_i32(&self, key: &str) -> Result<Option<i32>, JsonValueError> {
        self.optional_field(key, "i32", |value| value.as_i32())
    }

    pub fn required_i32(&self, key: &str) -> Result<i32, JsonValueError> {
        self.required(key)?
            .as_i32()
            .ok_or_else(|| JsonValueError::WrongType(format!("field `{key}` expected i32")))
    }

    #[must_use]
    pub fn get_u64(&self, key: &str) -> Option<u64> {
        self.get(key).and_then(|value| value.as_u64())
    }

    pub fn optional_u64(&self, key: &str) -> Result<Option<u64>, JsonValueError> {
        self.optional_field(key, "u64", |value| value.as_u64())
    }

    pub fn required_u64(&self, key: &str) -> Result<u64, JsonValueError> {
        self.required(key)?
            .as_u64()
            .ok_or_else(|| JsonValueError::WrongType(format!("field `{key}` expected u64")))
    }

    #[must_use]
    pub fn get_u32(&self, key: &str) -> Option<u32> {
        self.get(key).and_then(|value| value.as_u32())
    }

    pub fn optional_u32(&self, key: &str) -> Result<Option<u32>, JsonValueError> {
        self.optional_field(key, "u32", |value| value.as_u32())
    }

    pub fn required_u32(&self, key: &str) -> Result<u32, JsonValueError> {
        self.required(key)?
            .as_u32()
            .ok_or_else(|| JsonValueError::WrongType(format!("field `{key}` expected u32")))
    }

    #[must_use]
    pub fn get_usize(&self, key: &str) -> Option<usize> {
        self.get(key).and_then(|value| value.as_usize())
    }

    pub fn optional_usize(&self, key: &str) -> Result<Option<usize>, JsonValueError> {
        self.optional_field(key, "usize", |value| value.as_usize())
    }

    pub fn required_usize(&self, key: &str) -> Result<usize, JsonValueError> {
        self.required(key)?
            .as_usize()
            .ok_or_else(|| JsonValueError::WrongType(format!("field `{key}` expected usize")))
    }

    #[must_use]
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(|value| value.as_f64())
    }

    pub fn optional_f64(&self, key: &str) -> Result<Option<f64>, JsonValueError> {
        self.optional_field(key, "f64", |value| value.as_f64())
    }

    pub fn required_f64(&self, key: &str) -> Result<f64, JsonValueError> {
        self.required(key)?
            .as_f64()
            .ok_or_else(|| JsonValueError::WrongType(format!("field `{key}` expected f64")))
    }

    #[must_use]
    pub fn get_array(&self, key: &str) -> Option<Vec<TapeValue<'a>>> {
        self.get(key).and_then(|value| value.array_items())
    }

    pub fn required_array(&self, key: &str) -> Result<Vec<TapeValue<'a>>, JsonValueError> {
        self.required(key)?
            .array_items()
            .ok_or_else(|| JsonValueError::WrongType(format!("field `{key}` expected array")))
    }

    #[must_use]
    pub fn get_object_fields(&self, key: &str) -> Option<Vec<(&'a str, TapeValue<'a>)>> {
        self.get(key).and_then(|value| value.object_fields())
    }

    pub fn required_object_fields(
        &self,
        key: &str,
    ) -> Result<Vec<(&'a str, TapeValue<'a>)>, JsonValueError> {
        self.required(key)?
            .object_fields()
            .ok_or_else(|| JsonValueError::WrongType(format!("field `{key}` expected object")))
    }

    #[must_use]
    pub fn build_object_index(&self) -> Option<TapeObjectIndex> {
        if self.kind() != TapeTokenKind::Object {
            return None;
        }
        let parent = self.index;
        let tokens = &self.tape.tokens;
        let mut entries = Vec::new();
        let mut i = self.index + 1;
        while i + 1 < tokens.len() {
            if tokens[i].parent != Some(parent) {
                i += 1;
                continue;
            }
            if tokens[i].kind == TapeTokenKind::Key && tokens[i + 1].parent == Some(parent) {
                let candidate = TapeValue {
                    tape: self.tape,
                    input: self.input,
                    index: i,
                };
                let key = candidate.as_str().unwrap_or("");
                let hash = util::hash_key(key.as_bytes());
                entries.push((hash, i, i + 1));
                i += 2;
            } else {
                i += 1;
            }
        }
        let bucket_count = (entries.len().next_power_of_two().max(1)) * 2;
        let mut buckets = vec![Vec::new(); bucket_count];
        for entry in entries {
            let bucket = (entry.0 as usize) & (bucket_count - 1);
            buckets[bucket].push(entry);
        }
        Some(TapeObjectIndex { buckets })
    }

    #[must_use]
    pub fn with_index<'b>(&'b self, index: &'b TapeObjectIndex) -> IndexedTapeObject<'b> {
        IndexedTapeObject {
            object: TapeValue {
                tape: self.tape,
                input: self.input,
                index: self.index,
            },
            index,
        }
    }

    fn get_linear(&self, key: &str) -> Option<TapeValue<'a>> {
        let parent = self.index;
        let tokens = &self.tape.tokens;
        let mut i = self.index + 1;
        while i < tokens.len() {
            if tokens[i].parent != Some(parent) {
                i += 1;
                continue;
            }
            if tokens[i].kind != TapeTokenKind::Key {
                i += 1;
                continue;
            }
            let candidate = TapeValue {
                tape: self.tape,
                input: self.input,
                index: i,
            };
            if candidate.as_str() == Some(key) {
                let value_index = i + 1;
                if value_index < tokens.len() && tokens[value_index].parent == Some(parent) {
                    return Some(TapeValue {
                        tape: self.tape,
                        input: self.input,
                        index: value_index,
                    });
                }
                return None;
            }
            i += 1;
        }
        None
    }
}

impl TapeObjectIndex {
    #[must_use]
    pub fn get<'a>(&self, object: TapeValue<'a>, key: &str) -> Option<TapeValue<'a>> {
        self.get_hashed(object, util::hash_key(key.as_bytes()), key)
    }

    #[must_use]
    pub fn get_compiled<'a>(
        &self,
        object: TapeValue<'a>,
        key: &CompiledTapeKey,
    ) -> Option<TapeValue<'a>> {
        self.get_hashed(object, key.hash, &key.key)
    }

    fn get_hashed<'a>(&self, object: TapeValue<'a>, hash: u64, key: &str) -> Option<TapeValue<'a>> {
        let bucket = (hash as usize) & (self.buckets.len() - 1);
        for (entry_hash, key_index, value_index) in &self.buckets[bucket] {
            if *entry_hash != hash {
                continue;
            }
            let candidate = TapeValue {
                tape: object.tape,
                input: object.input,
                index: *key_index,
            };
            if candidate.as_str() == Some(key) {
                return Some(TapeValue {
                    tape: object.tape,
                    input: object.input,
                    index: *value_index,
                });
            }
        }
        None
    }
}

impl CompiledTapeKey {
    pub fn new(key: impl Into<String>) -> Self {
        let key = key.into();
        let hash = util::hash_key(key.as_bytes());
        Self { key, hash }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.key
    }
}

impl CompiledTapeKeys {
    #[must_use]
    pub fn new(keys: &[&str]) -> Self {
        Self {
            keys: keys.iter().map(|key| CompiledTapeKey::new(*key)).collect(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &CompiledTapeKey> {
        self.keys.iter()
    }
}

impl<'a> IndexedTapeObject<'a> {
    #[must_use]
    pub fn get(&self, key: &str) -> Option<TapeValue<'a>> {
        self.index.get(self.object, key)
    }

    #[must_use]
    pub fn get_compiled(&self, key: &CompiledTapeKey) -> Option<TapeValue<'a>> {
        self.index.get_compiled(self.object, key)
    }

    pub fn get_many<'b>(
        &'b self,
        keys: &'b [&'b str],
    ) -> impl Iterator<Item = Option<TapeValue<'a>>> + 'b {
        keys.iter().map(|key| self.get(key))
    }

    pub fn get_compiled_many<'b>(
        &'b self,
        keys: &'b CompiledTapeKeys,
    ) -> impl Iterator<Item = Option<TapeValue<'a>>> + 'b {
        keys.iter().map(|key| self.get_compiled(key))
    }
}

impl CompiledObjectSchema {
    #[must_use]
    pub fn new(keys: &[&str]) -> Self {
        let mut fields = Vec::with_capacity(keys.len());
        let mut capacity_hint = 2;
        for (index, key) in keys.iter().enumerate() {
            let mut rendered_prefix = Vec::with_capacity(key.len() + 4);
            if index > 0 {
                rendered_prefix.push(b',');
            }
            util::write_json_key(&mut rendered_prefix, key);
            capacity_hint += rendered_prefix.len() + 8;
            fields.push(CompiledField {
                key: (*key).to_owned(),
                rendered_prefix,
            });
        }
        Self {
            fields,
            capacity_hint,
        }
    }

    #[must_use]
    pub fn keys(&self) -> impl ExactSizeIterator<Item = &str> {
        self.fields.iter().map(|field| field.key.as_str())
    }

    pub fn to_json_string<'a, I>(&self, values: I) -> Result<String, JsonError>
    where
        I: IntoIterator<Item = &'a JsonValue>,
    {
        let mut out = Vec::with_capacity(self.capacity_hint);
        self.write_json_bytes(&mut out, values)?;
        Ok(String::from_utf8(out).expect("JSON serialization produced invalid UTF-8"))
    }

    pub fn write_json_bytes<'a, I>(&self, out: &mut Vec<u8>, values: I) -> Result<(), JsonError>
    where
        I: IntoIterator<Item = &'a JsonValue>,
    {
        out.push(b'{');
        let mut iter = values.into_iter();
        for field in &self.fields {
            let Some(value) = iter.next() else {
                panic!(
                    "compiled object schema expected {} values",
                    self.fields.len()
                );
            };
            out.extend_from_slice(&field.rendered_prefix);
            util::write_json_value(out, value)?;
        }
        assert!(
            iter.next().is_none(),
            "compiled object schema received more than {} values",
            self.fields.len()
        );
        out.push(b'}');
        Ok(())
    }
}

impl CompiledRowSchema {
    #[must_use]
    pub fn new(keys: &[&str]) -> Self {
        let object = CompiledObjectSchema::new(keys);
        let row_capacity_hint = object.capacity_hint;
        Self {
            object,
            row_capacity_hint,
        }
    }

    #[must_use]
    pub fn object_schema(&self) -> &CompiledObjectSchema {
        &self.object
    }

    pub fn to_json_string<'a, R, I>(&self, rows: R) -> Result<String, JsonError>
    where
        R: IntoIterator<Item = I>,
        I: IntoIterator<Item = &'a JsonValue>,
    {
        let iter = rows.into_iter();
        let (lower, _) = iter.size_hint();
        let mut out = Vec::with_capacity(2 + lower.saturating_mul(self.row_capacity_hint + 1));
        self.write_json_bytes_from_iter(&mut out, iter)?;
        Ok(String::from_utf8(out).expect("JSON serialization produced invalid UTF-8"))
    }

    pub fn write_json_bytes<'a, R, I>(&self, out: &mut Vec<u8>, rows: R) -> Result<(), JsonError>
    where
        R: IntoIterator<Item = I>,
        I: IntoIterator<Item = &'a JsonValue>,
    {
        self.write_json_bytes_from_iter(out, rows.into_iter())
    }

    pub fn write_row_json_bytes<'a, I>(&self, out: &mut Vec<u8>, values: I) -> Result<(), JsonError>
    where
        I: IntoIterator<Item = &'a JsonValue>,
    {
        self.object.write_json_bytes(out, values)
    }

    fn write_json_bytes_from_iter<'a, R, I>(
        &self,
        out: &mut Vec<u8>,
        mut rows: R,
    ) -> Result<(), JsonError>
    where
        R: Iterator<Item = I>,
        I: IntoIterator<Item = &'a JsonValue>,
    {
        out.push(b'[');
        if let Some(first_row) = rows.next() {
            self.object.write_json_bytes(out, first_row)?;
            for row in rows {
                out.push(b',');
                self.object.write_json_bytes(out, row)?;
            }
        }
        out.push(b']');
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{TapeTokenKind, parse_json_tape};
    use alloc::{string::String, vec, vec::Vec};

    #[test]
    fn tape_value_scalar_accessors() {
        let input = r#"{"s":"edge","t":true,"f":false,"i":-7,"u":42,"float":1.5}"#;
        let tape = parse_json_tape(input).unwrap();
        let root = tape.root(input).unwrap();

        assert_eq!(root.get("t").unwrap().as_bool(), Some(true));
        assert_eq!(root.get("f").unwrap().as_bool(), Some(false));
        assert_eq!(root.get("i").unwrap().as_i64(), Some(-7));
        assert_eq!(root.get("u").unwrap().as_u64(), Some(42));
        assert_eq!(root.get("float").unwrap().as_i64(), None);
        assert_eq!(root.get("float").unwrap().as_u64(), None);
        assert_eq!(root.get_string("s"), Some(String::from("edge")));
        assert_eq!(
            root.optional_string("s").unwrap(),
            Some(String::from("edge"))
        );
        assert_eq!(root.optional_string("missing").unwrap(), None);
        assert!(root.optional_string("u").is_err());
        assert_eq!(root.get_f64("float"), Some(1.5));
        assert_eq!(root.optional_bool("t").unwrap(), Some(true));
        assert_eq!(root.optional_i64("i").unwrap(), Some(-7));
        assert_eq!(root.optional_i32("i").unwrap(), Some(-7));
        assert_eq!(root.optional_u64("u").unwrap(), Some(42));
        assert_eq!(root.optional_u32("u").unwrap(), Some(42));
        assert_eq!(root.optional_usize("u").unwrap(), Some(42));
        assert_eq!(root.optional_f64("float").unwrap(), Some(1.5));
        assert_eq!(root.required_bool("t").unwrap(), true);
        assert_eq!(root.required_i64("i").unwrap(), -7);
        assert_eq!(root.required_u64("u").unwrap(), 42);
        assert_eq!(root.required_i32("i").unwrap(), -7);
        assert_eq!(root.required_u32("u").unwrap(), 42);
        assert_eq!(root.required_usize("u").unwrap(), 42);
    }

    #[test]
    fn tape_value_array_items_and_object_fields_are_direct_children() {
        let input = r#"{"items":[1,{"nested":true},3],"other":null}"#;
        let tape = parse_json_tape(input).unwrap();
        let root = tape.root(input).unwrap();
        let items = root.get("items").unwrap().array_items().unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(items[0].as_u64(), Some(1));
        assert_eq!(items[1].kind(), TapeTokenKind::Object);
        assert_eq!(items[2].as_u64(), Some(3));

        let fields = root.object_fields().unwrap();
        assert_eq!(
            fields.iter().map(|(key, _)| *key).collect::<Vec<_>>(),
            vec!["items", "other"]
        );
        assert_eq!(root.required_array("items").unwrap().len(), 3);
        assert!(root.required_object_fields("items").is_err());
    }

    #[test]
    fn tape_value_converts_to_owned_json_value() {
        let input = r#"{"items":[1,{"nested":true},3.5],"other":null}"#;
        let tape = parse_json_tape(input).unwrap();
        let root = tape.root(input).unwrap();
        let value = root.to_json_value().unwrap();

        assert_eq!(value.get_array("items").unwrap().len(), 3);
        assert_eq!(
            value.pointer("/items/1/nested").unwrap().as_bool(),
            Some(true)
        );
        assert_eq!(value.pointer("/items/2").unwrap().as_f64(), Some(3.5));
        assert!(value.required("other").unwrap().is_null());
    }
}
