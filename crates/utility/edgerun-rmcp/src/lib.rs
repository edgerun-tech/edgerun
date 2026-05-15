//! Minimal EdgeRun-owned MCP model surface.

#![no_std]

extern crate alloc;

pub mod model {
    use alloc::format;
    use alloc::string::{String, ToString};
    use alloc::sync::Arc;
    use alloc::vec::Vec;

    use edgerun_json::{FromJson, JsonValueError, ToJson};

    pub type JsonObject = edgerun_json::Map<String, edgerun_json::Value>;
    pub type JsonValue = edgerun_json::Value;
    pub type Meta = JsonObject;

    pub fn object(value: edgerun_json::Value) -> JsonObject {
        match value {
            edgerun_json::Value::Object(object) => object,
            _ => JsonObject::new(),
        }
    }

    #[derive(Debug, Clone, PartialEq, ToJson, FromJson)]
    pub struct Tool {
        pub name: String,
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[json(default)]
        pub input_schema: Arc<JsonObject>,
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub output_schema: Option<Arc<JsonObject>>,
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub annotations: Option<ToolAnnotations>,
    }

    #[derive(Debug, Clone, Default, PartialEq, ToJson, FromJson)]
    pub struct ToolAnnotations {
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub title: Option<String>,
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub read_only_hint: Option<bool>,
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub destructive_hint: Option<bool>,
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub idempotent_hint: Option<bool>,
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub open_world_hint: Option<bool>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, ToJson, FromJson)]
    #[json(rename_all = "lowercase")]
    pub enum ElicitationAction {
        Accept,
        Decline,
        Cancel,
    }

    #[derive(Debug, Clone, PartialEq, ToJson, FromJson)]
    pub struct CreateElicitationResult {
        pub action: ElicitationAction,
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub content: Option<JsonObject>,
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub meta: Option<Meta>,
    }

    #[derive(Debug, Clone, PartialEq, ToJson, FromJson)]
    pub struct CreateElicitationRequestParams {
        pub message: String,
        pub requested_schema: ElicitationSchema,
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub meta: Option<Meta>,
    }

    #[derive(Debug, Clone, Default, PartialEq, ToJson, FromJson)]
    pub struct ElicitationSchema {
        #[json(default)]
        pub properties: JsonObject,
        #[json(default, skip_serializing_if = "Vec::is_empty")]
        pub required: Vec<String>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum PrimitiveSchema {
        String(StringSchema),
        Number(NumberSchema),
        Integer(NumberSchema),
        Boolean(BooleanSchema),
        Enum(EnumSchema),
    }

    #[derive(Debug, Clone, Default, PartialEq, ToJson, FromJson)]
    pub struct StringSchema {
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub title: Option<String>,
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
    }

    #[derive(Debug, Clone, Default, PartialEq, ToJson, FromJson)]
    pub struct NumberSchema {
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub title: Option<String>,
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
    }

    #[derive(Debug, Clone, Default, PartialEq, ToJson, FromJson)]
    pub struct BooleanSchema {
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub title: Option<String>,
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
    }

    #[derive(Debug, Clone, Default, PartialEq, ToJson, FromJson)]
    pub struct EnumSchema {
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub title: Option<String>,
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[json(default)]
        pub enum_values: Vec<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum NumberOrString {
        Number(i64),
        String(Arc<str>),
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum RequestId {
        Number(i64),
        String(Arc<str>),
    }

    #[derive(Debug, Clone, Default, PartialEq, ToJson, FromJson)]
    pub struct PaginatedRequestParams {
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub cursor: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, ToJson, FromJson)]
    pub struct ReadResourceRequestParams {
        pub uri: String,
    }

    #[derive(Debug, Clone, Default, PartialEq, ToJson, FromJson)]
    pub struct ListResourcesResult {
        #[json(default)]
        pub resources: Vec<Resource>,
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub next_cursor: Option<String>,
    }

    #[derive(Debug, Clone, Default, PartialEq, ToJson, FromJson)]
    pub struct ListResourceTemplatesResult {
        #[json(default)]
        pub resource_templates: Vec<ResourceTemplate>,
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub next_cursor: Option<String>,
    }

    #[derive(Debug, Clone, Default, PartialEq, ToJson, FromJson)]
    pub struct ReadResourceResult {
        #[json(default)]
        pub contents: Vec<ResourceContents>,
    }

    #[derive(Debug, Clone, Default, PartialEq)]
    pub struct Resource {
        pub raw: RawResource,
    }

    #[derive(Debug, Clone, Default, PartialEq)]
    pub struct ResourceTemplate {
        pub raw: RawResourceTemplate,
    }

    pub trait AnnotateAble {
        fn annotations_mut(&mut self) -> &mut Option<Annotations>;
    }

    impl AnnotateAble for Resource {
        fn annotations_mut(&mut self) -> &mut Option<Annotations> {
            &mut self.raw.annotations
        }
    }

    impl AnnotateAble for ResourceTemplate {
        fn annotations_mut(&mut self) -> &mut Option<Annotations> {
            &mut self.raw.annotations
        }
    }

    #[derive(Debug, Clone, Default, PartialEq, ToJson, FromJson)]
    pub struct RawResource {
        pub uri: String,
        pub name: String,
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub mime_type: Option<String>,
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub annotations: Option<Annotations>,
    }

    #[derive(Debug, Clone, Default, PartialEq, ToJson, FromJson)]
    pub struct RawResourceTemplate {
        pub uri_template: String,
        pub name: String,
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub mime_type: Option<String>,
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub annotations: Option<Annotations>,
    }

    #[derive(Debug, Clone, Default, PartialEq, ToJson, FromJson)]
    pub struct Annotations {
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub audience: Option<Vec<Role>>,
        #[json(default, skip_serializing_if = "Option::is_none")]
        pub priority: Option<f64>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, ToJson, FromJson)]
    #[json(rename_all = "lowercase")]
    pub enum Role {
        User,
        Assistant,
    }

    #[derive(Debug, Clone, PartialEq, ToJson, FromJson)]
    #[json(tag = "type", rename_all = "lowercase")]
    pub enum ResourceContents {
        TextResourceContents { uri: String, text: String },
        BlobResourceContents { uri: String, blob: String },
    }

    impl Default for ResourceContents {
        fn default() -> Self {
            Self::TextResourceContents {
                uri: String::new(),
                text: String::new(),
            }
        }
    }

    impl ToJson for PrimitiveSchema {
        fn to_json(&self) -> JsonValue {
            let (kind, value) = match self {
                Self::String(value) => ("string", value.to_json()),
                Self::Number(value) => ("number", value.to_json()),
                Self::Integer(value) => ("integer", value.to_json()),
                Self::Boolean(value) => ("boolean", value.to_json()),
                Self::Enum(value) => ("enum", value.to_json()),
            };
            let mut object = match value {
                JsonValue::Object(object) => object,
                _ => JsonObject::new(),
            };
            object.push_field("type", JsonValue::String(kind.to_string()));
            JsonValue::Object(object)
        }
    }

    impl FromJson for PrimitiveSchema {
        fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
            let mut object = value.into_object("PrimitiveSchema")?;
            let kind =
                String::from_json(object.remove("type").ok_or_else(|| {
                    JsonValueError::WrongType("missing schema type".to_string())
                })?)?;
            let value = JsonValue::Object(object);
            match kind.as_str() {
                "string" => StringSchema::from_json(value).map(Self::String),
                "number" => NumberSchema::from_json(value).map(Self::Number),
                "integer" => NumberSchema::from_json(value).map(Self::Integer),
                "boolean" => BooleanSchema::from_json(value).map(Self::Boolean),
                "enum" => EnumSchema::from_json(value).map(Self::Enum),
                _ => Err(JsonValueError::WrongType(format!(
                    "unknown schema type `{kind}`"
                ))),
            }
        }
    }

    macro_rules! impl_number_or_string_json {
        ($ty:ty) => {
            impl ToJson for $ty {
                fn to_json(&self) -> JsonValue {
                    match self {
                        Self::Number(value) => value.to_json(),
                        Self::String(value) => value.to_json(),
                    }
                }
            }

            impl FromJson for $ty {
                fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
                    match value {
                        JsonValue::String(value) => Ok(Self::String(Arc::<str>::from(value))),
                        value => i64::from_json(value).map(Self::Number),
                    }
                }
            }
        };
    }

    impl_number_or_string_json!(NumberOrString);
    impl_number_or_string_json!(RequestId);

    impl ToJson for Resource {
        fn to_json(&self) -> JsonValue {
            self.raw.to_json()
        }
    }

    impl FromJson for Resource {
        fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
            RawResource::from_json(value).map(|raw| Self { raw })
        }
    }

    impl ToJson for ResourceTemplate {
        fn to_json(&self) -> JsonValue {
            self.raw.to_json()
        }
    }

    impl FromJson for ResourceTemplate {
        fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
            RawResourceTemplate::from_json(value).map(|raw| Self { raw })
        }
    }
}
