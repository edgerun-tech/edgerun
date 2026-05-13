pub use edgerun_schemars_derive::JsonSchema;

pub trait JsonSchema {
    fn is_referenceable() -> bool {
        true
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Owned(Self::schema_name())
    }

    fn schema_name() -> String {
        core::any::type_name::<Self>()
            .rsplit("::")
            .next()
            .unwrap_or("Value")
            .to_string()
    }

    fn json_schema(generator: &mut r#gen::SchemaGenerator) -> schema::Schema {
        generator.default_schema_for(Self::schema_name())
    }
}

pub mod r#gen {
    use crate::JsonSchema;
    use crate::RootSchema;
    use crate::schema::{InstanceType, Metadata, Schema, SchemaObject};

    #[derive(Debug, Clone)]
    pub struct SchemaSettings {
        pub option_nullable: bool,
    }

    impl SchemaSettings {
        pub fn draft07() -> Self {
            Self {
                option_nullable: false,
            }
        }

        pub fn into_generator(self) -> SchemaGenerator {
            SchemaGenerator { settings: self }
        }
    }

    #[derive(Debug, Clone)]
    pub struct SchemaGenerator {
        settings: SchemaSettings,
    }

    impl Default for SchemaGenerator {
        fn default() -> Self {
            SchemaSettings::draft07().into_generator()
        }
    }

    impl SchemaGenerator {
        pub fn default_schema_for(&mut self, title: String) -> Schema {
            Schema::Object(SchemaObject {
                metadata: Some(Box::new(Metadata {
                    title: Some(title),
                    ..Default::default()
                })),
                instance_type: Some(InstanceType::Object.into()),
                ..Default::default()
            })
        }

        pub fn subschema_for<T: JsonSchema + ?Sized>(&mut self) -> Schema {
            T::json_schema(self)
        }

        pub fn into_root_schema_for<T: JsonSchema>(mut self) -> RootSchema {
            RootSchema {
                schema: T::json_schema(&mut self),
                definitions: Default::default(),
                meta_schema: None,
                option_nullable: self.settings.option_nullable,
            }
        }
    }
}

pub mod schema {
    use std::collections::BTreeMap;

    #[derive(Debug, Clone, PartialEq)]
    pub enum EdgeJsonValue {
        Null,
        Bool(bool),
        Number(f64),
        String(String),
        Array(Vec<EdgeJsonValue>),
        Object(Vec<(String, EdgeJsonValue)>),
    }

    impl EdgeJsonValue {
        pub fn empty_object() -> Self {
            Self::Object(Vec::new())
        }

        pub fn array(values: Vec<EdgeJsonValue>) -> Self {
            Self::Array(values)
        }

        pub fn object_from_iter<K, V, I>(entries: I) -> Self
        where
            K: Into<String>,
            V: Into<EdgeJsonValue>,
            I: IntoIterator<Item = (K, V)>,
        {
            Self::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| (key.into(), value.into()))
                    .collect(),
            )
        }

        pub fn array_from_iter<V, I>(values: I) -> Self
        where
            V: Into<EdgeJsonValue>,
            I: IntoIterator<Item = V>,
        {
            Self::Array(values.into_iter().map(Into::into).collect())
        }

        pub fn push_field(&mut self, key: impl Into<String>, value: impl Into<EdgeJsonValue>) {
            match self {
                Self::Object(entries) => entries.push((key.into(), value.into())),
                _ => panic!("push_field called on non-object schema JSON value"),
            }
        }

        pub fn to_json_string(&self) -> String {
            let mut out = String::new();
            self.write_json(&mut out);
            out
        }

        fn write_json(&self, out: &mut String) {
            match self {
                Self::Null => out.push_str("null"),
                Self::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
                Self::Number(value) => out.push_str(&value.to_string()),
                Self::String(value) => write_string(out, value),
                Self::Array(values) => {
                    out.push('[');
                    for (index, value) in values.iter().enumerate() {
                        if index > 0 {
                            out.push(',');
                        }
                        value.write_json(out);
                    }
                    out.push(']');
                }
                Self::Object(entries) => {
                    out.push('{');
                    for (index, (key, value)) in entries.iter().enumerate() {
                        if index > 0 {
                            out.push(',');
                        }
                        write_string(out, key);
                        out.push(':');
                        value.write_json(out);
                    }
                    out.push('}');
                }
            }
        }
    }

    impl From<bool> for EdgeJsonValue {
        fn from(value: bool) -> Self {
            Self::Bool(value)
        }
    }

    impl From<f64> for EdgeJsonValue {
        fn from(value: f64) -> Self {
            Self::Number(value)
        }
    }

    impl From<String> for EdgeJsonValue {
        fn from(value: String) -> Self {
            Self::String(value)
        }
    }

    impl From<&str> for EdgeJsonValue {
        fn from(value: &str) -> Self {
            Self::String(value.to_string())
        }
    }

    fn write_string(out: &mut String, value: &str) {
        out.push('"');
        for ch in value.chars() {
            match ch {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                '\u{08}' => out.push_str("\\b"),
                '\u{0c}' => out.push_str("\\f"),
                ch if ch < ' ' => {
                    use std::fmt::Write as _;
                    let _ = write!(out, "\\u{:04x}", ch as u32);
                }
                ch => out.push(ch),
            }
        }
        out.push('"');
    }

    pub trait ToJsonSchemaValue {
        fn to_json_schema_value(&self) -> EdgeJsonValue;
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum Schema {
        Bool(bool),
        Object(SchemaObject),
    }

    impl From<SchemaObject> for Schema {
        fn from(value: SchemaObject) -> Self {
            Self::Object(value)
        }
    }

    impl ToJsonSchemaValue for Schema {
        fn to_json_schema_value(&self) -> EdgeJsonValue {
            match self {
                Self::Bool(value) => EdgeJsonValue::Bool(*value),
                Self::Object(value) => value.to_json_schema_value(),
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum InstanceType {
        Null,
        Boolean,
        Object,
        Array,
        Number,
        String,
        Integer,
    }

    impl InstanceType {
        fn as_json_name(self) -> &'static str {
            match self {
                Self::Null => "null",
                Self::Boolean => "boolean",
                Self::Object => "object",
                Self::Array => "array",
                Self::Number => "number",
                Self::String => "string",
                Self::Integer => "integer",
            }
        }
    }

    impl ToJsonSchemaValue for InstanceType {
        fn to_json_schema_value(&self) -> EdgeJsonValue {
            EdgeJsonValue::String(self.as_json_name().to_string())
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum SingleOrVec<T> {
        Single(Box<T>),
        Vec(Vec<T>),
    }

    impl<T> From<T> for SingleOrVec<T> {
        fn from(value: T) -> Self {
            Self::Single(Box::new(value))
        }
    }

    impl<T> From<Vec<T>> for SingleOrVec<T> {
        fn from(value: Vec<T>) -> Self {
            Self::Vec(value)
        }
    }

    impl<T: ToJsonSchemaValue> ToJsonSchemaValue for SingleOrVec<T> {
        fn to_json_schema_value(&self) -> EdgeJsonValue {
            match self {
                Self::Single(value) => value.to_json_schema_value(),
                Self::Vec(values) => EdgeJsonValue::array(
                    values
                        .iter()
                        .map(ToJsonSchemaValue::to_json_schema_value)
                        .collect(),
                ),
            }
        }
    }

    #[derive(Debug, Clone, Default, PartialEq)]
    pub struct Metadata {
        pub title: Option<String>,
        pub description: Option<String>,
    }

    impl Metadata {
        fn append_json_fields(&self, object: &mut EdgeJsonValue) {
            if let Some(title) = &self.title {
                object.push_field("title", title.as_str());
            }
            if let Some(description) = &self.description {
                object.push_field("description", description.as_str());
            }
        }
    }

    impl ToJsonSchemaValue for Metadata {
        fn to_json_schema_value(&self) -> EdgeJsonValue {
            let mut object = EdgeJsonValue::empty_object();
            self.append_json_fields(&mut object);
            object
        }
    }

    #[derive(Debug, Clone, Default, PartialEq)]
    pub struct SubschemaValidation {
        pub one_of: Option<Vec<Schema>>,
        pub any_of: Option<Vec<Schema>>,
        pub all_of: Option<Vec<Schema>>,
    }

    impl SubschemaValidation {
        fn append_json_fields(&self, object: &mut EdgeJsonValue) {
            if let Some(one_of) = &self.one_of {
                object.push_field("oneOf", schema_array(one_of));
            }
            if let Some(any_of) = &self.any_of {
                object.push_field("anyOf", schema_array(any_of));
            }
            if let Some(all_of) = &self.all_of {
                object.push_field("allOf", schema_array(all_of));
            }
        }
    }

    impl ToJsonSchemaValue for SubschemaValidation {
        fn to_json_schema_value(&self) -> EdgeJsonValue {
            let mut object = EdgeJsonValue::empty_object();
            self.append_json_fields(&mut object);
            object
        }
    }

    #[derive(Debug, Clone, Default, PartialEq)]
    pub struct SchemaObject {
        pub reference: Option<String>,
        pub metadata: Option<Box<Metadata>>,
        pub instance_type: Option<SingleOrVec<InstanceType>>,
        pub const_value: Option<JsonValue>,
        pub enum_values: Option<Vec<JsonValue>>,
        pub subschemas: Option<Box<SubschemaValidation>>,
        pub properties: BTreeMap<String, Schema>,
        pub required: Vec<String>,
        pub items: Option<Box<Schema>>,
        pub additional_properties: Option<AdditionalProperties>,
    }

    impl ToJsonSchemaValue for SchemaObject {
        fn to_json_schema_value(&self) -> EdgeJsonValue {
            let mut object = EdgeJsonValue::empty_object();
            if let Some(reference) = &self.reference {
                object.push_field("$ref", reference.as_str());
            }
            if let Some(metadata) = &self.metadata {
                metadata.append_json_fields(&mut object);
            }
            if let Some(instance_type) = &self.instance_type {
                object.push_field("type", instance_type.to_json_schema_value());
            }
            if let Some(value) = &self.const_value {
                object.push_field("const", value.to_json_schema_value());
            }
            if let Some(values) = &self.enum_values {
                object.push_field(
                    "enum",
                    EdgeJsonValue::array(
                        values
                            .iter()
                            .map(ToJsonSchemaValue::to_json_schema_value)
                            .collect(),
                    ),
                );
            }
            if let Some(subschemas) = &self.subschemas {
                subschemas.append_json_fields(&mut object);
            }
            if !self.properties.is_empty() {
                object.push_field(
                    "properties",
                    EdgeJsonValue::object_from_iter(
                        self.properties
                            .iter()
                            .map(|(key, value)| (key.clone(), value.to_json_schema_value())),
                    ),
                );
            }
            if !self.required.is_empty() {
                object.push_field(
                    "required",
                    EdgeJsonValue::array_from_iter(
                        self.required.iter().map(|value| value.as_str()),
                    ),
                );
            }
            if let Some(items) = &self.items {
                object.push_field("items", items.to_json_schema_value());
            }
            if let Some(additional_properties) = &self.additional_properties {
                object.push_field(
                    "additionalProperties",
                    additional_properties.to_json_schema_value(),
                );
            }
            object
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum AdditionalProperties {
        Bool(bool),
        Schema(Box<Schema>),
    }

    impl ToJsonSchemaValue for AdditionalProperties {
        fn to_json_schema_value(&self) -> EdgeJsonValue {
            match self {
                Self::Bool(value) => EdgeJsonValue::Bool(*value),
                Self::Schema(value) => value.to_json_schema_value(),
            }
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum JsonValue {
        Null,
        Bool(bool),
        Number(f64),
        String(String),
        Array(Vec<JsonValue>),
        Object(BTreeMap<String, JsonValue>),
    }

    impl ToJsonSchemaValue for JsonValue {
        fn to_json_schema_value(&self) -> EdgeJsonValue {
            match self {
                Self::Null => EdgeJsonValue::Null,
                Self::Bool(value) => EdgeJsonValue::Bool(*value),
                Self::Number(value) => EdgeJsonValue::from(*value),
                Self::String(value) => EdgeJsonValue::String(value.clone()),
                Self::Array(values) => EdgeJsonValue::array(
                    values
                        .iter()
                        .map(ToJsonSchemaValue::to_json_schema_value)
                        .collect(),
                ),
                Self::Object(values) => EdgeJsonValue::object_from_iter(
                    values
                        .iter()
                        .map(|(key, value)| (key.clone(), value.to_json_schema_value())),
                ),
            }
        }
    }

    fn schema_array(values: &[Schema]) -> EdgeJsonValue {
        EdgeJsonValue::array(
            values
                .iter()
                .map(ToJsonSchemaValue::to_json_schema_value)
                .collect(),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RootSchema {
    pub meta_schema: Option<String>,
    pub schema: schema::Schema,
    pub option_nullable: bool,
    pub definitions: std::collections::BTreeMap<String, schema::Schema>,
}

impl RootSchema {
    pub fn to_json_value(&self) -> schema::EdgeJsonValue {
        use schema::ToJsonSchemaValue;

        let mut value = self.schema.to_json_schema_value();
        if let Some(meta_schema) = &self.meta_schema {
            value.push_field("$schema", meta_schema.as_str());
        }
        if !self.definitions.is_empty() {
            value.push_field(
                "definitions",
                schema::EdgeJsonValue::object_from_iter(
                    self.definitions
                        .iter()
                        .map(|(key, value)| (key.clone(), value.to_json_schema_value())),
                ),
            );
        }
        value
    }

    pub fn to_json_string(&self) -> String {
        self.to_json_value().to_json_string()
    }
}

#[macro_export]
macro_rules! schema_for {
    ($ty:ty $(,)?) => {{
        $crate::r#gen::SchemaSettings::draft07()
            .into_generator()
            .into_root_schema_for::<$ty>()
    }};
}

macro_rules! primitive_schema {
    ($ty:ty, $kind:expr) => {
        impl JsonSchema for $ty {
            fn json_schema(_: &mut r#gen::SchemaGenerator) -> schema::Schema {
                schema::Schema::Object(schema::SchemaObject {
                    instance_type: Some($kind.into()),
                    ..Default::default()
                })
            }
        }
    };
}

primitive_schema!(bool, schema::InstanceType::Boolean);
primitive_schema!(String, schema::InstanceType::String);
primitive_schema!(&str, schema::InstanceType::String);
primitive_schema!(u8, schema::InstanceType::Integer);
primitive_schema!(u16, schema::InstanceType::Integer);
primitive_schema!(u32, schema::InstanceType::Integer);
primitive_schema!(u64, schema::InstanceType::Integer);
primitive_schema!(usize, schema::InstanceType::Integer);
primitive_schema!(i8, schema::InstanceType::Integer);
primitive_schema!(i16, schema::InstanceType::Integer);
primitive_schema!(i32, schema::InstanceType::Integer);
primitive_schema!(i64, schema::InstanceType::Integer);
primitive_schema!(isize, schema::InstanceType::Integer);
primitive_schema!(f32, schema::InstanceType::Number);
primitive_schema!(f64, schema::InstanceType::Number);

impl<T: JsonSchema> JsonSchema for Option<T> {
    fn json_schema(generator: &mut r#gen::SchemaGenerator) -> schema::Schema {
        generator.subschema_for::<T>()
    }
}

impl<T: JsonSchema> JsonSchema for Vec<T> {
    fn json_schema(generator: &mut r#gen::SchemaGenerator) -> schema::Schema {
        array_schema::<T>(generator)
    }
}

impl<T: JsonSchema> JsonSchema for std::collections::VecDeque<T> {
    fn json_schema(generator: &mut r#gen::SchemaGenerator) -> schema::Schema {
        array_schema::<T>(generator)
    }
}

impl<T: JsonSchema> JsonSchema for std::collections::BTreeSet<T> {
    fn json_schema(generator: &mut r#gen::SchemaGenerator) -> schema::Schema {
        array_schema::<T>(generator)
    }
}

impl<T: JsonSchema> JsonSchema for std::collections::HashSet<T> {
    fn json_schema(generator: &mut r#gen::SchemaGenerator) -> schema::Schema {
        array_schema::<T>(generator)
    }
}

impl<T: JsonSchema> JsonSchema for Box<T> {
    fn json_schema(generator: &mut r#gen::SchemaGenerator) -> schema::Schema {
        generator.subschema_for::<T>()
    }
}

impl<T: JsonSchema> JsonSchema for std::sync::Arc<T> {
    fn json_schema(generator: &mut r#gen::SchemaGenerator) -> schema::Schema {
        generator.subschema_for::<T>()
    }
}

impl<K, T: JsonSchema> JsonSchema for std::collections::BTreeMap<K, T> {
    fn json_schema(generator: &mut r#gen::SchemaGenerator) -> schema::Schema {
        map_schema::<T>(generator)
    }
}

impl<K, T: JsonSchema> JsonSchema for std::collections::HashMap<K, T> {
    fn json_schema(generator: &mut r#gen::SchemaGenerator) -> schema::Schema {
        map_schema::<T>(generator)
    }
}

impl<'a> JsonSchema for std::borrow::Cow<'a, str> {
    fn json_schema(generator: &mut r#gen::SchemaGenerator) -> schema::Schema {
        generator.subschema_for::<String>()
    }
}

impl JsonSchema for std::path::Path {
    fn json_schema(generator: &mut r#gen::SchemaGenerator) -> schema::Schema {
        generator.subschema_for::<String>()
    }
}

impl JsonSchema for std::path::PathBuf {
    fn json_schema(generator: &mut r#gen::SchemaGenerator) -> schema::Schema {
        generator.subschema_for::<String>()
    }
}

impl JsonSchema for std::time::Duration {
    fn json_schema(_: &mut r#gen::SchemaGenerator) -> schema::Schema {
        schema::Schema::Object(schema::SchemaObject {
            instance_type: Some(schema::InstanceType::Number.into()),
            ..Default::default()
        })
    }
}

impl<T: JsonSchema, E: JsonSchema> JsonSchema for Result<T, E> {
    fn json_schema(generator: &mut r#gen::SchemaGenerator) -> schema::Schema {
        schema::Schema::Object(schema::SchemaObject {
            subschemas: Some(Box::new(schema::SubschemaValidation {
                any_of: Some(vec![
                    generator.subschema_for::<T>(),
                    generator.subschema_for::<E>(),
                ]),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}

macro_rules! nonzero_schema {
    ($($ty:ty),* $(,)?) => {
        $(
            impl JsonSchema for $ty {
                fn json_schema(_: &mut r#gen::SchemaGenerator) -> schema::Schema {
                    schema::Schema::Object(schema::SchemaObject {
                        instance_type: Some(schema::InstanceType::Integer.into()),
                        ..Default::default()
                    })
                }
            }
        )*
    };
}

nonzero_schema!(
    std::num::NonZeroU8,
    std::num::NonZeroU16,
    std::num::NonZeroU32,
    std::num::NonZeroU64,
    std::num::NonZeroUsize,
    std::num::NonZeroI8,
    std::num::NonZeroI16,
    std::num::NonZeroI32,
    std::num::NonZeroI64,
    std::num::NonZeroIsize,
);

impl JsonSchema for () {}
impl<T: JsonSchema, const N: usize> JsonSchema for [T; N] {
    fn json_schema(generator: &mut r#gen::SchemaGenerator) -> schema::Schema {
        array_schema::<T>(generator)
    }
}
impl<A: JsonSchema, B: JsonSchema> JsonSchema for (A, B) {}

fn array_schema<T: JsonSchema>(generator: &mut r#gen::SchemaGenerator) -> schema::Schema {
    schema::Schema::Object(schema::SchemaObject {
        instance_type: Some(schema::InstanceType::Array.into()),
        items: Some(Box::new(generator.subschema_for::<T>())),
        ..Default::default()
    })
}

fn map_schema<T: JsonSchema>(generator: &mut r#gen::SchemaGenerator) -> schema::Schema {
    schema::Schema::Object(schema::SchemaObject {
        instance_type: Some(schema::InstanceType::Object.into()),
        additional_properties: Some(schema::AdditionalProperties::Schema(Box::new(
            generator.subschema_for::<T>(),
        ))),
        ..Default::default()
    })
}
