pub use edgerun_schemars_derive::JsonSchema;
use serde::Serialize;

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

    use serde::Serialize;

    #[derive(Debug, Clone, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum Schema {
        Bool(bool),
        Object(SchemaObject),
    }

    impl From<SchemaObject> for Schema {
        fn from(value: SchemaObject) -> Self {
            Self::Object(value)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "lowercase")]
    pub enum InstanceType {
        Null,
        Boolean,
        Object,
        Array,
        Number,
        String,
        Integer,
    }

    #[derive(Debug, Clone, PartialEq, Serialize)]
    #[serde(untagged)]
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

    #[derive(Debug, Clone, Default, PartialEq, Serialize)]
    pub struct Metadata {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SubschemaValidation {
        #[serde(rename = "oneOf", skip_serializing_if = "Option::is_none")]
        pub one_of: Option<Vec<Schema>>,
        #[serde(rename = "anyOf", skip_serializing_if = "Option::is_none")]
        pub any_of: Option<Vec<Schema>>,
        #[serde(rename = "allOf", skip_serializing_if = "Option::is_none")]
        pub all_of: Option<Vec<Schema>>,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize)]
    pub struct SchemaObject {
        #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
        pub reference: Option<String>,
        #[serde(flatten, skip_serializing_if = "Option::is_none")]
        pub metadata: Option<Box<Metadata>>,
        #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
        pub instance_type: Option<SingleOrVec<InstanceType>>,
        #[serde(rename = "const", skip_serializing_if = "Option::is_none")]
        pub const_value: Option<JsonValue>,
        #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
        pub enum_values: Option<Vec<JsonValue>>,
        #[serde(flatten, skip_serializing_if = "Option::is_none")]
        pub subschemas: Option<Box<SubschemaValidation>>,
        #[serde(rename = "properties", skip_serializing_if = "BTreeMap::is_empty")]
        pub properties: BTreeMap<String, Schema>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub required: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub items: Option<Box<Schema>>,
        #[serde(
            rename = "additionalProperties",
            skip_serializing_if = "Option::is_none"
        )]
        pub additional_properties: Option<AdditionalProperties>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum AdditionalProperties {
        Bool(bool),
        Schema(Box<Schema>),
    }
    #[derive(Debug, Clone, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum JsonValue {
        Null,
        Bool(bool),
        Number(f64),
        String(String),
        Array(Vec<JsonValue>),
        Object(BTreeMap<String, JsonValue>),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RootSchema {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta_schema: Option<String>,
    #[serde(flatten)]
    pub schema: schema::Schema,
    #[serde(skip)]
    pub option_nullable: bool,
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub definitions: std::collections::BTreeMap<String, schema::Schema>,
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
