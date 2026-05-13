//! Minimal EdgeRun-owned MCP model surface.

extern crate serde as edgerun_serde;

pub mod model {
    use std::sync::Arc;

    use edgerun_serde::{Deserialize, Serialize};

    pub type JsonObject = edgerun_json::Map<String, edgerun_json::Value>;
    pub type JsonValue = edgerun_json::Value;
    pub type Meta = JsonObject;

    pub fn object(value: edgerun_json::Value) -> JsonObject {
        match value {
            edgerun_json::Value::Object(object) => object,
            _ => JsonObject::new(),
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Tool {
        pub name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(default)]
        pub input_schema: Arc<JsonObject>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub output_schema: Option<Arc<JsonObject>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub annotations: Option<ToolAnnotations>,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct ToolAnnotations {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub read_only_hint: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub destructive_hint: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub idempotent_hint: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub open_world_hint: Option<bool>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum ElicitationAction {
        Accept,
        Decline,
        Cancel,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct CreateElicitationResult {
        pub action: ElicitationAction,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub content: Option<JsonObject>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub meta: Option<Meta>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct CreateElicitationRequestParams {
        pub message: String,
        pub requested_schema: ElicitationSchema,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub meta: Option<Meta>,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct ElicitationSchema {
        #[serde(default)]
        pub properties: JsonObject,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub required: Vec<String>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(tag = "type", rename_all = "lowercase")]
    pub enum PrimitiveSchema {
        String(StringSchema),
        Number(NumberSchema),
        Integer(NumberSchema),
        Boolean(BooleanSchema),
        Enum(EnumSchema),
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct StringSchema {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct NumberSchema {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct BooleanSchema {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct EnumSchema {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(default)]
        pub enum_values: Vec<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(untagged)]
    pub enum NumberOrString {
        Number(i64),
        String(Arc<str>),
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(untagged)]
    pub enum RequestId {
        Number(i64),
        String(Arc<str>),
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct PaginatedRequestParams {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub cursor: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct ReadResourceRequestParams {
        pub uri: String,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct ListResourcesResult {
        #[serde(default)]
        pub resources: Vec<Resource>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub next_cursor: Option<String>,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct ListResourceTemplatesResult {
        #[serde(default)]
        pub resource_templates: Vec<ResourceTemplate>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub next_cursor: Option<String>,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct ReadResourceResult {
        #[serde(default)]
        pub contents: Vec<ResourceContents>,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct Resource {
        #[serde(flatten)]
        pub raw: RawResource,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct ResourceTemplate {
        #[serde(flatten)]
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

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct RawResource {
        pub uri: String,
        pub name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub mime_type: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub annotations: Option<Annotations>,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct RawResourceTemplate {
        pub uri_template: String,
        pub name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub mime_type: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub annotations: Option<Annotations>,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct Annotations {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub audience: Option<Vec<Role>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub priority: Option<f64>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum Role {
        User,
        Assistant,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(tag = "type", rename_all = "lowercase")]
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
}
