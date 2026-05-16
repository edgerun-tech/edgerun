//! Types used when representing Model Context Protocol (MCP) values inside the
//! Codex protocol.
//!
//! We intentionally keep these types JSON-schema friendly via `schemars` so
//! they can be embedded in Codex's own protocol structures.
use edgerun_json::FromJson;
use edgerun_json::Map;
use edgerun_json::ToJson;
use edgerun_json::Value;
use schemars::JsonSchema;

/// ID of a request, which can be either a string or an integer.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson,
)]
#[schemars(untagged)]
pub enum RequestId {
    String(String),
    Integer(i64),
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestId::String(s) => f.write_str(s),
            RequestId::Integer(i) => i.fmt(f),
        }
    }
}

/// Definition for a tool the client can call.
#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct Tool {
    pub name: String,
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: edgerun_json::Value,
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<edgerun_json::Value>,
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub annotations: Option<edgerun_json::Value>,
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub icons: Option<Vec<edgerun_json::Value>>,
    #[schemars(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<edgerun_json::Value>,
}

/// A known resource that the server is capable of reading.
#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct Resource {
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub annotations: Option<edgerun_json::Value>,
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    pub name: String,
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub uri: String,
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub icons: Option<Vec<edgerun_json::Value>>,
    #[schemars(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<edgerun_json::Value>,
}

/// Contents returned when reading a resource from an MCP server.
#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(untagged)]
pub enum ResourceContent {
    #[schemars(rename_all = "camelCase")]
    Text {
        /// The URI of this resource.
        uri: String,
        #[schemars(default, skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
        text: String,
        #[schemars(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
        meta: Option<edgerun_json::Value>,
    },
    #[schemars(rename_all = "camelCase")]
    Blob {
        /// The URI of this resource.
        uri: String,
        #[schemars(default, skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
        blob: String,
        #[schemars(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
        meta: Option<edgerun_json::Value>,
    },
}

/// A template description for resources available on the server.
#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct ResourceTemplate {
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub annotations: Option<edgerun_json::Value>,
    pub uri_template: String,
    pub name: String,
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// The server's response to a tool call.
#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct CallToolResult {
    pub content: Vec<edgerun_json::Value>,
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<edgerun_json::Value>,
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    #[schemars(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<edgerun_json::Value>,
}

impl ToJson for CallToolResult {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("content", self.content.clone());
        object.push_opt_field("structuredContent", self.structured_content.clone());
        object.push_opt_field("isError", self.is_error);
        object.push_opt_field("_meta", self.meta.clone());
        Value::Object(object)
    }
}

// === Adapter helpers ===
//
// These conversions intentionally live in `codex-protocol` so other crates can convert
// “wire-shaped” MCP JSON into our JSON-schema-friendly protocol types without depending on
// `mcp-types` or a third-party JSON bridge.

fn lossy_i64(value: edgerun_json::Value) -> Option<i64> {
    let number = edgerun_json::JsonNumber::from_json(value).ok()?;
    number
        .as_i64()
        .or_else(|| number.as_u64().and_then(|value| i64::try_from(value).ok()))
}

impl Tool {
    pub fn from_mcp_value(
        value: edgerun_json::Value,
    ) -> Result<Self, edgerun_json::JsonValueError> {
        let mut object = value.into_object("Tool")?;
        Ok(Self {
            name: object.take_required("name")?,
            title: object.take_optional("title")?,
            description: object.take_optional("description")?,
            input_schema: object
                .take_optional_any(&["inputSchema", "input_schema"])?
                .unwrap_or(edgerun_json::Value::Null),
            output_schema: object.take_optional_any(&["outputSchema", "output_schema"])?,
            annotations: object.take_optional("annotations")?,
            icons: object.take_optional("icons")?,
            meta: object.take_optional("_meta")?,
        })
    }
}

impl Resource {
    pub fn from_mcp_value(
        value: edgerun_json::Value,
    ) -> Result<Self, edgerun_json::JsonValueError> {
        let mut object = value.into_object("Resource")?;
        let size = object.remove("size").and_then(lossy_i64);
        Ok(Self {
            annotations: object.take_optional("annotations")?,
            description: object.take_optional("description")?,
            mime_type: object.take_optional_any(&["mimeType", "mime_type"])?,
            name: object.take_required("name")?,
            size,
            title: object.take_optional("title")?,
            uri: object.take_required("uri")?,
            icons: object.take_optional("icons")?,
            meta: object.take_optional("_meta")?,
        })
    }
}

impl ResourceTemplate {
    pub fn from_mcp_value(
        value: edgerun_json::Value,
    ) -> Result<Self, edgerun_json::JsonValueError> {
        let mut object = value.into_object("ResourceTemplate")?;
        Ok(Self {
            annotations: object.take_optional("annotations")?,
            uri_template: object.take_required_any(&["uriTemplate", "uri_template"])?,
            name: object.take_required("name")?,
            title: object.take_optional("title")?,
            description: object.take_optional("description")?,
            mime_type: object.take_optional_any(&["mimeType", "mime_type"])?,
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn resource_size_deserializes_without_narrowing() {
        let resource = edgerun_json::json!({
            "name": "big",
            "uri": "file:///tmp/big",
            "size": 5_000_000_000u64,
        });

        let parsed = Resource::from_mcp_value(resource).expect("should deserialize");
        assert_eq!(parsed.size, Some(5_000_000_000));

        let resource = edgerun_json::json!({
            "name": "negative",
            "uri": "file:///tmp/negative",
            "size": -1,
        });

        let parsed = Resource::from_mcp_value(resource).expect("should deserialize");
        assert_eq!(parsed.size, Some(-1));

        let resource = edgerun_json::json!({
            "name": "too_big_for_i64",
            "uri": "file:///tmp/too_big_for_i64",
            "size": 18446744073709551615u64,
        });

        let parsed = Resource::from_mcp_value(resource).expect("should deserialize");
        assert_eq!(parsed.size, None);
    }
}
