use crate::prelude::*;

#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::borrow::ToOwned;

#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::string::String;
#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::vec::Vec;

use crate::value::JsonValueError;
use crate::JsonValue;
use core::ops::{Deref, DerefMut};

#[derive(Clone, Debug, PartialEq)]
pub struct Map(pub(crate) Vec<(String, JsonValue)>);

impl Map {
    #[must_use]
    pub fn new() -> Self {
        Self(Vec::new())
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    #[must_use]
    pub fn keys(&self) -> impl ExactSizeIterator<Item = &String> {
        self.0.iter().map(|(key, _)| key)
    }

    #[must_use]
    pub fn values(&self) -> impl ExactSizeIterator<Item = &JsonValue> {
        self.0.iter().map(|(_, value)| value)
    }

    pub fn values_mut(&mut self) -> impl ExactSizeIterator<Item = &mut JsonValue> {
        self.0.iter_mut().map(|(_, value)| value)
    }

    #[must_use]
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &(String, JsonValue)> {
        self.0.iter()
    }

    #[must_use]
    pub fn fields(&self) -> impl ExactSizeIterator<Item = (&str, &JsonValue)> {
        self.0.iter().map(|(key, value)| (key.as_str(), value))
    }

    pub fn iter_mut(&mut self) -> impl ExactSizeIterator<Item = &mut (String, JsonValue)> {
        self.0.iter_mut()
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        self.0
            .iter()
            .find(|(candidate, _)| candidate == key)
            .map(|(_, value)| value)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut JsonValue> {
        self.0
            .iter_mut()
            .find(|(candidate, _)| candidate == key)
            .map(|(_, value)| value)
    }

    pub fn required(&self, key: &str) -> Result<&JsonValue, JsonValueError> {
        self.get(key)
            .ok_or_else(|| JsonValueError::WrongType(format!("missing required field `{key}`")))
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

    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub fn insert(&mut self, key: String, value: JsonValue) -> Option<JsonValue> {
        if let Some((_, existing)) = self.0.iter_mut().find(|(candidate, _)| candidate == &key) {
            return Some(core::mem::replace(existing, value));
        }
        self.0.push((key, value));
        None
    }

    pub fn push_field(&mut self, key: impl Into<String>, value: impl Into<JsonValue>) {
        self.0.push((key.into(), value.into()));
    }

    pub fn push_opt_field(&mut self, key: impl Into<String>, value: Option<impl Into<JsonValue>>) {
        if let Some(value) = value {
            self.push_field(key, value);
        }
    }

    #[must_use]
    pub fn into_vec(self) -> Vec<(String, JsonValue)> {
        self.0
    }

    pub fn remove(&mut self, key: &str) -> Option<JsonValue> {
        self.0
            .iter()
            .position(|(candidate, _)| candidate == key)
            .map(|index| self.0.remove(index).1)
    }

    pub fn append(&mut self, other: &mut Self) {
        self.0.append(&mut other.0);
    }

    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&String, &mut JsonValue) -> bool,
    {
        let mut i = 0;
        while i < self.0.len() {
            let keep = {
                let (key, value) = &mut self.0[i];
                f(key, value)
            };
            if keep {
                i += 1;
            } else {
                self.0.remove(i);
            }
        }
    }

    pub fn swap_remove(&mut self, key: &str) -> Option<JsonValue> {
        self.0
            .iter()
            .position(|(candidate, _)| candidate == key)
            .map(|index| self.0.swap_remove(index).1)
    }

    pub fn shift_insert(
        &mut self,
        index: usize,
        key: String,
        value: JsonValue,
    ) -> Option<JsonValue> {
        if let Some((_, existing)) = self.0.iter_mut().find(|(candidate, _)| candidate == &key) {
            return Some(core::mem::replace(existing, value));
        }
        let index = index.min(self.0.len());
        self.0.insert(index, (key, value));
        None
    }

    pub fn sort_keys(&mut self) {
        self.0.sort_by(|(a, _), (b, _)| a.cmp(b));
    }

    pub fn get_or_insert_null(&mut self, key: &str) -> &mut JsonValue {
        if let Some(pos) = self.0.iter().position(|(candidate, _)| candidate == key) {
            &mut self.0[pos].1
        } else {
            self.0.push((key.to_owned(), JsonValue::Null));
            &mut self.0.last_mut().unwrap().1
        }
    }
}

impl Default for Map {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Vec<(String, JsonValue)>> for Map {
    fn from(value: Vec<(String, JsonValue)>) -> Self {
        Self(value)
    }
}

impl core::iter::FromIterator<(String, JsonValue)> for Map {
    fn from_iter<T: IntoIterator<Item = (String, JsonValue)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl Deref for Map {
    type Target = Vec<(String, JsonValue)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Map {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
