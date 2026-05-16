use std::collections::BTreeMap;

use codex_app_server_protocol::AppInfo;
use codex_app_server_protocol::McpElicitationObjectType;
use codex_app_server_protocol::McpElicitationSchema;
use codex_app_server_protocol::McpServerElicitationRequest;
use codex_app_server_protocol::McpServerElicitationRequestParams;
use edgerun_json::{FromJson, JsonValueError, Map, ToJson, Value};

use crate::DiscoverableTool;
use crate::DiscoverableToolAction;
use crate::DiscoverableToolType;

pub const REQUEST_PLUGIN_INSTALL_APPROVAL_KIND_VALUE: &str = "tool_suggestion";
pub const REQUEST_PLUGIN_INSTALL_PERSIST_KEY: &str = "persist";
pub const REQUEST_PLUGIN_INSTALL_PERSIST_ALWAYS_VALUE: &str = "always";

#[derive(Debug)]
pub struct RequestPluginInstallArgs {
    pub tool_type: DiscoverableToolType,
    pub action_type: DiscoverableToolAction,
    pub tool_id: String,
    pub suggest_reason: String,
}

impl FromJson for RequestPluginInstallArgs {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("RequestPluginInstallArgs")?;
        Ok(Self {
            tool_type: object.take_required("tool_type")?,
            action_type: object.take_required("action_type")?,
            tool_id: object.take_required("tool_id")?,
            suggest_reason: object.take_required("suggest_reason")?,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RequestPluginInstallResult {
    pub completed: bool,
    pub user_confirmed: bool,
    pub tool_type: DiscoverableToolType,
    pub action_type: DiscoverableToolAction,
    pub tool_id: String,
    pub tool_name: String,
    pub suggest_reason: String,
}

impl ToJson for RequestPluginInstallResult {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("completed", self.completed);
        object.push_field("user_confirmed", self.user_confirmed);
        object.push_field("tool_type", self.tool_type.to_json());
        object.push_field("action_type", self.action_type.to_json());
        object.push_field("tool_id", self.tool_id.as_str());
        object.push_field("tool_name", self.tool_name.as_str());
        object.push_field("suggest_reason", self.suggest_reason.as_str());
        object.into()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RequestPluginInstallMeta<'a> {
    pub codex_approval_kind: &'static str,
    pub persist: &'static str,
    pub tool_type: DiscoverableToolType,
    pub suggest_type: DiscoverableToolAction,
    pub suggest_reason: &'a str,
    pub tool_id: &'a str,
    pub tool_name: &'a str,
    pub install_url: Option<&'a str>,
}

impl ToJson for RequestPluginInstallMeta<'_> {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("codex_approval_kind", self.codex_approval_kind);
        object.push_field("persist", self.persist);
        object.push_field("tool_type", self.tool_type.to_json());
        object.push_field("suggest_type", self.suggest_type.to_json());
        object.push_field("suggest_reason", self.suggest_reason);
        object.push_field("tool_id", self.tool_id);
        object.push_field("tool_name", self.tool_name);
        object.push_opt_field("install_url", self.install_url);
        object.into()
    }
}

pub fn build_request_plugin_install_elicitation_request(
    server_name: &str,
    thread_id: String,
    turn_id: String,
    args: &RequestPluginInstallArgs,
    suggest_reason: &str,
    tool: &DiscoverableTool,
) -> McpServerElicitationRequestParams {
    let tool_name = tool.name().to_string();
    let install_url = tool.install_url().map(ToString::to_string);
    let message = suggest_reason.to_string();

    McpServerElicitationRequestParams {
        thread_id,
        turn_id: Some(turn_id),
        server_name: server_name.to_string(),
        request: McpServerElicitationRequest::Form {
            meta: Some(
                build_request_plugin_install_meta(
                    args.tool_type,
                    args.action_type,
                    suggest_reason,
                    tool.id(),
                    tool_name.as_str(),
                    install_url.as_deref(),
                )
                .to_json(),
            ),
            message,
            requested_schema: McpElicitationSchema {
                schema_uri: None,
                type_: McpElicitationObjectType::Object,
                properties: BTreeMap::new(),
                required: None,
            },
        },
    }
}

pub fn all_requested_connectors_picked_up(
    expected_connector_ids: &[String],
    accessible_connectors: &[AppInfo],
) -> bool {
    expected_connector_ids.iter().all(|connector_id| {
        verified_connector_install_completed(connector_id, accessible_connectors)
    })
}

pub fn verified_connector_install_completed(
    tool_id: &str,
    accessible_connectors: &[AppInfo],
) -> bool {
    accessible_connectors
        .iter()
        .find(|connector| connector.id == tool_id)
        .is_some_and(|connector| connector.is_accessible)
}

fn build_request_plugin_install_meta<'a>(
    tool_type: DiscoverableToolType,
    action_type: DiscoverableToolAction,
    suggest_reason: &'a str,
    tool_id: &'a str,
    tool_name: &'a str,
    install_url: Option<&'a str>,
) -> RequestPluginInstallMeta<'a> {
    RequestPluginInstallMeta {
        codex_approval_kind: REQUEST_PLUGIN_INSTALL_APPROVAL_KIND_VALUE,
        persist: REQUEST_PLUGIN_INSTALL_PERSIST_ALWAYS_VALUE,
        tool_type,
        suggest_type: action_type,
        suggest_reason,
        tool_id,
        tool_name,
        install_url,
    }
}

#[cfg(test)]
#[path = "request_plugin_install_tests.rs"]
mod tests;
