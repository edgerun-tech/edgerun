//! We do not do true JSON-RPC 2.0, as we neither send nor expect the
//! "jsonrpc": "2.0" field.

use codex_protocol::protocol::W3cTraceContext;
use edgerun_json::FromJson;
use edgerun_json::JsonValue;
use edgerun_json::JsonValueError;
use edgerun_json::ToJson;
use schemars::JsonSchema;
use std::fmt;

pub const JSONRPC_VERSION: &str = "2.0";

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Hash, Eq, JsonSchema)]
#[serde(untagged)]
pub enum RequestId {
    String(String),
    Integer(i64),
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(value) => f.write_str(value),
            Self::Integer(value) => write!(f, "{value}"),
        }
    }
}

impl ToJson for RequestId {
    fn to_json(&self) -> JsonValue {
        match self {
            Self::String(value) => value.to_json(),
            Self::Integer(value) => value.to_json(),
        }
    }
}

impl FromJson for RequestId {
    fn from_json(value: JsonValue) -> std::result::Result<Self, JsonValueError> {
        match value {
            JsonValue::String(value) => Ok(Self::String(value)),
            JsonValue::Number(number) => i64::from_json(JsonValue::Number(number)).map(Self::Integer),
            other => Err(JsonValueError::WrongType(format!(
                "expected JSON-RPC request id, found {}",
                other.variant_name()
            ))),
        }
    }
}

pub type Result = edgerun_json::Value;

/// Refers to any valid JSON-RPC object that can be decoded off the wire, or encoded to be sent.
#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(untagged)]
pub enum JSONRPCMessage {
    Request(JSONRPCRequest),
    Notification(JSONRPCNotification),
    Response(JSONRPCResponse),
    Error(JSONRPCError),
}

/// A request that expects a response.
#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
pub struct JSONRPCRequest {
    pub id: RequestId,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<edgerun_json::Value>,
    /// Optional W3C Trace Context for distributed tracing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace: Option<W3cTraceContext>,
}

/// A notification which does not expect a response.
#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
pub struct JSONRPCNotification {
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<edgerun_json::Value>,
}

/// A successful (non-error) response to a request.
#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
pub struct JSONRPCResponse {
    pub id: RequestId,
    pub result: Result,
}

/// A response to a request that indicates an error occurred.
#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
pub struct JSONRPCError {
    pub error: JSONRPCErrorError,
    pub id: RequestId,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
pub struct JSONRPCErrorError {
    pub code: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<edgerun_json::Value>,
    pub message: String,
}
