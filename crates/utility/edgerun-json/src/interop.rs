use crate::{JsonNumber, JsonValue, Map};

#[cfg(any(feature = "schemars", feature = "ts-rs"))]
use alloc::borrow::Cow;
#[cfg(any(feature = "serde", feature = "ts-rs"))]
use alloc::format;
#[cfg(any(feature = "serde", feature = "ts-rs"))]
use alloc::string::{String, ToString};
#[cfg(feature = "serde")]
use alloc::vec::Vec;
#[cfg(feature = "serde")]
use serde::de::IntoDeserializer;

#[cfg(feature = "serde")]
pub(crate) fn to_json_value_from_serde<T: serde::Serialize + ?Sized>(
    value: &T,
) -> Result<JsonValue, crate::JsonError> {
    value.serialize(JsonSerializer)
}

#[cfg(feature = "serde")]
pub(crate) fn from_json_value_with_serde<T>(value: JsonValue) -> Result<T, crate::JsonError>
where
    T: serde::de::DeserializeOwned,
{
    T::deserialize(value)
}

#[cfg(feature = "serde")]
struct JsonSerializer;

#[cfg(feature = "serde")]
impl serde::Serializer for JsonSerializer {
    type Ok = JsonValue;
    type Error = crate::JsonError;
    type SerializeSeq = JsonSeqSerializer;
    type SerializeTuple = JsonSeqSerializer;
    type SerializeTupleStruct = JsonSeqSerializer;
    type SerializeTupleVariant = JsonTupleVariantSerializer;
    type SerializeMap = JsonMapSerializer;
    type SerializeStruct = JsonMapSerializer;
    type SerializeStructVariant = JsonStructVariantSerializer;

    fn serialize_bool(self, value: bool) -> Result<Self::Ok, Self::Error> {
        Ok(JsonValue::Bool(value))
    }

    fn serialize_i8(self, value: i8) -> Result<Self::Ok, Self::Error> {
        Ok(JsonValue::from(value))
    }

    fn serialize_i16(self, value: i16) -> Result<Self::Ok, Self::Error> {
        Ok(JsonValue::from(value))
    }

    fn serialize_i32(self, value: i32) -> Result<Self::Ok, Self::Error> {
        Ok(JsonValue::from(value))
    }

    fn serialize_i64(self, value: i64) -> Result<Self::Ok, Self::Error> {
        Ok(JsonValue::from(value))
    }

    fn serialize_i128(self, value: i128) -> Result<Self::Ok, Self::Error> {
        Ok(JsonValue::from(value))
    }

    fn serialize_u8(self, value: u8) -> Result<Self::Ok, Self::Error> {
        Ok(JsonValue::from(value))
    }

    fn serialize_u16(self, value: u16) -> Result<Self::Ok, Self::Error> {
        Ok(JsonValue::from(value))
    }

    fn serialize_u32(self, value: u32) -> Result<Self::Ok, Self::Error> {
        Ok(JsonValue::from(value))
    }

    fn serialize_u64(self, value: u64) -> Result<Self::Ok, Self::Error> {
        Ok(JsonValue::from(value))
    }

    fn serialize_u128(self, value: u128) -> Result<Self::Ok, Self::Error> {
        Ok(JsonValue::from(value))
    }

    fn serialize_f32(self, value: f32) -> Result<Self::Ok, Self::Error> {
        JsonNumber::from_f64(f64::from(value))
            .map(JsonValue::Number)
            .ok_or(crate::JsonError::NonFiniteNumber)
    }

    fn serialize_f64(self, value: f64) -> Result<Self::Ok, Self::Error> {
        JsonNumber::from_f64(value)
            .map(JsonValue::Number)
            .ok_or(crate::JsonError::NonFiniteNumber)
    }

    fn serialize_char(self, value: char) -> Result<Self::Ok, Self::Error> {
        Ok(JsonValue::String(value.to_string()))
    }

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        Ok(JsonValue::String(value.to_string()))
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(JsonValue::array_from_iter(value.iter().copied()))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(JsonValue::Null)
    }

    fn serialize_some<T: serde::Serialize + ?Sized>(
        self,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(JsonValue::Null)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(JsonValue::Null)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(JsonValue::String(variant.to_string()))
    }

    fn serialize_newtype_struct<T: serde::Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: serde::Serialize + ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        let mut object: Map = Map::new();
        object.push_field(variant, value.serialize(JsonSerializer)?);
        Ok(object.into())
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(JsonSeqSerializer {
            values: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(JsonTupleVariantSerializer {
            variant,
            values: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(JsonMapSerializer {
            object: Map::with_capacity(len.unwrap_or(0)),
            next_key: None,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(JsonStructVariantSerializer {
            variant,
            object: Map::with_capacity(len),
        })
    }
}

#[cfg(feature = "serde")]
struct JsonSeqSerializer {
    values: Vec<JsonValue>,
}

#[cfg(feature = "serde")]
impl serde::ser::SerializeSeq for JsonSeqSerializer {
    type Ok = JsonValue;
    type Error = crate::JsonError;

    fn serialize_element<T: serde::Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.values.push(value.serialize(JsonSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(JsonValue::Array(self.values))
    }
}

#[cfg(feature = "serde")]
impl serde::ser::SerializeTuple for JsonSeqSerializer {
    type Ok = JsonValue;
    type Error = crate::JsonError;

    fn serialize_element<T: serde::Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        serde::ser::SerializeSeq::end(self)
    }
}

#[cfg(feature = "serde")]
impl serde::ser::SerializeTupleStruct for JsonSeqSerializer {
    type Ok = JsonValue;
    type Error = crate::JsonError;

    fn serialize_field<T: serde::Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        serde::ser::SerializeSeq::end(self)
    }
}

#[cfg(feature = "serde")]
struct JsonTupleVariantSerializer {
    variant: &'static str,
    values: Vec<JsonValue>,
}

#[cfg(feature = "serde")]
impl serde::ser::SerializeTupleVariant for JsonTupleVariantSerializer {
    type Ok = JsonValue;
    type Error = crate::JsonError;

    fn serialize_field<T: serde::Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.values.push(value.serialize(JsonSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut object: Map = Map::new();
        object.push_field(self.variant, JsonValue::Array(self.values));
        Ok(object.into())
    }
}

#[cfg(feature = "serde")]
struct JsonMapSerializer {
    object: Map,
    next_key: Option<String>,
}

#[cfg(feature = "serde")]
impl serde::ser::SerializeMap for JsonMapSerializer {
    type Ok = JsonValue;
    type Error = crate::JsonError;

    fn serialize_key<T: serde::Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Self::Error> {
        let key = key.serialize(JsonSerializer)?;
        let key = match key {
            JsonValue::String(value) => value,
            other => other.to_json_string()?,
        };
        self.next_key = Some(key);
        Ok(())
    }

    fn serialize_value<T: serde::Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        let key = self
            .next_key
            .take()
            .ok_or_else(|| crate::JsonError::Message("missing serialized map key".to_string()))?;
        self.object
            .push_field(key, value.serialize(JsonSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.object.into())
    }
}

#[cfg(feature = "serde")]
impl serde::ser::SerializeStruct for JsonMapSerializer {
    type Ok = JsonValue;
    type Error = crate::JsonError;

    fn serialize_field<T: serde::Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.object
            .push_field(key, value.serialize(JsonSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.object.into())
    }
}

#[cfg(feature = "serde")]
struct JsonStructVariantSerializer {
    variant: &'static str,
    object: Map,
}

#[cfg(feature = "serde")]
impl serde::ser::SerializeStructVariant for JsonStructVariantSerializer {
    type Ok = JsonValue;
    type Error = crate::JsonError;

    fn serialize_field<T: serde::Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.object
            .push_field(key, value.serialize(JsonSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut object: Map = Map::new();
        object.push_field(self.variant, JsonValue::Object(self.object));
        Ok(object.into())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserializer<'de> for JsonValue {
    type Error = crate::JsonError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            JsonValue::Null => visitor.visit_unit(),
            JsonValue::Bool(value) => visitor.visit_bool(value),
            JsonValue::Number(JsonNumber::I64(value)) => visitor.visit_i64(value),
            JsonValue::Number(JsonNumber::U64(value)) => visitor.visit_u64(value),
            JsonValue::Number(JsonNumber::F64(value)) => visitor.visit_f64(value),
            JsonValue::String(value) => visitor.visit_string(value),
            JsonValue::Array(values) => visitor.visit_seq(JsonSeqAccess {
                iter: values.into_iter(),
            }),
            JsonValue::Object(object) => visitor.visit_map(JsonObjectAccess {
                iter: object.into_vec().into_iter(),
                pending_value: None,
            }),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            JsonValue::Null => visitor.visit_none(),
            value => visitor.visit_some(value),
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self {
            JsonValue::String(value) => visitor.visit_enum(value.into_deserializer()),
            value => value.deserialize_any(visitor),
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes
        byte_buf unit unit_struct newtype_struct seq tuple tuple_struct map struct
        identifier ignored_any
    }
}

#[cfg(feature = "serde")]
struct JsonSeqAccess {
    iter: alloc::vec::IntoIter<JsonValue>,
}

#[cfg(feature = "serde")]
impl<'de> serde::de::SeqAccess<'de> for JsonSeqAccess {
    type Error = crate::JsonError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        self.iter
            .next()
            .map(|value| seed.deserialize(value))
            .transpose()
    }
}

#[cfg(feature = "serde")]
struct JsonObjectAccess {
    iter: alloc::vec::IntoIter<(String, JsonValue)>,
    pending_value: Option<JsonValue>,
}

#[cfg(feature = "serde")]
impl<'de> serde::de::MapAccess<'de> for JsonObjectAccess {
    type Error = crate::JsonError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        let Some((key, value)) = self.iter.next() else {
            return Ok(None);
        };
        self.pending_value = Some(value);
        seed.deserialize(key.into_deserializer()).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let value = self
            .pending_value
            .take()
            .ok_or_else(|| crate::JsonError::Message("missing JSON object value".to_string()))?;
        seed.deserialize(value)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for JsonNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::I64(value) => serializer.serialize_i64(*value),
            Self::U64(value) => serializer.serialize_u64(*value),
            Self::F64(value) => serializer.serialize_f64(*value),
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for JsonNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(JsonNumberVisitor)
    }
}

#[cfg(feature = "serde")]
struct JsonNumberVisitor;

#[cfg(feature = "serde")]
impl<'de> serde::de::Visitor<'de> for JsonNumberVisitor {
    type Value = JsonNumber;

    fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("a finite JSON number")
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(JsonNumber::from(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(JsonNumber::from(value))
    }

    fn visit_i128<E>(self, value: i128) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        JsonNumber::from_i128(value)
            .ok_or_else(|| E::custom("integer is outside edgerun-json number range"))
    }

    fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        JsonNumber::from_u128(value)
            .ok_or_else(|| E::custom("integer is outside edgerun-json number range"))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        JsonNumber::from_f64(value).ok_or_else(|| E::custom("JSON number must be finite"))
    }
}

#[cfg(feature = "serde")]
impl<K, V> serde::Serialize for Map<K, V> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(self.fields().len()))?;
        for (key, value) in self.fields() {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

#[cfg(feature = "serde")]
impl<'de, K, V> serde::Deserialize<'de> for Map<K, V> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer
            .deserialize_map(JsonMapVisitor)
            .map(|map: Map| map.into_vec().into())
    }
}

#[cfg(feature = "serde")]
struct JsonMapVisitor;

#[cfg(feature = "serde")]
impl<'de> serde::de::Visitor<'de> for JsonMapVisitor {
    type Value = Map;

    fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("a JSON object")
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut entries = Vec::with_capacity(access.size_hint().unwrap_or(0));
        while let Some((key, value)) = access.next_entry::<String, JsonValue>()? {
            entries.push((key, value));
        }
        Ok(entries.into())
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for JsonValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Bool(value) => serializer.serialize_bool(*value),
            Self::Number(value) => value.serialize(serializer),
            Self::String(value) => serializer.serialize_str(value),
            Self::Array(values) => {
                use serde::ser::SerializeSeq;

                let mut seq = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    seq.serialize_element(value)?;
                }
                seq.end()
            }
            Self::Object(value) => value.serialize(serializer),
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for JsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(JsonValueVisitor)
    }
}

#[cfg(feature = "serde")]
struct JsonValueVisitor;

#[cfg(feature = "serde")]
impl<'de> serde::de::Visitor<'de> for JsonValueVisitor {
    type Value = JsonValue;

    fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("any JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(JsonValue::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(JsonValue::Number(JsonNumber::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(JsonValue::Number(JsonNumber::from(value)))
    }

    fn visit_i128<E>(self, value: i128) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        JsonNumber::from_i128(value)
            .map(JsonValue::Number)
            .ok_or_else(|| E::custom("integer is outside edgerun-json number range"))
    }

    fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        JsonNumber::from_u128(value)
            .map(JsonValue::Number)
            .ok_or_else(|| E::custom("integer is outside edgerun-json number range"))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        JsonNumber::from_f64(value)
            .map(JsonValue::Number)
            .ok_or_else(|| E::custom("JSON number must be finite"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(JsonValue::String(value.to_string()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(JsonValue::String(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(JsonValue::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(JsonValue::Null)
    }

    fn visit_seq<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut values = Vec::with_capacity(access.size_hint().unwrap_or(0));
        while let Some(value) = access.next_element::<JsonValue>()? {
            values.push(value);
        }
        Ok(JsonValue::Array(values))
    }

    fn visit_map<A>(self, access: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        JsonMapVisitor.visit_map(access).map(JsonValue::Object)
    }
}

#[cfg(feature = "schemars")]
impl schemars::JsonSchema for JsonValue {
    fn is_referenceable() -> bool {
        false
    }

    fn schema_name() -> String {
        "AnyValue".to_string()
    }

    fn schema_id() -> Cow<'static, str> {
        Cow::Borrowed("AnyValue")
    }

    fn json_schema(_: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        schemars::schema::Schema::Bool(true)
    }
}

#[cfg(feature = "schemars")]
impl<K, V> schemars::JsonSchema for Map<K, V> {
    fn is_referenceable() -> bool {
        false
    }

    fn schema_name() -> String {
        "JsonObject".to_string()
    }

    fn schema_id() -> Cow<'static, str> {
        Cow::Borrowed("JsonObject")
    }

    fn json_schema(_: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        schemars::schema::Schema::Bool(true)
    }
}

#[cfg(feature = "schemars")]
impl schemars::JsonSchema for JsonNumber {
    fn is_referenceable() -> bool {
        false
    }

    fn schema_name() -> String {
        "Number".to_string()
    }

    fn schema_id() -> Cow<'static, str> {
        Cow::Borrowed("Number")
    }

    fn json_schema(_: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        schemars::schema::SchemaObject {
            instance_type: Some(schemars::schema::InstanceType::Number.into()),
            ..Default::default()
        }
        .into()
    }
}

#[cfg(feature = "ts-rs")]
impl ts_rs::TS for JsonValue {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn name() -> String {
        "JsonValue".to_string()
    }

    fn inline() -> String {
        "null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue }".to_string()
    }

    fn inline_flattened() -> String {
        panic!("{} cannot be flattened", <Self as ts_rs::TS>::name())
    }

    fn decl() -> String {
        format!("type {} = {};", Self::name(), Self::inline())
    }

    fn decl_concrete() -> String {
        Self::decl()
    }
}

#[cfg(feature = "ts-rs")]
impl<K, V> ts_rs::TS for Map<K, V> {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn name() -> String {
        "JsonObject".to_string()
    }

    fn inline() -> String {
        "{ [key: string]: JsonValue }".to_string()
    }

    fn inline_flattened() -> String {
        panic!("{} cannot be flattened", <Self as ts_rs::TS>::name())
    }

    fn decl() -> String {
        format!("type {} = {};", Self::name(), Self::inline())
    }

    fn decl_concrete() -> String {
        Self::decl()
    }
}

#[cfg(feature = "ts-rs")]
impl ts_rs::TS for JsonNumber {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn name() -> String {
        "number".to_string()
    }

    fn inline() -> String {
        Self::name()
    }

    fn inline_flattened() -> String {
        panic!("{} cannot be flattened", <Self as ts_rs::TS>::name())
    }

    fn decl() -> String {
        panic!("{} cannot be declared", <Self as ts_rs::TS>::name())
    }

    fn decl_concrete() -> String {
        Self::decl()
    }
}
