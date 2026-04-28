use crate::prelude::*;

use alloc::collections::BTreeMap;

use crate::{JsonNumber, JsonValue, JsonValueError, Map};

pub trait ToJson {
    fn to_json(&self) -> JsonValue;
}

pub trait FromJson: Sized {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError>;
}

pub fn to_json_value<T: ToJson + ?Sized>(value: &T) -> JsonValue {
    value.to_json()
}

pub fn from_json_value<T: FromJson>(value: JsonValue) -> Result<T, JsonValueError> {
    T::from_json(value)
}

pub fn from_json_str<T: FromJson>(input: &str) -> Result<T, JsonValueError> {
    from_json_value(crate::parse_json(input)?)
}

pub fn from_json_slice<T: FromJson>(input: &[u8]) -> Result<T, JsonValueError> {
    let input = core::str::from_utf8(input).map_err(|_| crate::JsonParseError::InvalidUtf8)?;
    from_json_str(input)
}

macro_rules! impl_to_from_json_number {
    ($ty:ty) => {
        impl ToJson for $ty {
            fn to_json(&self) -> JsonValue {
                JsonValue::from(*self)
            }
        }

        impl FromJson for $ty {
            fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
                Self::try_from(value)
            }
        }
    };
}

impl ToJson for JsonValue {
    fn to_json(&self) -> JsonValue {
        self.clone()
    }
}

impl FromJson for JsonValue {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        Ok(value)
    }
}

impl ToJson for bool {
    fn to_json(&self) -> JsonValue {
        JsonValue::Bool(*self)
    }
}

impl FromJson for bool {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        Self::try_from(value)
    }
}

impl ToJson for str {
    fn to_json(&self) -> JsonValue {
        JsonValue::String(self.to_owned())
    }
}

impl ToJson for &str {
    fn to_json(&self) -> JsonValue {
        JsonValue::String((*self).to_owned())
    }
}

impl ToJson for String {
    fn to_json(&self) -> JsonValue {
        JsonValue::String(self.clone())
    }
}

impl FromJson for String {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        Self::try_from(value)
    }
}

impl ToJson for JsonNumber {
    fn to_json(&self) -> JsonValue {
        JsonValue::Number(self.clone())
    }
}

impl FromJson for JsonNumber {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match value {
            JsonValue::Number(number) => Ok(number),
            other => Err(JsonValueError::WrongType(format!(
                "expected number, found {}",
                other.variant_name()
            ))),
        }
    }
}

impl_to_from_json_number!(i32);
impl_to_from_json_number!(i64);
impl_to_from_json_number!(u32);
impl_to_from_json_number!(u64);
impl_to_from_json_number!(usize);
impl_to_from_json_number!(f64);

impl<T: ToJson> ToJson for Option<T> {
    fn to_json(&self) -> JsonValue {
        self.as_ref().map_or(JsonValue::Null, ToJson::to_json)
    }
}

impl<T: FromJson> FromJson for Option<T> {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match value {
            JsonValue::Null => Ok(None),
            value => T::from_json(value).map(Some),
        }
    }
}

impl<T: ToJson> ToJson for Vec<T> {
    fn to_json(&self) -> JsonValue {
        JsonValue::array_from_iter(self.iter().map(ToJson::to_json))
    }
}

impl<T: FromJson> FromJson for Vec<T> {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match value {
            JsonValue::Array(values) => values.into_iter().map(T::from_json).collect(),
            other => Err(JsonValueError::WrongType(format!(
                "expected array, found {}",
                other.variant_name()
            ))),
        }
    }
}

impl<T: ToJson> ToJson for BTreeMap<String, T> {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(self.len());
        for (key, value) in self {
            object.push_field(key.clone(), value.to_json());
        }
        object.into()
    }
}

impl<T: FromJson> FromJson for BTreeMap<String, T> {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match value {
            JsonValue::Object(object) => object
                .into_vec()
                .into_iter()
                .map(|(key, value)| T::from_json(value).map(|value| (key, value)))
                .collect(),
            other => Err(JsonValueError::WrongType(format!(
                "expected object, found {}",
                other.variant_name()
            ))),
        }
    }
}

impl ToJson for Map {
    fn to_json(&self) -> JsonValue {
        JsonValue::Object(self.clone())
    }
}

impl FromJson for Map {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        Self::try_from(value)
    }
}
