use codex_analytics::GuardianReviewedAction;
use codex_protocol::approvals::GuardianAssessmentAction;
use codex_protocol::approvals::GuardianCommandSource;
use codex_protocol::approvals::NetworkApprovalProtocol;
use codex_utils_absolute_path::AbsolutePathBuf;
use edgerun_json::Map;
use edgerun_json::ToJson;
use edgerun_json::Value;

use super::GUARDIAN_MAX_ACTION_STRING_TOKENS;
use super::prompt::guardian_truncate_text;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GuardianApprovalRequest {
    Shell {
        id: String,
        command: Vec<String>,
        cwd: AbsolutePathBuf,
        justification: Option<String>,
    },
    ExecCommand {
        id: String,
        command: Vec<String>,
        cwd: AbsolutePathBuf,
        justification: Option<String>,
        tty: bool,
    },
    #[cfg(unix)]
    Execve {
        id: String,
        source: GuardianCommandSource,
        program: String,
        argv: Vec<String>,
        cwd: AbsolutePathBuf,
    },
    ApplyPatch {
        id: String,
        cwd: AbsolutePathBuf,
        files: Vec<AbsolutePathBuf>,
        patch: String,
    },
    NetworkAccess {
        id: String,
        turn_id: String,
        target: String,
        host: String,
        protocol: NetworkApprovalProtocol,
        port: u16,
        trigger: Option<GuardianNetworkAccessTrigger>,
    },
    McpToolCall {
        id: String,
        server: String,
        tool_name: String,
        arguments: Option<Value>,
        connector_id: Option<String>,
        connector_name: Option<String>,
        connector_description: Option<String>,
        tool_title: Option<String>,
        tool_description: Option<String>,
        annotations: Option<GuardianMcpAnnotations>,
    },
}

#[derive(Debug, Clone, PartialEq, ToJson)]
#[serde(rename_all = "camelCase")]
#[json(rename_all = "camelCase")]
pub(crate) struct GuardianNetworkAccessTrigger {
    pub(crate) call_id: String,
    pub(crate) tool_name: String,
    pub(crate) command: Vec<String>,
    pub(crate) cwd: AbsolutePathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[json(skip_serializing_if = "Option::is_none")]
    pub(crate) justification: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[json(skip_serializing_if = "Option::is_none")]
    pub(crate) tty: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, ToJson)]
pub(crate) struct GuardianMcpAnnotations {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[json(skip_serializing_if = "Option::is_none")]
    pub(crate) destructive_hint: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[json(skip_serializing_if = "Option::is_none")]
    pub(crate) open_world_hint: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[json(skip_serializing_if = "Option::is_none")]
    pub(crate) read_only_hint: Option<bool>,
}

fn serialize_command_guardian_action(
    tool: &'static str,
    command: &[String],
    cwd: &AbsolutePathBuf,
    justification: Option<&String>,
    tty: Option<bool>,
) -> edgerun_json::Result<Value> {
    let mut object = Map::new();
    object.push_field("tool", tool.to_json());
    object.push_field("command", command.to_json());
    object.push_field("cwd", cwd.to_json());
    if let Some(justification) = justification {
        object.push_field("justification", justification.to_json());
    }
    if let Some(tty) = tty {
        object.push_field("tty", tty.to_json());
    }
    Ok(Value::Object(object))
}

fn command_assessment_action(
    source: GuardianCommandSource,
    command: &[String],
    cwd: &AbsolutePathBuf,
) -> GuardianAssessmentAction {
    GuardianAssessmentAction::Command {
        source,
        command: codex_shell_command::parse_command::shlex_join(command),
        cwd: cwd.clone(),
    }
}

#[cfg(unix)]
fn guardian_command_source_tool_name(source: GuardianCommandSource) -> &'static str {
    match source {
        GuardianCommandSource::Shell => "shell",
        GuardianCommandSource::UnifiedExec => "exec_command",
    }
}

fn truncate_guardian_action_value(value: Value) -> (Value, bool) {
    match value {
        Value::String(text) => {
            let (text, truncated) =
                guardian_truncate_text(&text, GUARDIAN_MAX_ACTION_STRING_TOKENS);
            (Value::String(text), truncated)
        }
        Value::Array(values) => {
            let mut truncated = false;
            let values = values
                .into_iter()
                .map(|value| {
                    let (value, value_truncated) = truncate_guardian_action_value(value);
                    truncated |= value_truncated;
                    value
                })
                .collect::<Vec<_>>();
            (Value::Array(values), truncated)
        }
        Value::Object(values) => {
            let mut entries = values.into_iter().collect::<Vec<_>>();
            entries.sort_by(|(left, _), (right, _)| left.cmp(right));
            let mut truncated = false;
            let values = entries
                .into_iter()
                .map(|(key, value)| {
                    let (value, value_truncated) = truncate_guardian_action_value(value);
                    truncated |= value_truncated;
                    (key, value)
                })
                .collect();
            (Value::Object(values), truncated)
        }
        other => (other, false),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FormattedGuardianAction {
    pub(crate) text: String,
    pub(crate) truncated: bool,
}

pub(crate) fn guardian_approval_request_to_json(
    action: &GuardianApprovalRequest,
) -> edgerun_json::Result<Value> {
    match action {
        GuardianApprovalRequest::Shell {
            id: _,
            command,
            cwd,
            justification,
        } => serialize_command_guardian_action(
            "shell",
            command,
            cwd,
            justification.as_ref(),
            /*tty*/ None,
        ),
        GuardianApprovalRequest::ExecCommand {
            id: _,
            command,
            cwd,
            justification,
            tty,
        } => serialize_command_guardian_action(
            "exec_command",
            command,
            cwd,
            justification.as_ref(),
            Some(*tty),
        ),
        #[cfg(unix)]
        GuardianApprovalRequest::Execve {
            id: _,
            source,
            program,
            argv,
            cwd,
        } => Ok(edgerun_json::json!({
            "tool": guardian_command_source_tool_name(*source),
            "program": program,
            "argv": argv,
            "cwd": cwd,
        })),
        GuardianApprovalRequest::ApplyPatch {
            id: _,
            cwd,
            files,
            patch,
        } => Ok(edgerun_json::json!({
            "tool": "apply_patch",
            "cwd": cwd,
            "files": files,
            "patch": patch,
        })),
        GuardianApprovalRequest::NetworkAccess {
            id: _,
            turn_id: _,
            target,
            host,
            protocol,
            port,
            trigger,
        } => {
            let mut object = Map::new();
            object.push_field("tool", "network_access".to_json());
            object.push_field("target", target.to_json());
            object.push_field("host", host.to_json());
            object.push_field("protocol", protocol.to_json());
            object.push_field("port", port.to_json());
            if let Some(trigger) = trigger {
                object.push_field("trigger", trigger.to_json());
            }
            Ok(Value::Object(object))
        }
        GuardianApprovalRequest::McpToolCall {
            id: _,
            server,
            tool_name,
            arguments,
            connector_id,
            connector_name,
            connector_description,
            tool_title,
            tool_description,
            annotations,
        } => {
            let mut object = Map::new();
            object.push_field("tool", "mcp_tool_call".to_json());
            object.push_field("server", server.to_json());
            object.push_field("tool_name", tool_name.to_json());
            if let Some(arguments) = arguments {
                object.push_field("arguments", arguments.to_json());
            }
            if let Some(connector_id) = connector_id {
                object.push_field("connector_id", connector_id.to_json());
            }
            if let Some(connector_name) = connector_name {
                object.push_field("connector_name", connector_name.to_json());
            }
            if let Some(connector_description) = connector_description {
                object.push_field("connector_description", connector_description.to_json());
            }
            if let Some(tool_title) = tool_title {
                object.push_field("tool_title", tool_title.to_json());
            }
            if let Some(tool_description) = tool_description {
                object.push_field("tool_description", tool_description.to_json());
            }
            if let Some(annotations) = annotations {
                object.push_field("annotations", annotations.to_json());
            }
            Ok(Value::Object(object))
        }
    }
}

pub(crate) fn guardian_assessment_action(
    action: &GuardianApprovalRequest,
) -> GuardianAssessmentAction {
    match action {
        GuardianApprovalRequest::Shell { command, cwd, .. } => {
            command_assessment_action(GuardianCommandSource::Shell, command, cwd)
        }
        GuardianApprovalRequest::ExecCommand { command, cwd, .. } => {
            command_assessment_action(GuardianCommandSource::UnifiedExec, command, cwd)
        }
        #[cfg(unix)]
        GuardianApprovalRequest::Execve {
            source,
            program,
            argv,
            cwd,
            ..
        } => GuardianAssessmentAction::Execve {
            source: *source,
            program: program.clone(),
            argv: argv.clone(),
            cwd: cwd.clone(),
        },
        GuardianApprovalRequest::ApplyPatch { cwd, files, .. } => {
            GuardianAssessmentAction::ApplyPatch {
                cwd: cwd.clone(),
                files: files.clone(),
            }
        }
        GuardianApprovalRequest::NetworkAccess {
            id: _id,
            turn_id: _turn_id,
            target,
            host,
            protocol,
            port,
            trigger: _trigger,
        } => GuardianAssessmentAction::NetworkAccess {
            target: target.clone(),
            host: host.clone(),
            protocol: *protocol,
            port: *port,
        },
        GuardianApprovalRequest::McpToolCall {
            server,
            tool_name,
            connector_id,
            connector_name,
            tool_title,
            ..
        } => GuardianAssessmentAction::McpToolCall {
            server: server.clone(),
            tool_name: tool_name.clone(),
            connector_id: connector_id.clone(),
            connector_name: connector_name.clone(),
            tool_title: tool_title.clone(),
        },
    }
}

pub(crate) fn guardian_reviewed_action(
    request: &GuardianApprovalRequest,
) -> GuardianReviewedAction {
    match request {
        GuardianApprovalRequest::Shell { .. } => GuardianReviewedAction::Shell {},
        GuardianApprovalRequest::ExecCommand { tty, .. } => GuardianReviewedAction::UnifiedExec {
            tty: *tty,
        },
        #[cfg(unix)]
        GuardianApprovalRequest::Execve {
            source,
            program,
            ..
        } => GuardianReviewedAction::Execve {
            source: *source,
            program: program.clone(),
        },
        GuardianApprovalRequest::ApplyPatch { .. } => GuardianReviewedAction::ApplyPatch {},
        GuardianApprovalRequest::NetworkAccess { protocol, port, .. } => {
            GuardianReviewedAction::NetworkAccess {
                protocol: *protocol,
                port: *port,
            }
        }
        GuardianApprovalRequest::McpToolCall {
            server,
            tool_name,
            connector_id,
            connector_name,
            tool_title,
            ..
        } => GuardianReviewedAction::McpToolCall {
            server: server.clone(),
            tool_name: tool_name.clone(),
            connector_id: connector_id.clone(),
            connector_name: connector_name.clone(),
            tool_title: tool_title.clone(),
        },
    }
}

pub(crate) fn guardian_request_target_item_id(request: &GuardianApprovalRequest) -> Option<&str> {
    match request {
        GuardianApprovalRequest::Shell { id, .. }
        | GuardianApprovalRequest::ExecCommand { id, .. }
        | GuardianApprovalRequest::ApplyPatch { id, .. }
        | GuardianApprovalRequest::McpToolCall { id, .. } => Some(id),
        GuardianApprovalRequest::NetworkAccess { .. } => None,
        #[cfg(unix)]
        GuardianApprovalRequest::Execve { id, .. } => Some(id),
    }
}

pub(crate) fn guardian_request_turn_id<'a>(
    request: &'a GuardianApprovalRequest,
    default_turn_id: &'a str,
) -> &'a str {
    match request {
        GuardianApprovalRequest::NetworkAccess { turn_id, .. } => turn_id,
        GuardianApprovalRequest::Shell { .. }
        | GuardianApprovalRequest::ExecCommand { .. }
        | GuardianApprovalRequest::ApplyPatch { .. }
        | GuardianApprovalRequest::McpToolCall { .. } => default_turn_id,
        #[cfg(unix)]
        GuardianApprovalRequest::Execve { .. } => default_turn_id,
    }
}

pub(crate) fn format_guardian_action_pretty(
    action: &GuardianApprovalRequest,
) -> edgerun_json::Result<FormattedGuardianAction> {
    let value = guardian_approval_request_to_json(action)?;
    let (value, truncated) = truncate_guardian_action_value(value);
    Ok(FormattedGuardianAction {
        text: edgerun_json::to_json_string_pretty(&value)?,
        truncated,
    })
}
