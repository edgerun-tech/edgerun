use super::AdditionalFileSystemPermissions;
use super::ExecPolicyAmendment;
use super::McpToolCallError;
use super::McpToolCallResult;
use super::NetworkApprovalContext;
use super::NetworkApprovalProtocol;
use super::NetworkPolicyAmendment;
use super::UserInput;
use super::shared::v2_enum_from_core;
use crate::protocol::item_builders::convert_patch_changes;
use codex_protocol::approvals::GuardianAssessmentAction as CoreGuardianAssessmentAction;
use codex_protocol::approvals::GuardianAssessmentDecisionSource as CoreGuardianAssessmentDecisionSource;
use codex_protocol::approvals::GuardianCommandSource as CoreGuardianCommandSource;
use codex_protocol::compat::absolute_path::AbsolutePathBuf;
use codex_protocol::items::AgentMessageContent as CoreAgentMessageContent;
use codex_protocol::items::McpToolCallStatus as CoreMcpToolCallStatus;
use codex_protocol::items::TurnItem as CoreTurnItem;
use codex_protocol::memory_citation::MemoryCitation as CoreMemoryCitation;
use codex_protocol::memory_citation::MemoryCitationEntry as CoreMemoryCitationEntry;
use codex_protocol::models::MessagePhase;
use codex_protocol::models::ResponseItem;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::parse_command::ParsedCommand as CoreParsedCommand;
use codex_protocol::protocol::AgentStatus as CoreAgentStatus;
use codex_protocol::protocol::ExecCommandSource as CoreExecCommandSource;
use codex_protocol::protocol::ExecCommandStatus as CoreExecCommandStatus;
use codex_protocol::protocol::GuardianRiskLevel as CoreGuardianRiskLevel;
use codex_protocol::protocol::GuardianUserAuthorization as CoreGuardianUserAuthorization;
use codex_protocol::protocol::PatchApplyStatus as CorePatchApplyStatus;
use codex_protocol::protocol::ReviewDecision as CoreReviewDecision;
use edgerun_json::FromJson;
use edgerun_json::JsonValueError;
use edgerun_json::Map;
use edgerun_json::ToJson;
use edgerun_json::Value as JsonValue;
use schemars::JsonSchema;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson)]
#[serde(rename_all = "camelCase")]
pub enum CommandExecutionApprovalDecision {
    /// User approved the command.
    Accept,
    /// User approved the command and future prompts in the same session-scoped
    /// approval cache should run without prompting.
    AcceptForSession,
    /// User approved the command, and wants to apply the proposed execpolicy amendment so future
    /// matching commands can run without prompting.
    AcceptWithExecpolicyAmendment {
        execpolicy_amendment: ExecPolicyAmendment,
    },
    /// User chose a persistent network policy rule (allow/deny) for this host.
    ApplyNetworkPolicyAmendment {
        network_policy_amendment: NetworkPolicyAmendment,
    },
    /// User denied the command. The agent will continue the turn.
    Decline,
    /// User denied the command. The turn will also be immediately interrupted.
    Cancel,
}

impl FromJson for CommandExecutionApprovalDecision {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        if let Some(value) = value.as_str() {
            return match value {
                "accept" => Ok(Self::Accept),
                "acceptForSession" => Ok(Self::AcceptForSession),
                "decline" => Ok(Self::Decline),
                "cancel" => Ok(Self::Cancel),
                other => Err(JsonValueError::WrongType(format!(
                    "unknown command approval decision `{other}`"
                ))),
            };
        }

        let mut object = value.into_object("CommandExecutionApprovalDecision")?;
        let ty: String = object.take_required("type")?;
        match ty.as_str() {
            "acceptWithExecpolicyAmendment" => Ok(Self::AcceptWithExecpolicyAmendment {
                execpolicy_amendment: object.take_required("execpolicyAmendment")?,
            }),
            "applyNetworkPolicyAmendment" => Ok(Self::ApplyNetworkPolicyAmendment {
                network_policy_amendment: object.take_required("networkPolicyAmendment")?,
            }),
            other => Err(JsonValueError::WrongType(format!(
                "unknown command approval decision `{other}`"
            ))),
        }
    }
}

impl From<CoreReviewDecision> for CommandExecutionApprovalDecision {
    fn from(value: CoreReviewDecision) -> Self {
        match value {
            CoreReviewDecision::Approved => Self::Accept,
            CoreReviewDecision::ApprovedExecpolicyAmendment {
                proposed_execpolicy_amendment,
            } => Self::AcceptWithExecpolicyAmendment {
                execpolicy_amendment: proposed_execpolicy_amendment.into(),
            },
            CoreReviewDecision::ApprovedForSession => Self::AcceptForSession,
            CoreReviewDecision::NetworkPolicyAmendment {
                network_policy_amendment,
            } => Self::ApplyNetworkPolicyAmendment {
                network_policy_amendment: network_policy_amendment.into(),
            },
            CoreReviewDecision::Abort => Self::Cancel,
            CoreReviewDecision::Denied => Self::Decline,
            CoreReviewDecision::TimedOut => Self::Decline,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub enum FileChangeApprovalDecision {
    /// User approved the file changes.
    Accept,
    /// User approved the file changes and future changes to the same files should run without prompting.
    AcceptForSession,
    /// User denied the file changes. The agent will continue the turn.
    Decline,
    /// User denied the file changes. The turn will also be immediately interrupted.
    Cancel,
}

#[derive(Debug, Clone, PartialEq, JsonSchema)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CommandAction {
    Read {
        command: String,
        name: String,
        path: AbsolutePathBuf,
    },
    ListFiles {
        command: String,
        path: Option<String>,
    },
    Search {
        command: String,
        query: Option<String>,
        path: Option<String>,
    },
    Unknown {
        command: String,
    },
}

impl FromJson for CommandAction {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("CommandAction")?;
        let ty: String = object.take_required("type")?;
        match ty.as_str() {
            "read" => Ok(Self::Read {
                command: object.take_required("command")?,
                name: object.take_required("name")?,
                path: required_absolute_path(object.remove("path"), "path")?,
            }),
            "listFiles" => Ok(Self::ListFiles {
                command: object.take_required("command")?,
                path: object.take_optional("path")?,
            }),
            "search" => Ok(Self::Search {
                command: object.take_required("command")?,
                query: object.take_optional("query")?,
                path: object.take_optional("path")?,
            }),
            "unknown" => Ok(Self::Unknown {
                command: object.take_required("command")?,
            }),
            other => Err(JsonValueError::WrongType(format!(
                "unknown command action `{other}`"
            ))),
        }
    }
}

impl ToJson for CommandAction {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::new();
        match self {
            CommandAction::Read {
                command,
                name,
                path,
            } => {
                object.push_field("type", "read");
                object.push_field("command", command.clone());
                object.push_field("name", name.clone());
                object.push_field("path", absolute_path_json(path));
            }
            CommandAction::ListFiles { command, path } => {
                object.push_field("type", "listFiles");
                object.push_field("command", command.clone());
                object.push_field("path", path.to_json());
            }
            CommandAction::Search {
                command,
                query,
                path,
            } => {
                object.push_field("type", "search");
                object.push_field("command", command.clone());
                object.push_field("query", query.to_json());
                object.push_field("path", path.to_json());
            }
            CommandAction::Unknown { command } => {
                object.push_field("type", "unknown");
                object.push_field("command", command.clone());
            }
        }
        JsonValue::Object(object)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct MemoryCitation {
    pub entries: Vec<MemoryCitationEntry>,
    pub thread_ids: Vec<String>,
}

impl From<CoreMemoryCitation> for MemoryCitation {
    fn from(value: CoreMemoryCitation) -> Self {
        Self {
            entries: value.entries.into_iter().map(Into::into).collect(),
            thread_ids: value.rollout_ids,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct MemoryCitationEntry {
    pub path: String,
    pub line_start: u32,
    pub line_end: u32,
    pub note: String,
}

impl From<CoreMemoryCitationEntry> for MemoryCitationEntry {
    fn from(value: CoreMemoryCitationEntry) -> Self {
        Self {
            path: value.path,
            line_start: value.line_start,
            line_end: value.line_end,
            note: value.note,
        }
    }
}

impl CommandAction {
    pub fn into_core(self) -> CoreParsedCommand {
        match self {
            CommandAction::Read {
                command: cmd,
                name,
                path,
            } => CoreParsedCommand::Read {
                cmd,
                name,
                path: path.into_path_buf(),
            },
            CommandAction::ListFiles { command: cmd, path } => {
                CoreParsedCommand::ListFiles { cmd, path }
            }
            CommandAction::Search {
                command: cmd,
                query,
                path,
            } => CoreParsedCommand::Search { cmd, query, path },
            CommandAction::Unknown { command: cmd } => CoreParsedCommand::Unknown { cmd },
        }
    }

    pub fn from_core_with_cwd(value: CoreParsedCommand, cwd: &AbsolutePathBuf) -> Self {
        match value {
            CoreParsedCommand::Read { cmd, name, path } => CommandAction::Read {
                command: cmd,
                name,
                path: cwd.join(path),
            },
            CoreParsedCommand::ListFiles { cmd, path } => {
                CommandAction::ListFiles { command: cmd, path }
            }
            CoreParsedCommand::Search { cmd, query, path } => CommandAction::Search {
                command: cmd,
                query,
                path,
            },
            CoreParsedCommand::Unknown { cmd } => CommandAction::Unknown { command: cmd },
        }
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ThreadItem {
    #[serde(rename_all = "camelCase")]
    UserMessage { id: String, content: Vec<UserInput> },
    #[serde(rename_all = "camelCase")]
    HookPrompt {
        id: String,
        fragments: Vec<HookPromptFragment>,
    },
    #[serde(rename_all = "camelCase")]
    AgentMessage {
        id: String,
        text: String,
        #[serde(default)]
        phase: Option<MessagePhase>,
        #[serde(default)]
        memory_citation: Option<MemoryCitation>,
    },
    #[serde(rename_all = "camelCase")]
    /// EXPERIMENTAL - proposed plan item content. The completed plan item is
    /// authoritative and may not match the concatenation of `PlanDelta` text.
    Plan { id: String, text: String },
    #[serde(rename_all = "camelCase")]
    Reasoning {
        id: String,
        #[serde(default)]
        summary: Vec<String>,
        #[serde(default)]
        content: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    CommandExecution {
        id: String,
        /// The command to be executed.
        command: String,
        /// The command's working directory.
        cwd: AbsolutePathBuf,
        /// Identifier for the underlying PTY process (when available).
        process_id: Option<String>,
        #[serde(default)]
        source: CommandExecutionSource,
        status: CommandExecutionStatus,
        /// A best-effort parsing of the command to understand the action(s) it will perform.
        /// This returns a list of CommandAction objects because a single shell command may
        /// be composed of many commands piped together.
        command_actions: Vec<CommandAction>,
        /// The command's output, aggregated from stdout and stderr.
        aggregated_output: Option<String>,
        /// The command's exit code.
        exit_code: Option<i32>,
        /// The duration of the command execution in milliseconds.
        duration_ms: Option<i64>,
    },
    #[serde(rename_all = "camelCase")]
    FileChange {
        id: String,
        changes: Vec<FileUpdateChange>,
        status: PatchApplyStatus,
    },
    #[serde(rename_all = "camelCase")]
    McpToolCall {
        id: String,
        server: String,
        tool: String,
        status: McpToolCallStatus,
        arguments: JsonValue,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mcp_app_resource_uri: Option<String>,
        result: Option<Box<McpToolCallResult>>,
        error: Option<McpToolCallError>,
        /// The duration of the MCP tool call in milliseconds.
        duration_ms: Option<i64>,
    },
    #[serde(rename_all = "camelCase")]
    DynamicToolCall {
        id: String,
        namespace: Option<String>,
        tool: String,
        arguments: JsonValue,
        status: DynamicToolCallStatus,
        content_items: Option<Vec<DynamicToolCallOutputContentItem>>,
        success: Option<bool>,
        /// The duration of the dynamic tool call in milliseconds.
        duration_ms: Option<i64>,
    },
    #[serde(rename_all = "camelCase")]
    CollabAgentToolCall {
        /// Unique identifier for this collab tool call.
        id: String,
        /// Name of the collab tool that was invoked.
        tool: CollabAgentTool,
        /// Current status of the collab tool call.
        status: CollabAgentToolCallStatus,
        /// Thread ID of the agent issuing the collab request.
        sender_thread_id: String,
        /// Thread ID of the receiving agent, when applicable. In case of spawn operation,
        /// this corresponds to the newly spawned agent.
        receiver_thread_ids: Vec<String>,
        /// Prompt text sent as part of the collab tool call, when available.
        prompt: Option<String>,
        /// Model requested for the spawned agent, when applicable.
        model: Option<String>,
        /// Reasoning effort requested for the spawned agent, when applicable.
        reasoning_effort: Option<ReasoningEffort>,
        /// Last known status of the target agents, when available.
        agents_states: HashMap<String, CollabAgentState>,
    },
    #[serde(rename_all = "camelCase")]
    WebSearch {
        id: String,
        query: String,
        action: Option<WebSearchAction>,
    },
    #[serde(rename_all = "camelCase")]
    ImageView { id: String, path: AbsolutePathBuf },
    #[serde(rename_all = "camelCase")]
    ImageGeneration {
        id: String,
        status: String,
        revised_prompt: Option<String>,
        result: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        saved_path: Option<AbsolutePathBuf>,
    },
    #[serde(rename_all = "camelCase")]
    EnteredReviewMode { id: String, review: String },
    #[serde(rename_all = "camelCase")]
    ExitedReviewMode { id: String, review: String },
    #[serde(rename_all = "camelCase")]
    ContextCompaction { id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct HookPromptFragment {
    pub text: String,
    pub hook_run_id: String,
}

impl ThreadItem {
    pub fn id(&self) -> &str {
        match self {
            ThreadItem::UserMessage { id, .. }
            | ThreadItem::HookPrompt { id, .. }
            | ThreadItem::AgentMessage { id, .. }
            | ThreadItem::Plan { id, .. }
            | ThreadItem::Reasoning { id, .. }
            | ThreadItem::CommandExecution { id, .. }
            | ThreadItem::FileChange { id, .. }
            | ThreadItem::McpToolCall { id, .. }
            | ThreadItem::DynamicToolCall { id, .. }
            | ThreadItem::CollabAgentToolCall { id, .. }
            | ThreadItem::WebSearch { id, .. }
            | ThreadItem::ImageView { id, .. }
            | ThreadItem::ImageGeneration { id, .. }
            | ThreadItem::EnteredReviewMode { id, .. }
            | ThreadItem::ExitedReviewMode { id, .. }
            | ThreadItem::ContextCompaction { id, .. } => id,
        }
    }
}

impl ToJson for ThreadItem {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::new();
        match self {
            ThreadItem::UserMessage { id, content } => {
                object.push_field("type", "userMessage");
                object.push_field("id", id.clone());
                object.push_field("content", content.to_json());
            }
            ThreadItem::HookPrompt { id, fragments } => {
                object.push_field("type", "hookPrompt");
                object.push_field("id", id.clone());
                object.push_field("fragments", fragments.to_json());
            }
            ThreadItem::AgentMessage {
                id,
                text,
                phase,
                memory_citation,
            } => {
                object.push_field("type", "agentMessage");
                object.push_field("id", id.clone());
                object.push_field("text", text.clone());
                object.push_field("phase", message_phase_to_json(phase.clone()));
                object.push_field("memoryCitation", memory_citation.to_json());
            }
            ThreadItem::Plan { id, text } => {
                object.push_field("type", "plan");
                object.push_field("id", id.clone());
                object.push_field("text", text.clone());
            }
            ThreadItem::Reasoning {
                id,
                summary,
                content,
            } => {
                object.push_field("type", "reasoning");
                object.push_field("id", id.clone());
                object.push_field("summary", summary.to_json());
                object.push_field("content", content.to_json());
            }
            ThreadItem::CommandExecution {
                id,
                command,
                cwd,
                process_id,
                source,
                status,
                command_actions,
                aggregated_output,
                exit_code,
                duration_ms,
            } => {
                object.push_field("type", "commandExecution");
                object.push_field("id", id.clone());
                object.push_field("command", command.clone());
                object.push_field("cwd", absolute_path_json(cwd));
                object.push_field("processId", process_id.to_json());
                object.push_field("source", source.to_json());
                object.push_field("status", status.to_json());
                object.push_field("commandActions", command_actions.to_json());
                object.push_field("aggregatedOutput", aggregated_output.to_json());
                object.push_field("exitCode", exit_code.to_json());
                object.push_field("durationMs", duration_ms.to_json());
            }
            ThreadItem::FileChange {
                id,
                changes,
                status,
            } => {
                object.push_field("type", "fileChange");
                object.push_field("id", id.clone());
                object.push_field("changes", changes.to_json());
                object.push_field("status", status.to_json());
            }
            ThreadItem::McpToolCall {
                id,
                server,
                tool,
                status,
                arguments,
                mcp_app_resource_uri,
                result,
                error,
                duration_ms,
            } => {
                object.push_field("type", "mcpToolCall");
                object.push_field("id", id.clone());
                object.push_field("server", server.clone());
                object.push_field("tool", tool.clone());
                object.push_field("status", status.to_json());
                object.push_field("arguments", arguments.clone());
                object.push_opt_field("mcpAppResourceUri", mcp_app_resource_uri.clone());
                object.push_field("result", result.to_json());
                object.push_field("error", error.to_json());
                object.push_field("durationMs", duration_ms.to_json());
            }
            ThreadItem::DynamicToolCall {
                id,
                namespace,
                tool,
                arguments,
                status,
                content_items,
                success,
                duration_ms,
            } => {
                object.push_field("type", "dynamicToolCall");
                object.push_field("id", id.clone());
                object.push_field("namespace", namespace.to_json());
                object.push_field("tool", tool.clone());
                object.push_field("arguments", arguments.clone());
                object.push_field("status", status.to_json());
                object.push_field("contentItems", content_items.to_json());
                object.push_field("success", success.to_json());
                object.push_field("durationMs", duration_ms.to_json());
            }
            ThreadItem::CollabAgentToolCall {
                id,
                tool,
                status,
                sender_thread_id,
                receiver_thread_ids,
                prompt,
                model,
                reasoning_effort,
                agents_states,
            } => {
                object.push_field("type", "collabAgentToolCall");
                object.push_field("id", id.clone());
                object.push_field("tool", tool.to_json());
                object.push_field("status", status.to_json());
                object.push_field("senderThreadId", sender_thread_id.clone());
                object.push_field("receiverThreadIds", receiver_thread_ids.to_json());
                object.push_field("prompt", prompt.to_json());
                object.push_field("model", model.to_json());
                object.push_field("reasoningEffort", reasoning_effort.to_json());
                object.push_field("agentsStates", agents_states.to_json());
            }
            ThreadItem::WebSearch { id, query, action } => {
                object.push_field("type", "webSearch");
                object.push_field("id", id.clone());
                object.push_field("query", query.clone());
                object.push_field("action", action.to_json());
            }
            ThreadItem::ImageView { id, path } => {
                object.push_field("type", "imageView");
                object.push_field("id", id.clone());
                object.push_field("path", absolute_path_json(path));
            }
            ThreadItem::ImageGeneration {
                id,
                status,
                revised_prompt,
                result,
                saved_path,
            } => {
                object.push_field("type", "imageGeneration");
                object.push_field("id", id.clone());
                object.push_field("status", status.clone());
                object.push_field("revisedPrompt", revised_prompt.to_json());
                object.push_field("result", result.clone());
                if let Some(saved_path) = saved_path {
                    object.push_field("savedPath", absolute_path_json(saved_path));
                }
            }
            ThreadItem::EnteredReviewMode { id, review } => {
                object.push_field("type", "enteredReviewMode");
                object.push_field("id", id.clone());
                object.push_field("review", review.clone());
            }
            ThreadItem::ExitedReviewMode { id, review } => {
                object.push_field("type", "exitedReviewMode");
                object.push_field("id", id.clone());
                object.push_field("review", review.clone());
            }
            ThreadItem::ContextCompaction { id } => {
                object.push_field("type", "contextCompaction");
                object.push_field("id", id.clone());
            }
        }
        JsonValue::Object(object)
    }
}

impl ToJson for HookPromptFragment {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(2);
        object.push_field("text", self.text.clone());
        object.push_field("hookRunId", self.hook_run_id.clone());
        JsonValue::Object(object)
    }
}

impl ToJson for MemoryCitation {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(2);
        object.push_field("entries", self.entries.to_json());
        object.push_field("threadIds", self.thread_ids.to_json());
        JsonValue::Object(object)
    }
}

impl ToJson for MemoryCitationEntry {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(4);
        object.push_field("path", self.path.clone());
        object.push_field("lineStart", self.line_start as u64);
        object.push_field("lineEnd", self.line_end as u64);
        object.push_field("note", self.note.clone());
        JsonValue::Object(object)
    }
}

fn message_phase_to_json(value: Option<MessagePhase>) -> JsonValue {
    value.map_or(JsonValue::Null, |phase| {
        JsonValue::from(match phase {
            MessagePhase::Commentary => "commentary",
            MessagePhase::FinalAnswer => "final_answer",
        })
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
/// [UNSTABLE] Lifecycle state for an approval auto-review.
pub enum GuardianApprovalReviewStatus {
    InProgress,
    Approved,
    Denied,
    TimedOut,
    Aborted,
}

impl ToJson for GuardianApprovalReviewStatus {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            Self::InProgress => "inProgress",
            Self::Approved => "approved",
            Self::Denied => "denied",
            Self::TimedOut => "timedOut",
            Self::Aborted => "aborted",
        })
    }
}

impl FromJson for GuardianApprovalReviewStatus {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "inProgress" => Ok(Self::InProgress),
            "approved" => Ok(Self::Approved),
            "denied" => Ok(Self::Denied),
            "timedOut" => Ok(Self::TimedOut),
            "aborted" => Ok(Self::Aborted),
            other => Err(JsonValueError::WrongType(format!(
                "unknown guardian approval review status `{other}`"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
/// [UNSTABLE] Source that produced a terminal approval auto-review decision.
pub enum AutoReviewDecisionSource {
    Agent,
}

impl From<CoreGuardianAssessmentDecisionSource> for AutoReviewDecisionSource {
    fn from(value: CoreGuardianAssessmentDecisionSource) -> Self {
        match value {
            CoreGuardianAssessmentDecisionSource::Agent => Self::Agent,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "lowercase")]
/// [UNSTABLE] Risk level assigned by approval auto-review.
pub enum GuardianRiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl ToJson for GuardianRiskLevel {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        })
    }
}

impl FromJson for GuardianRiskLevel {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "critical" => Ok(Self::Critical),
            other => Err(JsonValueError::WrongType(format!(
                "unknown guardian risk level `{other}`"
            ))),
        }
    }
}

impl From<CoreGuardianRiskLevel> for GuardianRiskLevel {
    fn from(value: CoreGuardianRiskLevel) -> Self {
        match value {
            CoreGuardianRiskLevel::Low => Self::Low,
            CoreGuardianRiskLevel::Medium => Self::Medium,
            CoreGuardianRiskLevel::High => Self::High,
            CoreGuardianRiskLevel::Critical => Self::Critical,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "lowercase")]
/// [UNSTABLE] Authorization level assigned by approval auto-review.
pub enum GuardianUserAuthorization {
    Unknown,
    Low,
    Medium,
    High,
}

impl ToJson for GuardianUserAuthorization {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            Self::Unknown => "unknown",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        })
    }
}

impl FromJson for GuardianUserAuthorization {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "unknown" => Ok(Self::Unknown),
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            other => Err(JsonValueError::WrongType(format!(
                "unknown guardian user authorization `{other}`"
            ))),
        }
    }
}

impl From<CoreGuardianUserAuthorization> for GuardianUserAuthorization {
    fn from(value: CoreGuardianUserAuthorization) -> Self {
        match value {
            CoreGuardianUserAuthorization::Unknown => Self::Unknown,
            CoreGuardianUserAuthorization::Low => Self::Low,
            CoreGuardianUserAuthorization::Medium => Self::Medium,
            CoreGuardianUserAuthorization::High => Self::High,
        }
    }
}

/// [UNSTABLE] Temporary approval auto-review payload used by
/// `item/autoApprovalReview/*` notifications. This shape is expected to change
/// soon.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GuardianApprovalReview {
    pub status: GuardianApprovalReviewStatus,
    pub risk_level: Option<GuardianRiskLevel>,
    pub user_authorization: Option<GuardianUserAuthorization>,
    pub rationale: Option<String>,
}

impl ToJson for GuardianApprovalReview {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(4);
        object.push_field("status", self.status.to_json());
        object.push_field("riskLevel", self.risk_level.to_json());
        object.push_field("userAuthorization", self.user_authorization.to_json());
        object.push_field("rationale", self.rationale.to_json());
        JsonValue::Object(object)
    }
}

impl FromJson for GuardianApprovalReview {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("GuardianApprovalReview")?;
        Ok(Self {
            status: object.take_required("status")?,
            risk_level: object.take_optional("riskLevel")?,
            user_authorization: object.take_optional("userAuthorization")?,
            rationale: object.take_optional("rationale")?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum GuardianCommandSource {
    Shell,
    UnifiedExec,
}

impl ToJson for GuardianCommandSource {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            Self::Shell => "shell",
            Self::UnifiedExec => "unifiedExec",
        })
    }
}

impl FromJson for GuardianCommandSource {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "shell" => Ok(Self::Shell),
            "unifiedExec" => Ok(Self::UnifiedExec),
            other => Err(JsonValueError::WrongType(format!(
                "unknown guardian command source `{other}`"
            ))),
        }
    }
}

impl From<CoreGuardianCommandSource> for GuardianCommandSource {
    fn from(value: CoreGuardianCommandSource) -> Self {
        match value {
            CoreGuardianCommandSource::Shell => Self::Shell,
            CoreGuardianCommandSource::UnifiedExec => Self::UnifiedExec,
        }
    }
}

impl From<GuardianCommandSource> for CoreGuardianCommandSource {
    fn from(value: GuardianCommandSource) -> Self {
        match value {
            GuardianCommandSource::Shell => Self::Shell,
            GuardianCommandSource::UnifiedExec => Self::UnifiedExec,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct GuardianCommandReviewAction {
    pub source: GuardianCommandSource,
    pub command: String,
    pub cwd: AbsolutePathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct GuardianExecveReviewAction {
    pub source: GuardianCommandSource,
    pub program: String,
    pub argv: Vec<String>,
    pub cwd: AbsolutePathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct GuardianApplyPatchReviewAction {
    pub cwd: AbsolutePathBuf,
    pub files: Vec<AbsolutePathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct GuardianNetworkAccessReviewAction {
    pub target: String,
    pub host: String,
    pub protocol: NetworkApprovalProtocol,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct GuardianMcpToolCallReviewAction {
    pub server: String,
    pub tool_name: String,
    pub connector_id: Option<String>,
    pub connector_name: Option<String>,
    pub tool_title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum GuardianApprovalReviewAction {
    #[serde(rename_all = "camelCase")]
    Command {
        source: GuardianCommandSource,
        command: String,
        cwd: AbsolutePathBuf,
    },
    #[serde(rename_all = "camelCase")]
    Execve {
        source: GuardianCommandSource,
        program: String,
        argv: Vec<String>,
        cwd: AbsolutePathBuf,
    },
    #[serde(rename_all = "camelCase")]
    ApplyPatch {
        cwd: AbsolutePathBuf,
        files: Vec<AbsolutePathBuf>,
    },
    #[serde(rename_all = "camelCase")]
    NetworkAccess {
        target: String,
        host: String,
        protocol: NetworkApprovalProtocol,
        port: u16,
    },
    #[serde(rename_all = "camelCase")]
    McpToolCall {
        server: String,
        tool_name: String,
        connector_id: Option<String>,
        connector_name: Option<String>,
        tool_title: Option<String>,
    },
}

impl ToJson for GuardianApprovalReviewAction {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::new();
        match self {
            Self::Command {
                source,
                command,
                cwd,
            } => {
                object.push_field("type", "command");
                object.push_field("source", source.to_json());
                object.push_field("command", command.clone());
                object.push_field("cwd", absolute_path_json(cwd));
            }
            Self::Execve {
                source,
                program,
                argv,
                cwd,
            } => {
                object.push_field("type", "execve");
                object.push_field("source", source.to_json());
                object.push_field("program", program.clone());
                object.push_field("argv", argv.to_json());
                object.push_field("cwd", absolute_path_json(cwd));
            }
            Self::ApplyPatch { cwd, files } => {
                object.push_field("type", "applyPatch");
                object.push_field("cwd", absolute_path_json(cwd));
                object.push_field("files", absolute_paths_json(files));
            }
            Self::NetworkAccess {
                target,
                host,
                protocol,
                port,
            } => {
                object.push_field("type", "networkAccess");
                object.push_field("target", target.clone());
                object.push_field("host", host.clone());
                object.push_field("protocol", protocol.to_json());
                object.push_field("port", *port as u64);
            }
            Self::McpToolCall {
                server,
                tool_name,
                connector_id,
                connector_name,
                tool_title,
            } => {
                object.push_field("type", "mcpToolCall");
                object.push_field("server", server.clone());
                object.push_field("toolName", tool_name.clone());
                object.push_field("connectorId", connector_id.to_json());
                object.push_field("connectorName", connector_name.to_json());
                object.push_field("toolTitle", tool_title.to_json());
            }
        }
        JsonValue::Object(object)
    }
}

impl FromJson for GuardianApprovalReviewAction {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("GuardianApprovalReviewAction")?;
        let ty: String = object.take_required("type")?;
        match ty.as_str() {
            "command" => Ok(Self::Command {
                source: object.take_required("source")?,
                command: object.take_required("command")?,
                cwd: required_absolute_path(object.remove("cwd"), "cwd")?,
            }),
            "execve" => Ok(Self::Execve {
                source: object.take_required("source")?,
                program: object.take_required("program")?,
                argv: object.take_required("argv")?,
                cwd: required_absolute_path(object.remove("cwd"), "cwd")?,
            }),
            "applyPatch" => Ok(Self::ApplyPatch {
                cwd: required_absolute_path(object.remove("cwd"), "cwd")?,
                files: required_absolute_paths(object.remove("files"), "files")?,
            }),
            "networkAccess" => Ok(Self::NetworkAccess {
                target: object.take_required("target")?,
                host: object.take_required("host")?,
                protocol: object.take_required("protocol")?,
                port: object.take_required("port")?,
            }),
            "mcpToolCall" => Ok(Self::McpToolCall {
                server: object.take_required("server")?,
                tool_name: object.take_required("toolName")?,
                connector_id: object.take_optional("connectorId")?,
                connector_name: object.take_optional("connectorName")?,
                tool_title: object.take_optional("toolTitle")?,
            }),
            other => Err(JsonValueError::WrongType(format!(
                "unknown guardian approval review action `{other}`"
            ))),
        }
    }
}

impl From<CoreGuardianAssessmentAction> for GuardianApprovalReviewAction {
    fn from(value: CoreGuardianAssessmentAction) -> Self {
        match value {
            CoreGuardianAssessmentAction::Command {
                source,
                command,
                cwd,
            } => Self::Command {
                source: source.into(),
                command,
                cwd,
            },
            CoreGuardianAssessmentAction::Execve {
                source,
                program,
                argv,
                cwd,
            } => Self::Execve {
                source: source.into(),
                program,
                argv,
                cwd,
            },
            CoreGuardianAssessmentAction::ApplyPatch { cwd, files } => {
                Self::ApplyPatch { cwd, files }
            }
            CoreGuardianAssessmentAction::NetworkAccess {
                target,
                host,
                protocol,
                port,
            } => Self::NetworkAccess {
                target,
                host,
                protocol: protocol.into(),
                port,
            },
            CoreGuardianAssessmentAction::McpToolCall {
                server,
                tool_name,
                connector_id,
                connector_name,
                tool_title,
            } => Self::McpToolCall {
                server,
                tool_name,
                connector_id,
                connector_name,
                tool_title,
            },
        }
    }
}

impl From<GuardianApprovalReviewAction> for CoreGuardianAssessmentAction {
    fn from(value: GuardianApprovalReviewAction) -> Self {
        match value {
            GuardianApprovalReviewAction::Command {
                source,
                command,
                cwd,
            } => Self::Command {
                source: source.into(),
                command,
                cwd,
            },
            GuardianApprovalReviewAction::Execve {
                source,
                program,
                argv,
                cwd,
            } => Self::Execve {
                source: source.into(),
                program,
                argv,
                cwd,
            },
            GuardianApprovalReviewAction::ApplyPatch { cwd, files } => {
                Self::ApplyPatch { cwd, files }
            }
            GuardianApprovalReviewAction::NetworkAccess {
                target,
                host,
                protocol,
                port,
            } => Self::NetworkAccess {
                target,
                host,
                protocol: protocol.to_core(),
                port,
            },
            GuardianApprovalReviewAction::McpToolCall {
                server,
                tool_name,
                connector_id,
                connector_name,
                tool_title,
            } => Self::McpToolCall {
                server,
                tool_name,
                connector_id,
                connector_name,
                tool_title,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WebSearchAction {
    Search {
        query: Option<String>,
        queries: Option<Vec<String>>,
    },
    OpenPage {
        url: Option<String>,
    },
    FindInPage {
        url: Option<String>,
        pattern: Option<String>,
    },
    #[serde(other)]
    Other,
}

impl ToJson for WebSearchAction {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::new();
        match self {
            WebSearchAction::Search { query, queries } => {
                object.push_field("type", "search");
                object.push_field("query", query.to_json());
                object.push_field("queries", queries.to_json());
            }
            WebSearchAction::OpenPage { url } => {
                object.push_field("type", "openPage");
                object.push_field("url", url.to_json());
            }
            WebSearchAction::FindInPage { url, pattern } => {
                object.push_field("type", "findInPage");
                object.push_field("url", url.to_json());
                object.push_field("pattern", pattern.to_json());
            }
            WebSearchAction::Other => {
                object.push_field("type", "other");
            }
        }
        JsonValue::Object(object)
    }
}

impl From<codex_protocol::models::WebSearchAction> for WebSearchAction {
    fn from(value: codex_protocol::models::WebSearchAction) -> Self {
        match value {
            codex_protocol::models::WebSearchAction::Search { query, queries } => {
                WebSearchAction::Search { query, queries }
            }
            codex_protocol::models::WebSearchAction::OpenPage { url } => {
                WebSearchAction::OpenPage { url }
            }
            codex_protocol::models::WebSearchAction::FindInPage { url, pattern } => {
                WebSearchAction::FindInPage { url, pattern }
            }
            codex_protocol::models::WebSearchAction::Other => WebSearchAction::Other,
        }
    }
}

impl From<CoreTurnItem> for ThreadItem {
    fn from(value: CoreTurnItem) -> Self {
        match value {
            CoreTurnItem::UserMessage(user) => ThreadItem::UserMessage {
                id: user.id,
                content: user.content.into_iter().map(UserInput::from).collect(),
            },
            CoreTurnItem::HookPrompt(hook_prompt) => ThreadItem::HookPrompt {
                id: hook_prompt.id,
                fragments: hook_prompt
                    .fragments
                    .into_iter()
                    .map(HookPromptFragment::from)
                    .collect(),
            },
            CoreTurnItem::AgentMessage(agent) => {
                let text = agent
                    .content
                    .into_iter()
                    .map(|entry| match entry {
                        CoreAgentMessageContent::Text { text } => text,
                    })
                    .collect::<String>();
                ThreadItem::AgentMessage {
                    id: agent.id,
                    text,
                    phase: agent.phase,
                    memory_citation: agent.memory_citation.map(Into::into),
                }
            }
            CoreTurnItem::Plan(plan) => ThreadItem::Plan {
                id: plan.id,
                text: plan.text,
            },
            CoreTurnItem::Reasoning(reasoning) => ThreadItem::Reasoning {
                id: reasoning.id,
                summary: reasoning.summary_text,
                content: reasoning.raw_content,
            },
            CoreTurnItem::WebSearch(search) => ThreadItem::WebSearch {
                id: search.id,
                query: search.query,
                action: Some(WebSearchAction::from(search.action)),
            },
            CoreTurnItem::ImageView(image) => ThreadItem::ImageView {
                id: image.id,
                path: image.path,
            },
            CoreTurnItem::ImageGeneration(image) => ThreadItem::ImageGeneration {
                id: image.id,
                status: image.status,
                revised_prompt: image.revised_prompt,
                result: image.result,
                saved_path: image.saved_path,
            },
            CoreTurnItem::FileChange(file_change) => ThreadItem::FileChange {
                id: file_change.id,
                changes: convert_patch_changes(&file_change.changes),
                status: file_change
                    .status
                    .as_ref()
                    .map(PatchApplyStatus::from)
                    .unwrap_or(PatchApplyStatus::InProgress),
            },
            CoreTurnItem::McpToolCall(mcp) => {
                let duration_ms = mcp
                    .duration
                    .and_then(|duration| i64::try_from(duration.as_millis()).ok());

                ThreadItem::McpToolCall {
                    id: mcp.id,
                    server: mcp.server,
                    tool: mcp.tool,
                    status: McpToolCallStatus::from(mcp.status),
                    arguments: mcp.arguments,
                    mcp_app_resource_uri: mcp.mcp_app_resource_uri,
                    result: mcp.result.map(McpToolCallResult::from).map(Box::new),
                    error: mcp.error.map(McpToolCallError::from),
                    duration_ms,
                }
            }
            CoreTurnItem::ContextCompaction(compaction) => {
                ThreadItem::ContextCompaction { id: compaction.id }
            }
        }
    }
}

impl From<codex_protocol::items::HookPromptFragment> for HookPromptFragment {
    fn from(value: codex_protocol::items::HookPromptFragment) -> Self {
        Self {
            text: value.text,
            hook_run_id: value.hook_run_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub enum CommandExecutionStatus {
    InProgress,
    Completed,
    Failed,
    Declined,
}

impl ToJson for CommandExecutionStatus {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            CommandExecutionStatus::InProgress => "inProgress",
            CommandExecutionStatus::Completed => "completed",
            CommandExecutionStatus::Failed => "failed",
            CommandExecutionStatus::Declined => "declined",
        })
    }
}

impl From<CoreExecCommandStatus> for CommandExecutionStatus {
    fn from(value: CoreExecCommandStatus) -> Self {
        Self::from(&value)
    }
}

impl From<&CoreExecCommandStatus> for CommandExecutionStatus {
    fn from(value: &CoreExecCommandStatus) -> Self {
        match value {
            CoreExecCommandStatus::Completed => CommandExecutionStatus::Completed,
            CoreExecCommandStatus::Failed => CommandExecutionStatus::Failed,
            CoreExecCommandStatus::Declined => CommandExecutionStatus::Declined,
        }
    }
}

v2_enum_from_core! {
    #[derive(Default)]
    pub enum CommandExecutionSource from CoreExecCommandSource {
        #[default]
        Agent,
        UserShell,
        UnifiedExecStartup,
        UnifiedExecInteraction,
    }
}


#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub enum CollabAgentTool {
    SpawnAgent,
    SendInput,
    ResumeAgent,
    Wait,
    CloseAgent,
}

impl ToJson for CollabAgentTool {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            CollabAgentTool::SpawnAgent => "spawnAgent",
            CollabAgentTool::SendInput => "sendInput",
            CollabAgentTool::ResumeAgent => "resumeAgent",
            CollabAgentTool::Wait => "wait",
            CollabAgentTool::CloseAgent => "closeAgent",
        })
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct FileUpdateChange {
    pub path: String,
    pub kind: PatchChangeKind,
    pub diff: String,
}

impl ToJson for FileUpdateChange {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(3);
        object.push_field("path", self.path.clone());
        object.push_field("kind", self.kind.to_json());
        object.push_field("diff", self.diff.clone());
        JsonValue::Object(object)
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PatchChangeKind {
    Add,
    Delete,
    Update { move_path: Option<PathBuf> },
}

impl ToJson for PatchChangeKind {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::new();
        match self {
            PatchChangeKind::Add => object.push_field("type", "add"),
            PatchChangeKind::Delete => object.push_field("type", "delete"),
            PatchChangeKind::Update { move_path } => {
                object.push_field("type", "update");
                object.push_field(
                    "movePath",
                    move_path.as_ref().map_or(JsonValue::Null, |path| {
                        JsonValue::String(path.as_path().to_string_lossy().into_owned())
                    }),
                );
            }
        }
        JsonValue::Object(object)
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub enum PatchApplyStatus {
    InProgress,
    Completed,
    Failed,
    Declined,
}

impl ToJson for PatchApplyStatus {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            PatchApplyStatus::InProgress => "inProgress",
            PatchApplyStatus::Completed => "completed",
            PatchApplyStatus::Failed => "failed",
            PatchApplyStatus::Declined => "declined",
        })
    }
}

impl From<CorePatchApplyStatus> for PatchApplyStatus {
    fn from(value: CorePatchApplyStatus) -> Self {
        Self::from(&value)
    }
}

impl From<&CorePatchApplyStatus> for PatchApplyStatus {
    fn from(value: &CorePatchApplyStatus) -> Self {
        match value {
            CorePatchApplyStatus::Completed => PatchApplyStatus::Completed,
            CorePatchApplyStatus::Failed => PatchApplyStatus::Failed,
            CorePatchApplyStatus::Declined => PatchApplyStatus::Declined,
        }
    }
}

impl From<CoreMcpToolCallStatus> for McpToolCallStatus {
    fn from(value: CoreMcpToolCallStatus) -> Self {
        match value {
            CoreMcpToolCallStatus::InProgress => McpToolCallStatus::InProgress,
            CoreMcpToolCallStatus::Completed => McpToolCallStatus::Completed,
            CoreMcpToolCallStatus::Failed => McpToolCallStatus::Failed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub enum McpToolCallStatus {
    InProgress,
    Completed,
    Failed,
}

impl ToJson for McpToolCallStatus {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            McpToolCallStatus::InProgress => "inProgress",
            McpToolCallStatus::Completed => "completed",
            McpToolCallStatus::Failed => "failed",
        })
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub enum DynamicToolCallStatus {
    InProgress,
    Completed,
    Failed,
}

impl ToJson for DynamicToolCallStatus {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            DynamicToolCallStatus::InProgress => "inProgress",
            DynamicToolCallStatus::Completed => "completed",
            DynamicToolCallStatus::Failed => "failed",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub enum CollabAgentToolCallStatus {
    InProgress,
    Completed,
    Failed,
}

impl ToJson for CollabAgentToolCallStatus {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            CollabAgentToolCallStatus::InProgress => "inProgress",
            CollabAgentToolCallStatus::Completed => "completed",
            CollabAgentToolCallStatus::Failed => "failed",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub enum CollabAgentStatus {
    PendingInit,
    Running,
    Interrupted,
    Completed,
    Errored,
    Shutdown,
    NotFound,
}

impl ToJson for CollabAgentStatus {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            CollabAgentStatus::PendingInit => "pendingInit",
            CollabAgentStatus::Running => "running",
            CollabAgentStatus::Interrupted => "interrupted",
            CollabAgentStatus::Completed => "completed",
            CollabAgentStatus::Errored => "errored",
            CollabAgentStatus::Shutdown => "shutdown",
            CollabAgentStatus::NotFound => "notFound",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct CollabAgentState {
    pub status: CollabAgentStatus,
    pub message: Option<String>,
}

impl ToJson for CollabAgentState {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(2);
        object.push_field("status", self.status.to_json());
        object.push_field("message", self.message.to_json());
        JsonValue::Object(object)
    }
}

impl From<CoreAgentStatus> for CollabAgentState {
    fn from(value: CoreAgentStatus) -> Self {
        match value {
            CoreAgentStatus::PendingInit => Self {
                status: CollabAgentStatus::PendingInit,
                message: None,
            },
            CoreAgentStatus::Running => Self {
                status: CollabAgentStatus::Running,
                message: None,
            },
            CoreAgentStatus::Interrupted => Self {
                status: CollabAgentStatus::Interrupted,
                message: None,
            },
            CoreAgentStatus::Completed(message) => Self {
                status: CollabAgentStatus::Completed,
                message,
            },
            CoreAgentStatus::Errored(message) => Self {
                status: CollabAgentStatus::Errored,
                message: Some(message),
            },
            CoreAgentStatus::Shutdown => Self {
                status: CollabAgentStatus::Shutdown,
                message: None,
            },
            CoreAgentStatus::NotFound => Self {
                status: CollabAgentStatus::NotFound,
                message: None,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct ItemStartedNotification {
    pub item: ThreadItem,
    pub thread_id: String,
    pub turn_id: String,
    /// Unix timestamp (in milliseconds) when this item lifecycle started.
    pub started_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
/// [UNSTABLE] Temporary notification payload for approval auto-review. This
/// shape is expected to change soon.
pub struct ItemGuardianApprovalReviewStartedNotification {
    pub thread_id: String,
    pub turn_id: String,
    /// Unix timestamp (in milliseconds) when this review started.
    pub started_at_ms: i64,
    /// Stable identifier for this review.
    pub review_id: String,
    /// Identifier for the reviewed item or tool call when one exists.
    ///
    /// In most cases, one review maps to one target item. The exceptions are
    /// - execve reviews, where a single command may contain multiple execve
    ///   calls to review (only possible when using the shell_zsh_fork feature)
    /// - network policy reviews, where there is no target item
    ///
    /// A network call is triggered by a CommandExecution item, so having a
    /// target_item_id set to the CommandExecution item would be misleading
    /// because the review is about the network call, not the command execution.
    /// Therefore, target_item_id is set to None for network policy reviews.
    pub target_item_id: Option<String>,
    pub review: GuardianApprovalReview,
    pub action: GuardianApprovalReviewAction,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
/// [UNSTABLE] Temporary notification payload for approval auto-review. This
/// shape is expected to change soon.
pub struct ItemGuardianApprovalReviewCompletedNotification {
    pub thread_id: String,
    pub turn_id: String,
    /// Unix timestamp (in milliseconds) when this review started.
    pub started_at_ms: i64,
    /// Unix timestamp (in milliseconds) when this review completed.
    pub completed_at_ms: i64,
    /// Stable identifier for this review.
    pub review_id: String,
    /// Identifier for the reviewed item or tool call when one exists.
    ///
    /// In most cases, one review maps to one target item. The exceptions are
    /// - execve reviews, where a single command may contain multiple execve
    ///   calls to review (only possible when using the shell_zsh_fork feature)
    /// - network policy reviews, where there is no target item
    ///
    /// A network call is triggered by a CommandExecution item, so having a
    /// target_item_id set to the CommandExecution item would be misleading
    /// because the review is about the network call, not the command execution.
    /// Therefore, target_item_id is set to None for network policy reviews.
    pub target_item_id: Option<String>,
    pub decision_source: AutoReviewDecisionSource,
    pub review: GuardianApprovalReview,
    pub action: GuardianApprovalReviewAction,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct ItemCompletedNotification {
    pub item: ThreadItem,
    pub thread_id: String,
    pub turn_id: String,
    /// Unix timestamp (in milliseconds) when this item lifecycle completed.
    pub completed_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct RawResponseItemCompletedNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item: ResponseItem,
}

// Item-specific progress notifications
#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct AgentMessageDeltaNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub delta: String,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
/// EXPERIMENTAL - proposed plan streaming deltas for plan items. Clients should
/// not assume concatenated deltas match the completed plan item content.
pub struct PlanDeltaNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub delta: String,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningSummaryTextDeltaNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub delta: String,
    pub summary_index: i64,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningSummaryPartAddedNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub summary_index: i64,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningTextDeltaNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub delta: String,
    pub content_index: i64,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct TerminalInteractionNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub process_id: String,
    pub stdin: String,
}

#[derive(Debug, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommandExecutionOutputDeltaNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub delta: String,
}

impl ToJson for CommandExecutionOutputDeltaNotification {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(4);
        object.push_field("threadId", self.thread_id.clone());
        object.push_field("turnId", self.turn_id.clone());
        object.push_field("itemId", self.item_id.clone());
        object.push_field("delta", self.delta.clone());
        JsonValue::Object(object)
    }
}

impl FromJson for CommandExecutionOutputDeltaNotification {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object: Map = Map::try_from(value)?;
        Ok(Self {
            thread_id: object.take_required("threadId")?,
            turn_id: object.take_required("turnId")?,
            item_id: object.take_required("itemId")?,
            delta: object.take_required("delta")?,
        })
    }
}

fn optional_absolute_path(
    value: Option<JsonValue>,
) -> Result<Option<AbsolutePathBuf>, JsonValueError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let path = String::from_json(value)?;
    AbsolutePathBuf::try_from(PathBuf::from(&path))
        .map(Some)
        .map_err(|_| JsonValueError::WrongType(format!("expected absolute path, found `{path}`")))
}

fn absolute_path_json(path: &AbsolutePathBuf) -> JsonValue {
    JsonValue::from(path.as_path().display().to_string())
}

fn absolute_paths_json(paths: &[AbsolutePathBuf]) -> JsonValue {
    JsonValue::Array(paths.iter().map(absolute_path_json).collect())
}

fn optional_absolute_paths(
    value: Option<JsonValue>,
) -> Result<Option<Vec<AbsolutePathBuf>>, JsonValueError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    Vec::<String>::from_json(value)?
        .into_iter()
        .map(|path| {
            AbsolutePathBuf::try_from(PathBuf::from(&path)).map_err(|_| {
                JsonValueError::WrongType(format!("expected absolute path, found `{path}`"))
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn required_absolute_paths(
    value: Option<JsonValue>,
    field: &str,
) -> Result<Vec<AbsolutePathBuf>, JsonValueError> {
    optional_absolute_paths(value)?
        .ok_or_else(|| JsonValueError::WrongType(format!("missing required field `{field}`")))
}

fn required_absolute_path(
    value: Option<JsonValue>,
    field: &str,
) -> Result<AbsolutePathBuf, JsonValueError> {
    optional_absolute_path(value)?
        .ok_or_else(|| JsonValueError::WrongType(format!("missing required field `{field}`")))
}

/// Deprecated legacy notification for `apply_patch` textual output.
///
/// The server no longer emits this notification.
#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct FileChangeOutputDeltaNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub delta: String,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct FileChangePatchUpdatedNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub changes: Vec<FileUpdateChange>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson)]
#[serde(rename_all = "camelCase")]
pub struct CommandExecutionRequestApprovalParams {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    /// Unix timestamp (in milliseconds) when this approval request started.
    pub started_at_ms: i64,
    /// Unique identifier for this specific approval callback.
    ///
    /// For regular shell/unified_exec approvals, this is null.
    ///
    /// For zsh-exec-bridge subcommand approvals, multiple callbacks can belong to
    /// one parent `itemId`, so `approvalId` is a distinct opaque callback id
    /// (a UUID) used to disambiguate routing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_id: Option<String>,
    /// Optional explanatory reason (e.g. request for network access).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Optional context for a managed-network approval prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_approval_context: Option<NetworkApprovalContext>,
    /// The command to be executed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// The command's working directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<AbsolutePathBuf>,
    /// Best-effort parsed command actions for friendly display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_actions: Option<Vec<CommandAction>>,
    /// Optional proposed execpolicy amendment to allow similar commands without prompting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposed_execpolicy_amendment: Option<ExecPolicyAmendment>,
    /// Optional proposed network policy amendments (allow/deny host) for future requests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposed_network_policy_amendments: Option<Vec<NetworkPolicyAmendment>>,
    /// Ordered list of decisions the client may present for this prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub available_decisions: Option<Vec<CommandExecutionApprovalDecision>>,
    /// [UNSTABLE] Additional filesystem permissions requested for this command.
    pub additional_permissions: Option<AdditionalFileSystemPermissions>,
}

impl FromJson for CommandExecutionRequestApprovalParams {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("CommandExecutionRequestApprovalParams")?;
        Ok(Self {
            thread_id: object.take_required("threadId")?,
            turn_id: object.take_required("turnId")?,
            item_id: object.take_required("itemId")?,
            started_at_ms: object.take_required("startedAtMs")?,
            approval_id: object.take_optional("approvalId")?,
            reason: object.take_optional("reason")?,
            network_approval_context: object.take_optional("networkApprovalContext")?,
            command: object.take_optional("command")?,
            cwd: optional_absolute_path(object.remove("cwd"))?,
            command_actions: object.take_optional("commandActions")?,
            proposed_execpolicy_amendment: object.take_optional("proposedExecpolicyAmendment")?,
            proposed_network_policy_amendments: object
                .take_optional("proposedNetworkPolicyAmendments")?,
            available_decisions: object.take_optional("availableDecisions")?,
            additional_permissions: object.take_optional("additionalPermissions")?,
        })
    }
}

impl CommandExecutionRequestApprovalParams {
    pub fn strip_experimental_fields(&mut self) {
        self.additional_permissions = None;
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct CommandExecutionRequestApprovalResponse {
    pub decision: CommandExecutionApprovalDecision,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct FileChangeRequestApprovalParams {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    /// Unix timestamp (in milliseconds) when this approval request started.
    pub started_at_ms: i64,
    /// Optional explanatory reason (e.g. request for extra write access).
    pub reason: Option<String>,
    /// [UNSTABLE] When set, the agent is asking the user to allow writes under this root
    /// for the remainder of the session (unclear if this is honored today).
    pub grant_root: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
pub struct FileChangeRequestApprovalResponse {
    pub decision: FileChangeApprovalDecision,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct DynamicToolCallParams {
    pub thread_id: String,
    pub turn_id: String,
    pub call_id: String,
    pub namespace: Option<String>,
    pub tool: String,
    pub arguments: JsonValue,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct DynamicToolCallResponse {
    pub content_items: Vec<DynamicToolCallOutputContentItem>,
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DynamicToolCallOutputContentItem {
    #[serde(rename_all = "camelCase")]
    InputText { text: String },
    #[serde(rename_all = "camelCase")]
    InputImage { image_url: String },
}

impl ToJson for DynamicToolCallOutputContentItem {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::new();
        match self {
            Self::InputText { text } => {
                object.push_field("type", "inputText");
                object.push_field("text", text);
            }
            Self::InputImage { image_url } => {
                object.push_field("type", "inputImage");
                object.push_field("imageUrl", image_url);
            }
        }
        JsonValue::Object(object)
    }
}

impl ToJson for DynamicToolCallResponse {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::new();
        object.push_field("contentItems", self.content_items.to_json());
        object.push_field("success", self.success);
        JsonValue::Object(object)
    }
}

impl From<DynamicToolCallOutputContentItem>
    for codex_protocol::dynamic_tools::DynamicToolCallOutputContentItem
{
    fn from(item: DynamicToolCallOutputContentItem) -> Self {
        match item {
            DynamicToolCallOutputContentItem::InputText { text } => Self::InputText { text },
            DynamicToolCallOutputContentItem::InputImage { image_url } => {
                Self::InputImage { image_url }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
/// EXPERIMENTAL. Defines a single selectable option for request_user_input.
pub struct ToolRequestUserInputOption {
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
/// EXPERIMENTAL. Represents one request_user_input question and its required options.
pub struct ToolRequestUserInputQuestion {
    pub id: String,
    pub header: String,
    pub question: String,
    #[serde(default)]
    pub is_other: bool,
    #[serde(default)]
    pub is_secret: bool,
    pub options: Option<Vec<ToolRequestUserInputOption>>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
/// EXPERIMENTAL. Params sent with a request_user_input event.
pub struct ToolRequestUserInputParams {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub questions: Vec<ToolRequestUserInputQuestion>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
/// EXPERIMENTAL. Captures a user's answer to a request_user_input question.
pub struct ToolRequestUserInputAnswer {
    pub answers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
/// EXPERIMENTAL. Response payload mapping question ids to answers.
pub struct ToolRequestUserInputResponse {
    pub answers: HashMap<String, ToolRequestUserInputAnswer>,
}
