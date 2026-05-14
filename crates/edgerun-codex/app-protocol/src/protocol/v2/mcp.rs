use super::shared::v2_enum_from_core;
use codex_protocol::approvals::ElicitationRequest as CoreElicitationRequest;
use codex_protocol::items::McpToolCallError as CoreMcpToolCallError;
use codex_protocol::mcp::CallToolResult as CoreMcpCallToolResult;
use codex_protocol::mcp::Resource as McpResource;
pub use codex_protocol::mcp::ResourceContent as McpResourceContent;
use codex_protocol::mcp::ResourceTemplate as McpResourceTemplate;
use codex_protocol::mcp::Tool as McpTool;
use edgerun_json::FromJson;
use edgerun_json::JsonValueError;
use edgerun_json::Map;
use edgerun_json::ToJson;
use edgerun_json::Value as JsonValue;
use edgerun_serde::Deserialize;
use edgerun_serde::Serialize;
use schemars::JsonSchema;
use std::collections::BTreeMap;
use ts_rs::TS;

v2_enum_from_core!(
    pub enum McpAuthStatus from codex_protocol::protocol::McpAuthStatus {
        Unsupported,
        NotLoggedIn,
        BearerToken,
        OAuth
    }
);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ListMcpServerStatusParams {
    /// Opaque pagination cursor returned by a previous call.
    #[ts(optional = nullable)]
    pub cursor: Option<String>,
    /// Optional page size; defaults to a server-defined value.
    #[ts(optional = nullable)]
    pub limit: Option<u32>,
    /// Controls how much MCP inventory data to fetch for each server.
    /// Defaults to `Full` when omitted.
    #[ts(optional = nullable)]
    pub detail: Option<McpServerStatusDetail>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum McpServerStatusDetail {
    Full,
    ToolsAndAuthOnly,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpServerStatus {
    pub name: String,
    pub tools: std::collections::HashMap<String, McpTool>,
    pub resources: Vec<McpResource>,
    pub resource_templates: Vec<McpResourceTemplate>,
    pub auth_status: McpAuthStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ListMcpServerStatusResponse {
    pub data: Vec<McpServerStatus>,
    /// Opaque cursor to pass to the next call to continue after the last item.
    /// If None, there are no more items to return.
    pub next_cursor: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpResourceReadParams {
    #[ts(optional = nullable)]
    pub thread_id: Option<String>,
    pub server: String,
    pub uri: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpResourceReadResponse {
    pub contents: Vec<McpResourceContent>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpServerToolCallParams {
    pub thread_id: String,
    pub server: String,
    pub tool: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub arguments: Option<JsonValue>,
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub meta: Option<JsonValue>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpServerToolCallResponse {
    pub content: Vec<JsonValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub structured_content: Option<JsonValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub is_error: Option<bool>,
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub meta: Option<JsonValue>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpToolCallResult {
    // NOTE: `rmcp::model::Content` (and its `RawContent` variants) would be a more precise Rust
    // representation of MCP content blocks. We intentionally use `edgerun_json::Value` here because
    // this crate exports JSON schema + TS types (`schemars`/`ts-rs`), and the rmcp model types
    // aren't set up to be schema/TS friendly (and would introduce heavier coupling to rmcp's Rust
    // representations). Using `JsonValue` keeps the payload wire-shaped and easy to export.
    pub content: Vec<JsonValue>,
    pub structured_content: Option<JsonValue>,
    #[serde(rename = "_meta")]
    #[ts(rename = "_meta")]
    pub meta: Option<JsonValue>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpToolCallError {
    pub message: String,
}

impl From<CoreMcpCallToolResult> for McpServerToolCallResponse {
    fn from(result: CoreMcpCallToolResult) -> Self {
        Self {
            content: result.content,
            structured_content: result.structured_content,
            is_error: result.is_error,
            meta: result.meta,
        }
    }
}

impl From<CoreMcpCallToolResult> for McpToolCallResult {
    fn from(result: CoreMcpCallToolResult) -> Self {
        Self {
            content: result.content,
            structured_content: result.structured_content,
            meta: result.meta,
        }
    }
}

impl From<CoreMcpToolCallError> for McpToolCallError {
    fn from(error: CoreMcpToolCallError) -> Self {
        Self {
            message: error.message,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpServerRefreshParams {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpServerRefreshResponse {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpServerOauthLoginParams {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub scopes: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub timeout_secs: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpServerOauthLoginResponse {
    pub authorization_url: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpToolCallProgressNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpServerOauthLoginCompletedNotification {
    pub name: String,
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum McpServerStartupState {
    Starting,
    Ready,
    Failed,
    Cancelled,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpServerStatusUpdatedNotification {
    pub name: String,
    pub status: McpServerStartupState,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum McpServerElicitationAction {
    Accept,
    Decline,
    Cancel,
}

impl ToJson for McpServerElicitationAction {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            Self::Accept => "accept",
            Self::Decline => "decline",
            Self::Cancel => "cancel",
        })
    }
}

impl McpServerElicitationAction {
    pub fn to_core(self) -> codex_protocol::approvals::ElicitationAction {
        match self {
            Self::Accept => codex_protocol::approvals::ElicitationAction::Accept,
            Self::Decline => codex_protocol::approvals::ElicitationAction::Decline,
            Self::Cancel => codex_protocol::approvals::ElicitationAction::Cancel,
        }
    }
}

impl From<McpServerElicitationAction> for rmcp::model::ElicitationAction {
    fn from(value: McpServerElicitationAction) -> Self {
        match value {
            McpServerElicitationAction::Accept => Self::Accept,
            McpServerElicitationAction::Decline => Self::Decline,
            McpServerElicitationAction::Cancel => Self::Cancel,
        }
    }
}

impl From<rmcp::model::ElicitationAction> for McpServerElicitationAction {
    fn from(value: rmcp::model::ElicitationAction) -> Self {
        match value {
            rmcp::model::ElicitationAction::Accept => Self::Accept,
            rmcp::model::ElicitationAction::Decline => Self::Decline,
            rmcp::model::ElicitationAction::Cancel => Self::Cancel,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpServerElicitationRequestParams {
    pub thread_id: String,
    /// Active Codex turn when this elicitation was observed, if app-server could correlate one.
    ///
    /// This is nullable because MCP models elicitation as a standalone server-to-client request
    /// identified by the MCP server request id. It may be triggered during a turn, but turn
    /// context is app-server correlation rather than part of the protocol identity of the
    /// elicitation itself.
    pub turn_id: Option<String>,
    pub server_name: String,
    #[serde(flatten)]
    pub request: McpServerElicitationRequest,
    // TODO: When core can correlate an elicitation with an MCP tool call, expose the associated
    // McpToolCall item id here as an optional field. The current core event does not carry that
    // association.
}

/// Typed form schema for MCP `elicitation/create` requests.
///
/// This matches the `requestedSchema` shape from the MCP 2025-11-25
/// `ElicitRequestFormParams` schema.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct McpElicitationSchema {
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    #[ts(optional, rename = "$schema")]
    pub schema_uri: Option<String>,
    #[serde(rename = "type")]
    #[ts(rename = "type")]
    pub type_: McpElicitationObjectType,
    pub properties: BTreeMap<String, McpElicitationPrimitiveSchema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub required: Option<Vec<String>>,
}

impl FromJson for McpElicitationSchema {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        parse_mcp_elicitation_schema(value)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "v2/")]
pub enum McpElicitationObjectType {
    Object,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(untagged)]
#[ts(export_to = "v2/")]
pub enum McpElicitationPrimitiveSchema {
    Enum(McpElicitationEnumSchema),
    String(McpElicitationStringSchema),
    Number(McpElicitationNumberSchema),
    Boolean(McpElicitationBooleanSchema),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct McpElicitationStringSchema {
    #[serde(rename = "type")]
    #[ts(rename = "type")]
    pub type_: McpElicitationStringType,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub min_length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub max_length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub format: Option<McpElicitationStringFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub default: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "v2/")]
pub enum McpElicitationStringType {
    String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case", export_to = "v2/")]
pub enum McpElicitationStringFormat {
    Email,
    Uri,
    Date,
    DateTime,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct McpElicitationNumberSchema {
    #[serde(rename = "type")]
    #[ts(rename = "type")]
    pub type_: McpElicitationNumberType,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub maximum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub default: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "v2/")]
pub enum McpElicitationNumberType {
    Number,
    Integer,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct McpElicitationBooleanSchema {
    #[serde(rename = "type")]
    #[ts(rename = "type")]
    pub type_: McpElicitationBooleanType,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub default: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "v2/")]
pub enum McpElicitationBooleanType {
    Boolean,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(untagged)]
#[ts(export_to = "v2/")]
pub enum McpElicitationEnumSchema {
    SingleSelect(McpElicitationSingleSelectEnumSchema),
    MultiSelect(McpElicitationMultiSelectEnumSchema),
    Legacy(McpElicitationLegacyTitledEnumSchema),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct McpElicitationLegacyTitledEnumSchema {
    #[serde(rename = "type")]
    #[ts(rename = "type")]
    pub type_: McpElicitationStringType,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(rename = "enum")]
    #[ts(rename = "enum")]
    pub enum_: Vec<String>,
    #[serde(rename = "enumNames", skip_serializing_if = "Option::is_none")]
    #[ts(optional, rename = "enumNames")]
    pub enum_names: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub default: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(untagged)]
#[ts(export_to = "v2/")]
pub enum McpElicitationSingleSelectEnumSchema {
    Untitled(McpElicitationUntitledSingleSelectEnumSchema),
    Titled(McpElicitationTitledSingleSelectEnumSchema),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct McpElicitationUntitledSingleSelectEnumSchema {
    #[serde(rename = "type")]
    #[ts(rename = "type")]
    pub type_: McpElicitationStringType,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(rename = "enum")]
    #[ts(rename = "enum")]
    pub enum_: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub default: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct McpElicitationTitledSingleSelectEnumSchema {
    #[serde(rename = "type")]
    #[ts(rename = "type")]
    pub type_: McpElicitationStringType,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(rename = "oneOf")]
    #[ts(rename = "oneOf")]
    pub one_of: Vec<McpElicitationConstOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub default: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(untagged)]
#[ts(export_to = "v2/")]
pub enum McpElicitationMultiSelectEnumSchema {
    Untitled(McpElicitationUntitledMultiSelectEnumSchema),
    Titled(McpElicitationTitledMultiSelectEnumSchema),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct McpElicitationUntitledMultiSelectEnumSchema {
    #[serde(rename = "type")]
    #[ts(rename = "type")]
    pub type_: McpElicitationArrayType,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub min_items: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub max_items: Option<u64>,
    pub items: McpElicitationUntitledEnumItems,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub default: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct McpElicitationTitledMultiSelectEnumSchema {
    #[serde(rename = "type")]
    #[ts(rename = "type")]
    pub type_: McpElicitationArrayType,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub min_items: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub max_items: Option<u64>,
    pub items: McpElicitationTitledEnumItems,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub default: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "v2/")]
pub enum McpElicitationArrayType {
    Array,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct McpElicitationUntitledEnumItems {
    #[serde(rename = "type")]
    #[ts(rename = "type")]
    pub type_: McpElicitationStringType,
    #[serde(rename = "enum")]
    #[ts(rename = "enum")]
    pub enum_: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct McpElicitationTitledEnumItems {
    #[serde(rename = "anyOf", alias = "oneOf")]
    #[ts(rename = "anyOf")]
    pub any_of: Vec<McpElicitationConstOption>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct McpElicitationConstOption {
    #[serde(rename = "const")]
    #[ts(rename = "const")]
    pub const_: String,
    pub title: String,
}

fn json_wrong_type(message: impl Into<String>) -> JsonValueError {
    JsonValueError::WrongType(message.into())
}

fn parse_string_type(value: JsonValue) -> Result<McpElicitationStringType, JsonValueError> {
    match String::from_json(value)?.as_str() {
        "string" => Ok(McpElicitationStringType::String),
        other => Err(json_wrong_type(format!(
            "expected string type, found `{other}`"
        ))),
    }
}

fn parse_string_format(
    value: Option<JsonValue>,
) -> Result<Option<McpElicitationStringFormat>, JsonValueError> {
    let Some(value) = value else {
        return Ok(None);
    };
    match String::from_json(value)?.as_str() {
        "email" => Ok(Some(McpElicitationStringFormat::Email)),
        "uri" => Ok(Some(McpElicitationStringFormat::Uri)),
        "date" => Ok(Some(McpElicitationStringFormat::Date)),
        "date-time" => Ok(Some(McpElicitationStringFormat::DateTime)),
        other => Err(json_wrong_type(format!("unknown string format `{other}`"))),
    }
}

fn parse_const_options(
    values: JsonValue,
) -> Result<Vec<McpElicitationConstOption>, JsonValueError> {
    Vec::<JsonValue>::from_json(values)?
        .into_iter()
        .map(|value| {
            let mut object = value.into_object("McpElicitationConstOption")?;
            Ok(McpElicitationConstOption {
                const_: object.take_required("const")?,
                title: object.take_required("title")?,
            })
        })
        .collect()
}

fn parse_mcp_elicitation_schema(value: JsonValue) -> Result<McpElicitationSchema, JsonValueError> {
    let mut object = value.into_object("McpElicitationSchema")?;
    let type_value = String::from_json(object.take_required("type")?)?;
    if type_value != "object" {
        return Err(json_wrong_type(format!(
            "expected object schema type, found `{type_value}`"
        )));
    }
    let properties = object
        .take_required::<BTreeMap<String, JsonValue>>("properties")?
        .into_iter()
        .map(|(key, value)| parse_mcp_primitive_schema(value).map(|value| (key, value)))
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    Ok(McpElicitationSchema {
        schema_uri: object.take_optional("$schema")?,
        type_: McpElicitationObjectType::Object,
        properties,
        required: object.take_optional("required")?,
    })
}

fn parse_mcp_primitive_schema(
    value: JsonValue,
) -> Result<McpElicitationPrimitiveSchema, JsonValueError> {
    let mut object = value.into_object("McpElicitationPrimitiveSchema")?;
    let type_value = String::from_json(
        object
            .remove("type")
            .ok_or_else(|| json_wrong_type("missing required field `type`"))?,
    )?;
    match type_value.as_str() {
        "string" => {
            if object.contains_key("oneOf") {
                return Ok(McpElicitationPrimitiveSchema::Enum(
                    McpElicitationEnumSchema::SingleSelect(
                        McpElicitationSingleSelectEnumSchema::Titled(
                            McpElicitationTitledSingleSelectEnumSchema {
                                type_: McpElicitationStringType::String,
                                title: object.take_optional("title")?,
                                description: object.take_optional("description")?,
                                one_of: parse_const_options(object.take_required("oneOf")?)?,
                                default: object.take_optional("default")?,
                            },
                        ),
                    ),
                ));
            }
            if object.contains_key("enum") {
                let enum_: Vec<String> = object.take_required("enum")?;
                let title = object.take_optional("title")?;
                let description = object.take_optional("description")?;
                let default = object.take_optional("default")?;
                if object.contains_key("enumNames") {
                    return Ok(McpElicitationPrimitiveSchema::Enum(
                        McpElicitationEnumSchema::Legacy(McpElicitationLegacyTitledEnumSchema {
                            type_: McpElicitationStringType::String,
                            title,
                            description,
                            enum_,
                            enum_names: object.take_optional("enumNames")?,
                            default,
                        }),
                    ));
                }
                return Ok(McpElicitationPrimitiveSchema::Enum(
                    McpElicitationEnumSchema::SingleSelect(
                        McpElicitationSingleSelectEnumSchema::Untitled(
                            McpElicitationUntitledSingleSelectEnumSchema {
                                type_: McpElicitationStringType::String,
                                title,
                                description,
                                enum_,
                                default,
                            },
                        ),
                    ),
                ));
            }
            Ok(McpElicitationPrimitiveSchema::String(
                McpElicitationStringSchema {
                    type_: McpElicitationStringType::String,
                    title: object.take_optional("title")?,
                    description: object.take_optional("description")?,
                    min_length: object.take_optional("minLength")?,
                    max_length: object.take_optional("maxLength")?,
                    format: parse_string_format(object.remove("format"))?,
                    default: object.take_optional("default")?,
                },
            ))
        }
        "number" | "integer" => Ok(McpElicitationPrimitiveSchema::Number(
            McpElicitationNumberSchema {
                type_: if type_value == "integer" {
                    McpElicitationNumberType::Integer
                } else {
                    McpElicitationNumberType::Number
                },
                title: object.take_optional("title")?,
                description: object.take_optional("description")?,
                minimum: object.take_optional("minimum")?,
                maximum: object.take_optional("maximum")?,
                default: object.take_optional("default")?,
            },
        )),
        "boolean" => Ok(McpElicitationPrimitiveSchema::Boolean(
            McpElicitationBooleanSchema {
                type_: McpElicitationBooleanType::Boolean,
                title: object.take_optional("title")?,
                description: object.take_optional("description")?,
                default: object.take_optional("default")?,
            },
        )),
        "array" => {
            let mut items = object
                .take_required::<JsonValue>("items")?
                .into_object("McpElicitationArrayItems")?;
            if items.contains_key("anyOf") || items.contains_key("oneOf") {
                let options = items
                    .remove("anyOf")
                    .or_else(|| items.remove("oneOf"))
                    .ok_or_else(|| json_wrong_type("missing enum options"))?;
                return Ok(McpElicitationPrimitiveSchema::Enum(
                    McpElicitationEnumSchema::MultiSelect(
                        McpElicitationMultiSelectEnumSchema::Titled(
                            McpElicitationTitledMultiSelectEnumSchema {
                                type_: McpElicitationArrayType::Array,
                                title: object.take_optional("title")?,
                                description: object.take_optional("description")?,
                                min_items: object.take_optional("minItems")?,
                                max_items: object.take_optional("maxItems")?,
                                items: McpElicitationTitledEnumItems {
                                    any_of: parse_const_options(options)?,
                                },
                                default: object.take_optional("default")?,
                            },
                        ),
                    ),
                ));
            }
            Ok(McpElicitationPrimitiveSchema::Enum(
                McpElicitationEnumSchema::MultiSelect(
                    McpElicitationMultiSelectEnumSchema::Untitled(
                        McpElicitationUntitledMultiSelectEnumSchema {
                            type_: McpElicitationArrayType::Array,
                            title: object.take_optional("title")?,
                            description: object.take_optional("description")?,
                            min_items: object.take_optional("minItems")?,
                            max_items: object.take_optional("maxItems")?,
                            items: McpElicitationUntitledEnumItems {
                                type_: parse_string_type(items.take_required("type")?)?,
                                enum_: items.take_required("enum")?,
                            },
                            default: object.take_optional("default")?,
                        },
                    ),
                ),
            ))
        }
        other => Err(json_wrong_type(format!(
            "unknown MCP elicitation primitive type `{other}`"
        ))),
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(tag = "mode", rename_all = "camelCase")]
#[ts(tag = "mode")]
#[ts(export_to = "v2/")]
pub enum McpServerElicitationRequest {
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Form {
        #[serde(rename = "_meta")]
        #[ts(rename = "_meta")]
        meta: Option<JsonValue>,
        message: String,
        requested_schema: McpElicitationSchema,
    },
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Url {
        #[serde(rename = "_meta")]
        #[ts(rename = "_meta")]
        meta: Option<JsonValue>,
        message: String,
        url: String,
        elicitation_id: String,
    },
}

impl TryFrom<CoreElicitationRequest> for McpServerElicitationRequest {
    type Error = JsonValueError;

    fn try_from(value: CoreElicitationRequest) -> Result<Self, Self::Error> {
        match value {
            CoreElicitationRequest::Form {
                meta,
                message,
                requested_schema,
            } => Ok(Self::Form {
                meta,
                message,
                requested_schema: parse_mcp_elicitation_schema(requested_schema)?,
            }),
            CoreElicitationRequest::Url {
                meta,
                message,
                url,
                elicitation_id,
            } => Ok(Self::Url {
                meta,
                message,
                url,
                elicitation_id,
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct McpServerElicitationRequestResponse {
    pub action: McpServerElicitationAction,
    /// Structured user input for accepted elicitations, mirroring RMCP `CreateElicitationResult`.
    ///
    /// This is nullable because decline/cancel responses have no content.
    pub content: Option<JsonValue>,
    /// Optional client metadata for form-mode action handling.
    #[serde(rename = "_meta")]
    #[ts(rename = "_meta")]
    pub meta: Option<JsonValue>,
}

impl ToJson for McpServerElicitationRequestResponse {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(3);
        object.push_field("action", self.action.to_json());
        object.push_field("content", self.content.clone().unwrap_or(JsonValue::Null));
        object.push_field("_meta", self.meta.clone().unwrap_or(JsonValue::Null));
        JsonValue::Object(object)
    }
}

impl From<McpServerElicitationRequestResponse> for rmcp::model::CreateElicitationResult {
    fn from(value: McpServerElicitationRequestResponse) -> Self {
        Self {
            action: value.action.into(),
            content: value.content.and_then(|content| match content {
                JsonValue::Object(object) => Some(object),
                _ => None,
            }),
            meta: value.meta.and_then(|meta| match meta {
                JsonValue::Object(object) => Some(object),
                _ => None,
            }),
        }
    }
}

impl From<rmcp::model::CreateElicitationResult> for McpServerElicitationRequestResponse {
    fn from(value: rmcp::model::CreateElicitationResult) -> Self {
        Self {
            action: value.action.into(),
            content: value.content.map(JsonValue::Object),
            meta: value.meta.map(JsonValue::Object),
        }
    }
}
