use super::ApprovalsReviewer;
use super::AskForApproval;
use super::PermissionProfile;
use super::PermissionProfileSelectionParams;
use super::SandboxMode;
use super::SandboxPolicy;
use super::Thread;
use super::ThreadItem;
use super::ThreadSource;
use super::Turn;
use super::TurnEnvironmentParams;
use super::TurnItemsView;
use super::shared::v2_enum_from_core;
use codex_protocol::compat::absolute_path::AbsolutePathBuf;
use codex_protocol::config_types::Personality;
use codex_protocol::models::ResponseItem;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::protocol::ThreadGoalStatus as CoreThreadGoalStatus;
use codex_protocol::protocol::TokenUsage as CoreTokenUsage;
use codex_protocol::protocol::TokenUsageInfo as CoreTokenUsageInfo;
use edgerun_json::FromJson;
use edgerun_json::JsonValueError;
use edgerun_json::Map;
use edgerun_json::ToJson;
use edgerun_json::Value as JsonValue;
use edgerun_serde::Deserialize;
use edgerun_serde::Serialize;
use schemars::JsonSchema;
use std::collections::HashMap;
use std::path::PathBuf;
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum ThreadStartSource {
    Startup,
    Clear,
}

impl ToJson for ThreadStartSource {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            ThreadStartSource::Startup => "startup",
            ThreadStartSource::Clear => "clear",
        })
    }
}

impl FromJson for ThreadStartSource {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "startup" => Ok(ThreadStartSource::Startup),
            "clear" => Ok(ThreadStartSource::Clear),
            other => Err(JsonValueError::WrongType(format!(
                "unknown thread start source `{other}`"
            ))),
        }
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct DynamicToolSpec {
    #[ts(optional)]
    pub namespace: Option<String>,
    pub name: String,
    pub description: String,
    pub input_schema: JsonValue,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub defer_loading: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DynamicToolSpecDe {
    namespace: Option<String>,
    name: String,
    description: String,
    input_schema: JsonValue,
    defer_loading: Option<bool>,
    expose_to_context: Option<bool>,
}

impl<'de> Deserialize<'de> for DynamicToolSpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: edgerun_serde::Deserializer<'de>,
    {
        let DynamicToolSpecDe {
            namespace,
            name,
            description,
            input_schema,
            defer_loading,
            expose_to_context,
        } = DynamicToolSpecDe::deserialize(deserializer)?;

        Ok(Self {
            namespace,
            name,
            description,
            input_schema,
            defer_loading: defer_loading
                .unwrap_or_else(|| expose_to_context.map(|visible| !visible).unwrap_or(false)),
        })
    }
}

impl ToJson for DynamicToolSpec {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::new();
        if let Some(namespace) = &self.namespace {
            object.push_field("namespace", namespace);
        }
        object.push_field("name", &self.name);
        object.push_field("description", &self.description);
        object.push_field("inputSchema", self.input_schema.clone());
        if self.defer_loading {
            object.push_field("deferLoading", true);
        }
        JsonValue::Object(object)
    }
}

impl FromJson for DynamicToolSpec {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("DynamicToolSpec")?;
        let defer_loading = match object.take_optional("deferLoading")? {
            Some(value) => value,
            None => object
                .take_optional::<bool>("exposeToContext")?
                .map(|visible| !visible)
                .unwrap_or(false),
        };
        Ok(Self {
            namespace: object.take_optional("namespace")?,
            name: object.take_required("name")?,
            description: object.take_required("description")?,
            input_schema: object.take_required("inputSchema")?,
            defer_loading,
        })
    }
}

// === Threads, Turns, and Items ===
// Thread APIs
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadStartParams {
    #[ts(optional = nullable)]
    pub model: Option<String>,
    #[ts(optional = nullable)]
    pub model_provider: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::protocol::serde_helpers::deserialize_double_option",
        serialize_with = "crate::protocol::serde_helpers::serialize_double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[ts(optional = nullable)]
    pub service_tier: Option<Option<String>>,
    #[ts(optional = nullable)]
    pub cwd: Option<String>,
    #[ts(optional = nullable)]
    pub approval_policy: Option<AskForApproval>,
    /// Override where approval requests are routed for review on this thread
    /// and subsequent turns.
    #[ts(optional = nullable)]
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    #[ts(optional = nullable)]
    pub sandbox: Option<SandboxMode>,
    /// Named profile selection for this thread. Cannot be combined with
    /// `sandbox`.
    #[ts(optional = nullable)]
    pub permissions: Option<PermissionProfileSelectionParams>,
    #[ts(optional = nullable)]
    pub config: Option<HashMap<String, JsonValue>>,
    #[ts(optional = nullable)]
    pub service_name: Option<String>,
    #[ts(optional = nullable)]
    pub base_instructions: Option<String>,
    #[ts(optional = nullable)]
    pub developer_instructions: Option<String>,
    #[ts(optional = nullable)]
    pub personality: Option<Personality>,
    #[ts(optional = nullable)]
    pub ephemeral: Option<bool>,
    #[ts(optional = nullable)]
    pub session_start_source: Option<ThreadStartSource>,
    /// Optional client-supplied analytics source classification for this thread.
    #[ts(optional = nullable)]
    pub thread_source: Option<ThreadSource>,
    /// Optional sticky environments for this thread.
    ///
    /// Omitted selects the default environment when environment access is
    /// enabled. Empty disables environment access for turns that do not
    /// provide a turn override. Non-empty selects the first environment as the
    /// current turn environment.
    #[ts(optional = nullable)]
    pub environments: Option<Vec<TurnEnvironmentParams>>,
    #[ts(optional = nullable)]
    pub dynamic_tools: Option<Vec<DynamicToolSpec>>,
    /// Test-only experimental field used to validate experimental gating and
    /// schema filtering behavior in a stable way.
    #[ts(optional = nullable)]
    pub mock_experimental_field: Option<String>,
    /// If true, opt into emitting raw Responses API items on the event stream.
    /// This is for internal use only (e.g. Codex Cloud).
    #[serde(default)]
    pub experimental_raw_events: bool,
    /// Deprecated and ignored by app-server. Kept only so older clients can
    /// continue sending the field while rollout persistence always uses the
    /// limited history policy.
    #[serde(default)]
    pub persist_extended_history: bool,
}

impl ToJson for ThreadStartParams {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(23);
        object.push_opt_field("model", self.model.clone());
        object.push_opt_field("modelProvider", self.model_provider.clone());
        if let Some(service_tier) = &self.service_tier {
            object.push_field(
                "serviceTier",
                service_tier
                    .as_ref()
                    .map_or(JsonValue::Null, |tier| JsonValue::String(tier.clone())),
            );
        }
        object.push_opt_field("cwd", self.cwd.clone());
        object.push_opt_field(
            "approvalPolicy",
            self.approval_policy.map(|value| value.to_json()),
        );
        object.push_opt_field(
            "approvalsReviewer",
            self.approvals_reviewer.map(|value| value.to_json()),
        );
        object.push_opt_field("sandbox", self.sandbox.map(|value| value.to_json()));
        object.push_opt_field(
            "permissions",
            self.permissions.as_ref().map(ToJson::to_json),
        );
        object.push_opt_field("config", self.config.as_ref().map(ToJson::to_json));
        object.push_opt_field("serviceName", self.service_name.clone());
        object.push_opt_field("baseInstructions", self.base_instructions.clone());
        object.push_opt_field("developerInstructions", self.developer_instructions.clone());
        object.push_opt_field("personality", self.personality.map(|value| value.to_json()));
        object.push_opt_field("ephemeral", self.ephemeral);
        object.push_opt_field(
            "sessionStartSource",
            self.session_start_source.map(|value| value.to_json()),
        );
        object.push_opt_field(
            "threadSource",
            self.thread_source.map(|value| value.to_json()),
        );
        object.push_opt_field(
            "environments",
            self.environments.as_ref().map(ToJson::to_json),
        );
        object.push_opt_field(
            "dynamicTools",
            self.dynamic_tools.as_ref().map(ToJson::to_json),
        );
        object.push_opt_field(
            "mockExperimentalField",
            self.mock_experimental_field.clone(),
        );
        object.push_field("experimentalRawEvents", self.experimental_raw_events);
        object.push_field("persistExtendedHistory", self.persist_extended_history);
        JsonValue::Object(object)
    }
}

impl FromJson for ThreadStartParams {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("ThreadStartParams")?;
        let service_tier = match object.remove("serviceTier") {
            Some(JsonValue::Null) => Some(None),
            Some(value) => Some(Some(String::from_json(value)?)),
            None => None,
        };

        Ok(Self {
            model: object.take_optional("model")?,
            model_provider: object.take_optional("modelProvider")?,
            service_tier,
            cwd: object.take_optional("cwd")?,
            approval_policy: object.take_optional("approvalPolicy")?,
            approvals_reviewer: object.take_optional("approvalsReviewer")?,
            sandbox: object.take_optional("sandbox")?,
            permissions: object.take_optional("permissions")?,
            config: object.take_optional("config")?,
            service_name: object.take_optional("serviceName")?,
            base_instructions: object.take_optional("baseInstructions")?,
            developer_instructions: object.take_optional("developerInstructions")?,
            personality: object.take_optional("personality")?,
            ephemeral: object.take_optional("ephemeral")?,
            session_start_source: object.take_optional("sessionStartSource")?,
            thread_source: object.take_optional("threadSource")?,
            environments: object.take_optional("environments")?,
            dynamic_tools: object.take_optional("dynamicTools")?,
            mock_experimental_field: object.take_optional("mockExperimentalField")?,
            experimental_raw_events: object
                .take_optional("experimentalRawEvents")?
                .unwrap_or(false),
            persist_extended_history: object
                .take_optional("persistExtendedHistory")?
                .unwrap_or(false),
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct MockExperimentalMethodParams {
    /// Test-only payload field.
    #[ts(optional = nullable)]
    pub value: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct MockExperimentalMethodResponse {
    /// Echoes the input `value`.
    pub echoed: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadStartResponse {
    pub thread: Thread,
    pub model: String,
    pub model_provider: String,
    pub service_tier: Option<String>,
    pub cwd: AbsolutePathBuf,
    /// Instruction source files currently loaded for this thread.
    #[serde(default)]
    pub instruction_sources: Vec<AbsolutePathBuf>,
    pub approval_policy: AskForApproval,
    /// Reviewer currently used for approval requests on this thread.
    pub approvals_reviewer: ApprovalsReviewer,
    /// Legacy sandbox policy retained for compatibility. Experimental clients
    /// should prefer `permissionProfile` when they need exact runtime
    /// permissions.
    pub sandbox: SandboxPolicy,
    /// Full active permissions for this thread. `activePermissionProfile`
    /// carries display/provenance metadata for this runtime profile.
    #[serde(default)]
    pub permission_profile: Option<PermissionProfile>,
    pub reasoning_effort: Option<ReasoningEffort>,
}

impl FromJson for ThreadStartResponse {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        thread_lifecycle_response_from_json(value).map(
            |ThreadLifecycleResponse {
                 thread,
                 model,
                 model_provider,
                 service_tier,
                 cwd,
                 instruction_sources,
                 approval_policy,
                 approvals_reviewer,
                 sandbox,
                 permission_profile,
                 reasoning_effort,
             }| Self {
                thread,
                model,
                model_provider,
                service_tier,
                cwd,
                instruction_sources,
                approval_policy,
                approvals_reviewer,
                sandbox,
                permission_profile,
                reasoning_effort,
            },
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
/// There are three ways to resume a thread:
/// 1. By thread_id: load the thread from disk by thread_id and resume it.
/// 2. By history: instantiate the thread from memory and resume it.
/// 3. By path: load the thread from disk by path and resume it.
///
/// The precedence is: history > path > thread_id.
/// If using history or path, the thread_id param will be ignored.
///
/// Prefer using thread_id whenever possible.
pub struct ThreadResumeParams {
    pub thread_id: String,

    /// [UNSTABLE] FOR CODEX CLOUD - DO NOT USE.
    /// If specified, the thread will be resumed with the provided history
    /// instead of loaded from disk.
    #[ts(optional = nullable)]
    pub history: Option<Vec<ResponseItem>>,

    /// [UNSTABLE] Specify the rollout path to resume from.
    /// If specified, the thread_id param will be ignored.
    #[ts(optional = nullable)]
    pub path: Option<PathBuf>,

    /// Configuration overrides for the resumed thread, if any.
    #[ts(optional = nullable)]
    pub model: Option<String>,
    #[ts(optional = nullable)]
    pub model_provider: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::protocol::serde_helpers::deserialize_double_option",
        serialize_with = "crate::protocol::serde_helpers::serialize_double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[ts(optional = nullable)]
    pub service_tier: Option<Option<String>>,
    #[ts(optional = nullable)]
    pub cwd: Option<String>,
    #[ts(optional = nullable)]
    pub approval_policy: Option<AskForApproval>,
    /// Override where approval requests are routed for review on this thread
    /// and subsequent turns.
    #[ts(optional = nullable)]
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    #[ts(optional = nullable)]
    pub sandbox: Option<SandboxMode>,
    /// Named profile selection for the resumed thread. Cannot be combined
    /// with `sandbox`.
    #[ts(optional = nullable)]
    pub permissions: Option<PermissionProfileSelectionParams>,
    #[ts(optional = nullable)]
    pub config: Option<HashMap<String, edgerun_json::Value>>,
    #[ts(optional = nullable)]
    pub base_instructions: Option<String>,
    #[ts(optional = nullable)]
    pub developer_instructions: Option<String>,
    #[ts(optional = nullable)]
    pub personality: Option<Personality>,
    /// When true, return only thread metadata and live-resume state without
    /// populating `thread.turns`. This is useful when the client plans to call
    /// `thread/turns/list` immediately after resuming.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub exclude_turns: bool,
    /// Deprecated and ignored by app-server. Kept only so older clients can
    /// continue sending the field while rollout persistence always uses the
    /// limited history policy.
    #[serde(default)]
    pub persist_extended_history: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadResumeResponse {
    pub thread: Thread,
    pub model: String,
    pub model_provider: String,
    pub service_tier: Option<String>,
    pub cwd: AbsolutePathBuf,
    /// Instruction source files currently loaded for this thread.
    #[serde(default)]
    pub instruction_sources: Vec<AbsolutePathBuf>,
    pub approval_policy: AskForApproval,
    /// Reviewer currently used for approval requests on this thread.
    pub approvals_reviewer: ApprovalsReviewer,
    /// Legacy sandbox policy retained for compatibility. Experimental clients
    /// should prefer `permissionProfile` when they need exact runtime
    /// permissions.
    pub sandbox: SandboxPolicy,
    /// Full active permissions for this thread. `activePermissionProfile`
    /// carries display/provenance metadata for this runtime profile.
    #[serde(default)]
    pub permission_profile: Option<PermissionProfile>,
    pub reasoning_effort: Option<ReasoningEffort>,
}

impl FromJson for ThreadResumeResponse {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        thread_lifecycle_response_from_json(value).map(
            |ThreadLifecycleResponse {
                 thread,
                 model,
                 model_provider,
                 service_tier,
                 cwd,
                 instruction_sources,
                 approval_policy,
                 approvals_reviewer,
                 sandbox,
                 permission_profile,
                 reasoning_effort,
             }| Self {
                thread,
                model,
                model_provider,
                service_tier,
                cwd,
                instruction_sources,
                approval_policy,
                approvals_reviewer,
                sandbox,
                permission_profile,
                reasoning_effort,
            },
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
/// There are two ways to fork a thread:
/// 1. By thread_id: load the thread from disk by thread_id and fork it into a new thread.
/// 2. By path: load the thread from disk by path and fork it into a new thread.
///
/// If using path, the thread_id param will be ignored.
///
/// Prefer using thread_id whenever possible.
pub struct ThreadForkParams {
    pub thread_id: String,

    /// [UNSTABLE] Specify the rollout path to fork from.
    /// If specified, the thread_id param will be ignored.
    #[ts(optional = nullable)]
    pub path: Option<PathBuf>,

    /// Configuration overrides for the forked thread, if any.
    #[ts(optional = nullable)]
    pub model: Option<String>,
    #[ts(optional = nullable)]
    pub model_provider: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::protocol::serde_helpers::deserialize_double_option",
        serialize_with = "crate::protocol::serde_helpers::serialize_double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[ts(optional = nullable)]
    pub service_tier: Option<Option<String>>,
    #[ts(optional = nullable)]
    pub cwd: Option<String>,
    #[ts(optional = nullable)]
    pub approval_policy: Option<AskForApproval>,
    /// Override where approval requests are routed for review on this thread
    /// and subsequent turns.
    #[ts(optional = nullable)]
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    #[ts(optional = nullable)]
    pub sandbox: Option<SandboxMode>,
    /// Named profile selection for the forked thread. Cannot be combined with
    /// `sandbox`.
    #[ts(optional = nullable)]
    pub permissions: Option<PermissionProfileSelectionParams>,
    #[ts(optional = nullable)]
    pub config: Option<HashMap<String, edgerun_json::Value>>,
    #[ts(optional = nullable)]
    pub base_instructions: Option<String>,
    #[ts(optional = nullable)]
    pub developer_instructions: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ephemeral: bool,
    /// Optional client-supplied analytics source classification for this forked thread.
    #[ts(optional = nullable)]
    pub thread_source: Option<ThreadSource>,
    /// When true, return only thread metadata and live fork state without
    /// populating `thread.turns`. This is useful when the client plans to call
    /// `thread/turns/list` immediately after forking.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub exclude_turns: bool,
    /// Deprecated and ignored by app-server. Kept only so older clients can
    /// continue sending the field while rollout persistence always uses the
    /// limited history policy.
    #[serde(default)]
    pub persist_extended_history: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadForkResponse {
    pub thread: Thread,
    pub model: String,
    pub model_provider: String,
    pub service_tier: Option<String>,
    pub cwd: AbsolutePathBuf,
    /// Instruction source files currently loaded for this thread.
    #[serde(default)]
    pub instruction_sources: Vec<AbsolutePathBuf>,
    pub approval_policy: AskForApproval,
    /// Reviewer currently used for approval requests on this thread.
    pub approvals_reviewer: ApprovalsReviewer,
    /// Legacy sandbox policy retained for compatibility. Experimental clients
    /// should prefer `permissionProfile` when they need exact runtime
    /// permissions.
    pub sandbox: SandboxPolicy,
    /// Full active permissions for this thread. `activePermissionProfile`
    /// carries display/provenance metadata for this runtime profile.
    #[serde(default)]
    pub permission_profile: Option<PermissionProfile>,
    pub reasoning_effort: Option<ReasoningEffort>,
}

impl FromJson for ThreadForkResponse {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        thread_lifecycle_response_from_json(value).map(
            |ThreadLifecycleResponse {
                 thread,
                 model,
                 model_provider,
                 service_tier,
                 cwd,
                 instruction_sources,
                 approval_policy,
                 approvals_reviewer,
                 sandbox,
                 permission_profile,
                 reasoning_effort,
             }| Self {
                thread,
                model,
                model_provider,
                service_tier,
                cwd,
                instruction_sources,
                approval_policy,
                approvals_reviewer,
                sandbox,
                permission_profile,
                reasoning_effort,
            },
        )
    }
}

struct ThreadLifecycleResponse {
    thread: Thread,
    model: String,
    model_provider: String,
    service_tier: Option<String>,
    cwd: AbsolutePathBuf,
    instruction_sources: Vec<AbsolutePathBuf>,
    approval_policy: AskForApproval,
    approvals_reviewer: ApprovalsReviewer,
    sandbox: SandboxPolicy,
    permission_profile: Option<PermissionProfile>,
    reasoning_effort: Option<ReasoningEffort>,
}

fn thread_lifecycle_response_from_json(
    value: JsonValue,
) -> Result<ThreadLifecycleResponse, JsonValueError> {
    let mut object = value.into_object("ThreadLifecycleResponse")?;
    Ok(ThreadLifecycleResponse {
        thread: object.take_required("thread")?,
        model: object.take_required("model")?,
        model_provider: object.take_required("modelProvider")?,
        service_tier: object.take_optional("serviceTier")?,
        cwd: object.take_required("cwd")?,
        instruction_sources: object
            .take_optional("instructionSources")?
            .unwrap_or_default(),
        approval_policy: object.take_required("approvalPolicy")?,
        approvals_reviewer: object.take_required("approvalsReviewer")?,
        sandbox: object.take_required("sandbox")?,
        permission_profile: object.take_optional("permissionProfile")?,
        reasoning_effort: object.take_optional("reasoningEffort")?,
    })
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadArchiveParams {
    pub thread_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadArchiveResponse {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadUnsubscribeParams {
    pub thread_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadUnsubscribeResponse {
    pub status: ThreadUnsubscribeStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum ThreadUnsubscribeStatus {
    NotLoaded,
    NotSubscribed,
    Unsubscribed,
}

/// Parameters for `thread/increment_elicitation`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadIncrementElicitationParams {
    /// Thread whose out-of-band elicitation counter should be incremented.
    pub thread_id: String,
}

/// Response for `thread/increment_elicitation`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadIncrementElicitationResponse {
    /// Current out-of-band elicitation count after the increment.
    pub count: u64,
    /// Whether timeout accounting is paused after applying the increment.
    pub paused: bool,
}

/// Parameters for `thread/decrement_elicitation`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadDecrementElicitationParams {
    /// Thread whose out-of-band elicitation counter should be decremented.
    pub thread_id: String,
}

/// Response for `thread/decrement_elicitation`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadDecrementElicitationResponse {
    /// Current out-of-band elicitation count after the decrement.
    pub count: u64,
    /// Whether timeout accounting remains paused after applying the decrement.
    pub paused: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadSetNameParams {
    pub thread_id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadUnarchiveParams {
    pub thread_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadSetNameResponse {}

v2_enum_from_core! {
    pub enum ThreadGoalStatus from CoreThreadGoalStatus {
        Active,
        Paused,
        BudgetLimited,
        Complete,
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadGoal {
    pub thread_id: String,
    pub objective: String,
    pub status: ThreadGoalStatus,
    #[ts(type = "number | null")]
    pub token_budget: Option<i64>,
    #[ts(type = "number")]
    pub tokens_used: i64,
    #[ts(type = "number")]
    pub time_used_seconds: i64,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
}

impl From<codex_protocol::protocol::ThreadGoal> for ThreadGoal {
    fn from(value: codex_protocol::protocol::ThreadGoal) -> Self {
        Self {
            thread_id: value.thread_id.to_string(),
            objective: value.objective,
            status: value.status.into(),
            token_budget: value.token_budget,
            tokens_used: value.tokens_used,
            time_used_seconds: value.time_used_seconds,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadGoalSetParams {
    pub thread_id: String,
    #[ts(optional = nullable)]
    pub objective: Option<String>,
    #[ts(optional = nullable)]
    pub status: Option<ThreadGoalStatus>,
    #[serde(
        default,
        deserialize_with = "crate::protocol::serde_helpers::deserialize_double_option",
        serialize_with = "crate::protocol::serde_helpers::serialize_double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[ts(optional = nullable, type = "number | null")]
    pub token_budget: Option<Option<i64>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadGoalSetResponse {
    pub goal: ThreadGoal,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadGoalGetParams {
    pub thread_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadGoalGetResponse {
    pub goal: Option<ThreadGoal>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadGoalClearParams {
    pub thread_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadGoalClearResponse {
    pub cleared: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadMetadataUpdateParams {
    pub thread_id: String,
    /// Patch the stored Git metadata for this thread.
    /// Omit a field to leave it unchanged, set it to `null` to clear it, or
    /// provide a string to replace the stored value.
    #[ts(optional = nullable)]
    pub git_info: Option<ThreadMetadataGitInfoUpdateParams>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadMetadataGitInfoUpdateParams {
    /// Omit to leave the stored commit unchanged, set to `null` to clear it,
    /// or provide a non-empty string to replace it.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "crate::protocol::serde_helpers::serialize_double_option",
        deserialize_with = "crate::protocol::serde_helpers::deserialize_double_option"
    )]
    #[ts(optional = nullable, type = "string | null")]
    pub sha: Option<Option<String>>,
    /// Omit to leave the stored branch unchanged, set to `null` to clear it,
    /// or provide a non-empty string to replace it.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "crate::protocol::serde_helpers::serialize_double_option",
        deserialize_with = "crate::protocol::serde_helpers::deserialize_double_option"
    )]
    #[ts(optional = nullable, type = "string | null")]
    pub branch: Option<Option<String>>,
    /// Omit to leave the stored origin URL unchanged, set to `null` to clear it,
    /// or provide a non-empty string to replace it.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "crate::protocol::serde_helpers::serialize_double_option",
        deserialize_with = "crate::protocol::serde_helpers::deserialize_double_option"
    )]
    #[ts(optional = nullable, type = "string | null")]
    pub origin_url: Option<Option<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadMetadataUpdateResponse {
    pub thread: Thread,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
#[ts(rename_all = "lowercase")]
pub enum ThreadMemoryMode {
    Enabled,
    Disabled,
}

impl ThreadMemoryMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
        }
    }

    pub fn to_core(self) -> codex_protocol::protocol::ThreadMemoryMode {
        match self {
            Self::Enabled => codex_protocol::protocol::ThreadMemoryMode::Enabled,
            Self::Disabled => codex_protocol::protocol::ThreadMemoryMode::Disabled,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadMemoryModeSetParams {
    pub thread_id: String,
    pub mode: ThreadMemoryMode,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadMemoryModeSetResponse {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct MemoryResetResponse {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadUnarchiveResponse {
    pub thread: Thread,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadCompactStartParams {
    pub thread_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadCompactStartResponse {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadShellCommandParams {
    pub thread_id: String,
    /// Shell command string evaluated by the thread's configured shell.
    /// Unlike `command/exec`, this intentionally preserves shell syntax
    /// such as pipes, redirects, and quoting. This runs unsandboxed with full
    /// access rather than inheriting the thread sandbox policy.
    pub command: String,
}

impl ToJson for ThreadShellCommandParams {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::new();
        object.push_field("threadId", &self.thread_id);
        object.push_field("command", &self.command);
        JsonValue::Object(object)
    }
}

impl FromJson for ThreadShellCommandParams {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("ThreadShellCommandParams")?;
        Ok(Self {
            thread_id: object.take_required("threadId")?,
            command: object.take_required("command")?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadShellCommandResponse {}

impl ToJson for ThreadShellCommandResponse {
    fn to_json(&self) -> JsonValue {
        JsonValue::Object(Map::new())
    }
}

impl FromJson for ThreadShellCommandResponse {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let _ = value.into_object("ThreadShellCommandResponse")?;
        Ok(Self {})
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadApproveGuardianDeniedActionParams {
    pub thread_id: String,
    /// Serialized `codex_protocol::protocol::GuardianAssessmentEvent`.
    pub event: JsonValue,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadApproveGuardianDeniedActionResponse {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadBackgroundTerminalsCleanParams {
    pub thread_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadBackgroundTerminalsCleanResponse {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadRollbackParams {
    pub thread_id: String,
    /// The number of turns to drop from the end of the thread. Must be >= 1.
    ///
    /// This only modifies the thread's history and does not revert local file changes
    /// that have been made by the agent. Clients are responsible for reverting these changes.
    pub num_turns: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadRollbackResponse {
    /// The updated thread after applying the rollback, with `turns` populated.
    ///
    /// The ThreadItems stored in each Turn are lossy since we explicitly do not
    /// persist all agent interactions, such as command executions. This is the same
    /// behavior as `thread/resume`.
    pub thread: Thread,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadListParams {
    /// Opaque pagination cursor returned by a previous call.
    #[ts(optional = nullable)]
    pub cursor: Option<String>,
    /// Optional page size; defaults to a reasonable server-side value.
    #[ts(optional = nullable)]
    pub limit: Option<u32>,
    /// Optional sort key; defaults to created_at.
    #[ts(optional = nullable)]
    pub sort_key: Option<ThreadSortKey>,
    /// Optional sort direction; defaults to descending (newest first).
    #[ts(optional = nullable)]
    pub sort_direction: Option<SortDirection>,
    /// Optional provider filter; when set, only sessions recorded under these
    /// providers are returned. When present but empty, includes all providers.
    #[ts(optional = nullable)]
    pub model_providers: Option<Vec<String>>,
    /// Optional source filter; when set, only sessions from these source kinds
    /// are returned. When omitted or empty, defaults to interactive sources.
    #[ts(optional = nullable)]
    pub source_kinds: Option<Vec<ThreadSourceKind>>,
    /// Optional archived filter; when set to true, only archived threads are returned.
    /// If false or null, only non-archived threads are returned.
    #[ts(optional = nullable)]
    pub archived: Option<bool>,
    /// Optional cwd filter or filters; when set, only threads whose session cwd
    /// exactly matches one of these paths are returned.
    #[ts(optional = nullable, type = "string | Array<string> | null")]
    pub cwd: Option<ThreadListCwdFilter>,
    /// If true, return from the state DB without scanning JSONL rollouts to
    /// repair thread metadata. Omitted or false preserves scan-and-repair
    /// behavior.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub use_state_db_only: bool,
    /// Optional substring filter for the extracted thread title.
    #[ts(optional = nullable)]
    pub search_term: Option<String>,
}

impl FromJson for ThreadListParams {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("ThreadListParams")?;
        Ok(Self {
            cursor: object.take_optional("cursor")?,
            limit: object.take_optional("limit")?,
            sort_key: object.take_optional("sortKey")?,
            sort_direction: object.take_optional("sortDirection")?,
            model_providers: object.take_optional("modelProviders")?,
            source_kinds: object.take_optional("sourceKinds")?,
            archived: object.take_optional("archived")?,
            cwd: object.take_optional("cwd")?,
            use_state_db_only: object.take_optional("useStateDbOnly")?.unwrap_or(false),
            search_term: object.take_optional("searchTerm")?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(untagged)]
pub enum ThreadListCwdFilter {
    One(String),
    Many(Vec<String>),
}

impl FromJson for ThreadListCwdFilter {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        if let Some(value) = value.as_str() {
            return Ok(Self::One(value.to_string()));
        }
        Ok(Self::Many(Vec::<String>::from_json(value)?))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum ThreadSourceKind {
    Cli,
    #[serde(rename = "vscode")]
    #[ts(rename = "vscode")]
    VsCode,
    Exec,
    AppServer,
    SubAgent,
    SubAgentReview,
    SubAgentCompact,
    SubAgentThreadSpawn,
    SubAgentOther,
    Unknown,
}

impl FromJson for ThreadSourceKind {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "cli" => Ok(Self::Cli),
            "vscode" => Ok(Self::VsCode),
            "exec" => Ok(Self::Exec),
            "appServer" => Ok(Self::AppServer),
            "subAgent" => Ok(Self::SubAgent),
            "subAgentReview" => Ok(Self::SubAgentReview),
            "subAgentCompact" => Ok(Self::SubAgentCompact),
            "subAgentThreadSpawn" => Ok(Self::SubAgentThreadSpawn),
            "subAgentOther" => Ok(Self::SubAgentOther),
            "unknown" => Ok(Self::Unknown),
            other => Err(JsonValueError::WrongType(format!(
                "unknown thread source kind `{other}`"
            ))),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum ThreadSortKey {
    CreatedAt,
    UpdatedAt,
}

impl FromJson for ThreadSortKey {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "created_at" => Ok(Self::CreatedAt),
            "updated_at" => Ok(Self::UpdatedAt),
            other => Err(JsonValueError::WrongType(format!(
                "unknown thread sort key `{other}`"
            ))),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub enum SortDirection {
    Asc,
    Desc,
}

impl ToJson for SortDirection {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        })
    }
}

impl FromJson for SortDirection {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "asc" => Ok(Self::Asc),
            "desc" => Ok(Self::Desc),
            other => Err(JsonValueError::WrongType(format!(
                "unknown sort direction `{other}`"
            ))),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadListResponse {
    pub data: Vec<Thread>,
    /// Opaque cursor to pass to the next call to continue after the last item.
    /// if None, there are no more items to return.
    pub next_cursor: Option<String>,
    /// Opaque cursor to pass as `cursor` when reversing `sortDirection`.
    /// This is only populated when the page contains at least one thread.
    /// Use it with the opposite `sortDirection`; for timestamp sorts it anchors
    /// at the start of the page timestamp so same-second updates are not skipped.
    pub backwards_cursor: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadLoadedListParams {
    /// Opaque pagination cursor returned by a previous call.
    #[ts(optional = nullable)]
    pub cursor: Option<String>,
    /// Optional page size; defaults to no limit.
    #[ts(optional = nullable)]
    pub limit: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadLoadedListResponse {
    /// Thread ids for sessions currently loaded in memory.
    pub data: Vec<String>,
    /// Opaque cursor to pass to the next call to continue after the last item.
    /// if None, there are no more items to return.
    pub next_cursor: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(tag = "type")]
#[ts(export_to = "v2/")]
pub enum ThreadStatus {
    NotLoaded,
    Idle,
    SystemError,
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Active {
        active_flags: Vec<ThreadActiveFlag>,
    },
}

impl ToJson for ThreadStatus {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(2);
        match self {
            ThreadStatus::NotLoaded => object.push_field("type", "notLoaded"),
            ThreadStatus::Idle => object.push_field("type", "idle"),
            ThreadStatus::SystemError => object.push_field("type", "systemError"),
            ThreadStatus::Active { active_flags } => {
                object.push_field("type", "active");
                object.push_field("activeFlags", active_flags.to_json());
            }
        }
        JsonValue::Object(object)
    }
}

impl FromJson for ThreadStatus {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("ThreadStatus")?;
        match String::from_json(object.take_required("type")?)?.as_str() {
            "notLoaded" => Ok(ThreadStatus::NotLoaded),
            "idle" => Ok(ThreadStatus::Idle),
            "systemError" => Ok(ThreadStatus::SystemError),
            "active" => Ok(ThreadStatus::Active {
                active_flags: object.take_optional("activeFlags")?.unwrap_or_default(),
            }),
            other => Err(JsonValueError::WrongType(format!(
                "unknown thread status `{other}`"
            ))),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum ThreadActiveFlag {
    WaitingOnApproval,
    WaitingOnUserInput,
}

impl ToJson for ThreadActiveFlag {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            ThreadActiveFlag::WaitingOnApproval => "waitingOnApproval",
            ThreadActiveFlag::WaitingOnUserInput => "waitingOnUserInput",
        })
    }
}

impl FromJson for ThreadActiveFlag {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "waitingOnApproval" => Ok(ThreadActiveFlag::WaitingOnApproval),
            "waitingOnUserInput" => Ok(ThreadActiveFlag::WaitingOnUserInput),
            other => Err(JsonValueError::WrongType(format!(
                "unknown thread active flag `{other}`"
            ))),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadReadParams {
    pub thread_id: String,
    /// When true, include turns and their items from rollout history.
    #[serde(default)]
    pub include_turns: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadReadResponse {
    pub thread: Thread,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadInjectItemsParams {
    pub thread_id: String,
    /// Raw Responses API items to append to the thread's model-visible history.
    pub items: Vec<JsonValue>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadInjectItemsResponse {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadTurnsListParams {
    pub thread_id: String,
    /// Opaque cursor to pass to the next call to continue after the last turn.
    #[ts(optional = nullable)]
    pub cursor: Option<String>,
    /// Optional turn page size.
    #[ts(optional = nullable)]
    pub limit: Option<u32>,
    /// Optional turn pagination direction; defaults to descending.
    #[ts(optional = nullable)]
    pub sort_direction: Option<SortDirection>,
    /// How much item detail to include for each returned turn; defaults to summary.
    #[ts(optional = nullable)]
    pub items_view: Option<TurnItemsView>,
}

impl FromJson for ThreadTurnsListParams {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("ThreadTurnsListParams")?;
        Ok(Self {
            thread_id: object.take_required("threadId")?,
            cursor: object.take_optional("cursor")?,
            limit: object.take_optional("limit")?,
            sort_direction: object.take_optional("sortDirection")?,
            items_view: object.take_optional("itemsView")?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadTurnsListResponse {
    pub data: Vec<Turn>,
    /// Opaque cursor to pass to the next call to continue after the last turn.
    /// if None, there are no more turns to return.
    pub next_cursor: Option<String>,
    /// Opaque cursor to pass as `cursor` when reversing `sortDirection`.
    /// This is only populated when the page contains at least one turn.
    /// Use it with the opposite `sortDirection` to include the anchor turn again
    /// and catch updates to that turn.
    pub backwards_cursor: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadTurnsItemsListParams {
    pub thread_id: String,
    pub turn_id: String,
    /// Opaque cursor to pass to the next call to continue after the last item.
    #[ts(optional = nullable)]
    pub cursor: Option<String>,
    /// Optional item page size.
    #[ts(optional = nullable)]
    pub limit: Option<u32>,
    /// Optional item pagination direction; defaults to ascending.
    #[ts(optional = nullable)]
    pub sort_direction: Option<SortDirection>,
}

impl ToJson for ThreadTurnsItemsListParams {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(5);
        object.push_field("threadId", self.thread_id.clone());
        object.push_field("turnId", self.turn_id.clone());
        if let Some(cursor) = &self.cursor {
            object.push_field("cursor", cursor.clone());
        }
        if let Some(limit) = self.limit {
            object.push_field("limit", limit as u64);
        }
        if let Some(sort_direction) = self.sort_direction {
            object.push_field("sortDirection", sort_direction.to_json());
        }
        JsonValue::Object(object)
    }
}

impl FromJson for ThreadTurnsItemsListParams {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("ThreadTurnsItemsListParams")?;
        Ok(Self {
            thread_id: object.take_required("threadId")?,
            turn_id: object.take_required("turnId")?,
            cursor: object.take_optional("cursor")?,
            limit: object.take_optional("limit")?,
            sort_direction: object.take_optional("sortDirection")?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadTurnsItemsListResponse {
    pub data: Vec<ThreadItem>,
    /// Opaque cursor to pass to the next call to continue after the last item.
    /// if None, there are no more items to return.
    pub next_cursor: Option<String>,
    /// Opaque cursor to pass as `cursor` when reversing `sortDirection`.
    /// This is only populated when the page contains at least one item.
    pub backwards_cursor: Option<String>,
}

impl ToJson for ThreadTurnsItemsListResponse {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(3);
        object.push_field("data", self.data.to_json());
        object.push_field("nextCursor", self.next_cursor.to_json());
        object.push_field("backwardsCursor", self.backwards_cursor.to_json());
        JsonValue::Object(object)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadTokenUsageUpdatedNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub token_usage: ThreadTokenUsage,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadTokenUsage {
    pub total: TokenUsageBreakdown,
    pub last: TokenUsageBreakdown,
    // TODO(aibrahim): make this not optional
    #[ts(type = "number | null")]
    pub model_context_window: Option<i64>,
}

impl From<CoreTokenUsageInfo> for ThreadTokenUsage {
    fn from(value: CoreTokenUsageInfo) -> Self {
        Self {
            total: value.total_token_usage.into(),
            last: value.last_token_usage.into(),
            model_context_window: value.model_context_window,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TokenUsageBreakdown {
    #[ts(type = "number")]
    pub total_tokens: i64,
    #[ts(type = "number")]
    pub input_tokens: i64,
    #[ts(type = "number")]
    pub cached_input_tokens: i64,
    #[ts(type = "number")]
    pub output_tokens: i64,
    #[ts(type = "number")]
    pub reasoning_output_tokens: i64,
}

impl From<CoreTokenUsage> for TokenUsageBreakdown {
    fn from(value: CoreTokenUsage) -> Self {
        Self {
            total_tokens: value.total_tokens,
            input_tokens: value.input_tokens,
            cached_input_tokens: value.cached_input_tokens,
            output_tokens: value.output_tokens,
            reasoning_output_tokens: value.reasoning_output_tokens,
        }
    }
}

// Thread/Turn lifecycle notifications and item progress events
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadStartedNotification {
    pub thread: Thread,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadStatusChangedNotification {
    pub thread_id: String,
    pub status: ThreadStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadArchivedNotification {
    pub thread_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadUnarchivedNotification {
    pub thread_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadClosedNotification {
    pub thread_id: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadNameUpdatedNotification {
    pub thread_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub thread_name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadGoalUpdatedNotification {
    pub thread_id: String,
    pub turn_id: Option<String>,
    pub goal: ThreadGoal,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadGoalClearedNotification {
    pub thread_id: String,
}

/// Deprecated: Use `ContextCompaction` item type instead.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ContextCompactedNotification {
    pub thread_id: String,
    pub turn_id: String,
}
