//! Types used when representing Model Context Protocol (MCP) values inside the
//! Codex protocol.
//!
//! We intentionally keep these types TS/JSON-schema friendly (via `ts-rs` and
//! `schemars`) so they can be embedded in Codex's own protocol structures.
use edgerun_json::FromJson;
use edgerun_serde::Deserialize;
use edgerun_serde::Serialize;
use schemars::JsonSchema;
use ts_rs::TS;

/// ID of a request, which can be either a string or an integer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(untagged)]
pub enum RequestId {
    String(String),
    #[ts(type = "number")]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
    pub input_schema: edgerun_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub output_schema: Option<edgerun_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub annotations: Option<edgerun_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub icons: Option<Vec<edgerun_json::Value>>,
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub meta: Option<edgerun_json::Value>,
}

/// A known resource that the server is capable of reading.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub annotations: Option<edgerun_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub mime_type: Option<String>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    #[ts(type = "number")]
    pub size: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title: Option<String>,
    pub uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub icons: Option<Vec<edgerun_json::Value>>,
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub meta: Option<edgerun_json::Value>,
}

/// Contents returned when reading a resource from an MCP server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(untagged)]
pub enum ResourceContent {
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Text {
        /// The URI of this resource.
        uri: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        mime_type: Option<String>,
        text: String,
        #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        meta: Option<edgerun_json::Value>,
    },
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Blob {
        /// The URI of this resource.
        uri: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        mime_type: Option<String>,
        blob: String,
        #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        meta: Option<edgerun_json::Value>,
    },
}

/// A template description for resources available on the server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTemplate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub annotations: Option<edgerun_json::Value>,
    pub uri_template: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub mime_type: Option<String>,
}

/// The server's response to a tool call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct CallToolResult {
    pub content: Vec<edgerun_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub structured_content: Option<edgerun_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub is_error: Option<bool>,
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub meta: Option<edgerun_json::Value>,
}

// === Adapter helpers ===
//
// These conversions intentionally live in `codex-protocol` so other crates can convert
// “wire-shaped” MCP JSON into our TS/JsonSchema-friendly protocol types without depending on
// `mcp-types` or a serde bridge.

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
    use pretty_assertions::assert_eq;

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
