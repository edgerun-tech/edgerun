use super::ApprovalsReviewer;
use super::AskForApproval;
use super::PermissionProfileSelectionParams;
use super::SandboxPolicy;
use super::Turn;
use codex_protocol::compat::absolute_path::AbsolutePathBuf;
use codex_protocol::config_types::CollaborationMode;
use codex_protocol::config_types::Personality;
use codex_protocol::config_types::ReasoningSummary;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::plan_tool::PlanItemArg as CorePlanItemArg;
use codex_protocol::plan_tool::StepStatus as CorePlanStepStatus;
use codex_protocol::user_input::ByteRange as CoreByteRange;
use codex_protocol::user_input::TextElement as CoreTextElement;
use codex_protocol::user_input::UserInput as CoreUserInput;
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum TurnStatus {
    Completed,
    Interrupted,
    Failed,
    InProgress,
}

impl ToJson for TurnStatus {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            TurnStatus::Completed => "completed",
            TurnStatus::Interrupted => "interrupted",
            TurnStatus::Failed => "failed",
            TurnStatus::InProgress => "inProgress",
        })
    }
}

impl FromJson for TurnStatus {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "completed" => Ok(TurnStatus::Completed),
            "interrupted" => Ok(TurnStatus::Interrupted),
            "failed" => Ok(TurnStatus::Failed),
            "inProgress" => Ok(TurnStatus::InProgress),
            other => Err(JsonValueError::WrongType(format!(
                "unknown turn status `{other}`"
            ))),
        }
    }
}

// Turn APIs
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TurnEnvironmentParams {
    pub environment_id: String,
    pub cwd: AbsolutePathBuf,
}

impl ToJson for TurnEnvironmentParams {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(2);
        object.push_field("environmentId", self.environment_id.clone());
        object.push_field("cwd", self.cwd.to_json());
        JsonValue::Object(object)
    }
}

impl FromJson for TurnEnvironmentParams {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("TurnEnvironmentParams")?;
        Ok(Self {
            environment_id: object.take_required("environmentId")?,
            cwd: object.take_required("cwd")?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TurnStartParams {
    pub thread_id: String,
    pub input: Vec<UserInput>,
    /// Optional turn-scoped Responses API client metadata.
    #[ts(optional = nullable)]
    pub responsesapi_client_metadata: Option<HashMap<String, String>>,
    /// Optional turn-scoped environments.
    ///
    /// Omitted uses the thread sticky environments. Empty disables
    /// environment access for this turn. Non-empty selects the first
    /// environment as the current turn environment for this turn.
    #[ts(optional = nullable)]
    pub environments: Option<Vec<TurnEnvironmentParams>>,
    /// Override the working directory for this turn and subsequent turns.
    #[ts(optional = nullable)]
    pub cwd: Option<PathBuf>,
    /// Override the approval policy for this turn and subsequent turns.
    #[ts(optional = nullable)]
    pub approval_policy: Option<AskForApproval>,
    /// Override where approval requests are routed for review on this turn and
    /// subsequent turns.
    #[ts(optional = nullable)]
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    /// Override the sandbox policy for this turn and subsequent turns.
    #[ts(optional = nullable)]
    pub sandbox_policy: Option<SandboxPolicy>,
    /// Select a named permissions profile for this turn and subsequent turns.
    /// Cannot be combined with `sandboxPolicy`.
    #[ts(optional = nullable)]
    pub permissions: Option<PermissionProfileSelectionParams>,
    /// Override the model for this turn and subsequent turns.
    #[ts(optional = nullable)]
    pub model: Option<String>,
    /// Override the service tier for this turn and subsequent turns.
    #[serde(
        default,
        deserialize_with = "crate::protocol::serde_helpers::deserialize_double_option",
        serialize_with = "crate::protocol::serde_helpers::serialize_double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[ts(optional = nullable)]
    pub service_tier: Option<Option<String>>,
    /// Override the reasoning effort for this turn and subsequent turns.
    #[ts(optional = nullable)]
    pub effort: Option<ReasoningEffort>,
    /// Override the reasoning summary for this turn and subsequent turns.
    #[ts(optional = nullable)]
    pub summary: Option<ReasoningSummary>,
    /// Override the personality for this turn and subsequent turns.
    #[ts(optional = nullable)]
    pub personality: Option<Personality>,
    /// Optional JSON Schema used to constrain the final assistant message for
    /// this turn.
    #[ts(optional = nullable)]
    pub output_schema: Option<JsonValue>,

    /// EXPERIMENTAL - Set a pre-set collaboration mode.
    /// Takes precedence over model, reasoning_effort, and developer instructions if set.
    ///
    /// For `collaboration_mode.settings.developer_instructions`, `null` means
    /// "use the built-in instructions for the selected mode".
    #[ts(optional = nullable)]
    pub collaboration_mode: Option<CollaborationMode>,
}

impl ToJson for TurnStartParams {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(17);
        object.push_field("threadId", self.thread_id.clone());
        object.push_field("input", self.input.to_json());
        object.push_opt_field(
            "responsesapiClientMetadata",
            self.responsesapi_client_metadata
                .as_ref()
                .map(ToJson::to_json),
        );
        object.push_opt_field(
            "environments",
            self.environments.as_ref().map(ToJson::to_json),
        );
        object.push_opt_field(
            "cwd",
            self.cwd
                .as_ref()
                .map(|path| JsonValue::String(path.as_path().to_string_lossy().into_owned())),
        );
        object.push_opt_field(
            "approvalPolicy",
            self.approval_policy.map(|value| value.to_json()),
        );
        object.push_opt_field(
            "approvalsReviewer",
            self.approvals_reviewer.map(|value| value.to_json()),
        );
        object.push_opt_field(
            "sandboxPolicy",
            self.sandbox_policy.as_ref().map(ToJson::to_json),
        );
        object.push_opt_field(
            "permissions",
            self.permissions.as_ref().map(ToJson::to_json),
        );
        object.push_opt_field("model", self.model.clone());
        if let Some(service_tier) = &self.service_tier {
            object.push_field(
                "serviceTier",
                service_tier
                    .as_ref()
                    .map_or(JsonValue::Null, |tier| JsonValue::String(tier.clone())),
            );
        }
        object.push_opt_field("effort", self.effort.map(|value| value.to_json()));
        object.push_opt_field("summary", self.summary.map(|value| value.to_json()));
        object.push_opt_field("personality", self.personality.map(|value| value.to_json()));
        object.push_opt_field("outputSchema", self.output_schema.clone());
        object.push_opt_field(
            "collaborationMode",
            self.collaboration_mode
                .as_ref()
                .map(collaboration_mode_to_json),
        );
        JsonValue::Object(object)
    }
}

impl FromJson for TurnStartParams {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("TurnStartParams")?;
        let service_tier = match object.remove("serviceTier") {
            Some(JsonValue::Null) => Some(None),
            Some(value) => Some(Some(String::from_json(value)?)),
            None => None,
        };

        Ok(Self {
            thread_id: object.take_required("threadId")?,
            input: object.take_required("input")?,
            responsesapi_client_metadata: object.take_optional("responsesapiClientMetadata")?,
            environments: object.take_optional("environments")?,
            cwd: take_optional_path_buf(&mut object, "cwd")?,
            approval_policy: object.take_optional("approvalPolicy")?,
            approvals_reviewer: object.take_optional("approvalsReviewer")?,
            sandbox_policy: object.take_optional("sandboxPolicy")?,
            permissions: object.take_optional("permissions")?,
            model: object.take_optional("model")?,
            service_tier,
            effort: object.take_optional("effort")?,
            summary: object.take_optional("summary")?,
            personality: object.take_optional("personality")?,
            output_schema: object.take_optional("outputSchema")?,
            collaboration_mode: object
                .take_optional("collaborationMode")?
                .map(collaboration_mode_from_json)
                .transpose()?,
        })
    }
}

fn collaboration_mode_to_json(value: &CollaborationMode) -> JsonValue {
    let mut settings = Map::with_capacity(3);
    settings.push_field("model", value.settings.model.clone());
    if let Some(reasoning_effort) = value.settings.reasoning_effort {
        settings.push_field("reasoningEffort", reasoning_effort.to_json());
    }
    if let Some(developer_instructions) = &value.settings.developer_instructions {
        settings.push_field("developerInstructions", developer_instructions.clone());
    }

    let mut object = Map::with_capacity(2);
    object.push_field("mode", value.mode.to_json());
    object.push_field("settings", JsonValue::Object(settings));
    JsonValue::Object(object)
}

fn collaboration_mode_from_json(value: JsonValue) -> Result<CollaborationMode, JsonValueError> {
    let mut object = value.into_object("CollaborationMode")?;
    let mut settings = object
        .take_required::<JsonValue>("settings")?
        .into_object("CollaborationMode.settings")?;
    Ok(CollaborationMode {
        mode: object.take_required("mode")?,
        settings: codex_protocol::config_types::Settings {
            model: settings.take_required("model")?,
            reasoning_effort: settings.take_optional("reasoningEffort")?,
            developer_instructions: settings.take_optional("developerInstructions")?,
        },
    })
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TurnStartResponse {
    pub turn: Turn,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TurnSteerParams {
    pub thread_id: String,
    pub input: Vec<UserInput>,
    /// Optional turn-scoped Responses API client metadata.
    #[ts(optional = nullable)]
    pub responsesapi_client_metadata: Option<HashMap<String, String>>,
    /// Required active turn id precondition. The request fails when it does not
    /// match the currently active turn.
    pub expected_turn_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TurnSteerResponse {
    pub turn_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TurnInterruptParams {
    pub thread_id: String,
    pub turn_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TurnInterruptResponse {}

// User input types
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ByteRange {
    pub start: usize,
    pub end: usize,
}

impl ToJson for ByteRange {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(2);
        object.push_field("start", self.start as u64);
        object.push_field("end", self.end as u64);
        JsonValue::Object(object)
    }
}

impl FromJson for ByteRange {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("ByteRange")?;
        Ok(Self {
            start: object.take_required("start")?,
            end: object.take_required("end")?,
        })
    }
}

impl From<CoreByteRange> for ByteRange {
    fn from(value: CoreByteRange) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

impl From<ByteRange> for CoreByteRange {
    fn from(value: ByteRange) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TextElement {
    /// Byte range in the parent `text` buffer that this element occupies.
    pub byte_range: ByteRange,
    /// Optional human-readable placeholder for the element, displayed in the UI.
    placeholder: Option<String>,
}

impl ToJson for TextElement {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(2);
        object.push_field("byteRange", self.byte_range.to_json());
        object.push_opt_field("placeholder", self.placeholder.clone());
        JsonValue::Object(object)
    }
}

impl FromJson for TextElement {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("TextElement")?;
        Ok(Self {
            byte_range: object.take_required("byteRange")?,
            placeholder: object.take_optional("placeholder")?,
        })
    }
}

impl TextElement {
    pub fn new(byte_range: ByteRange, placeholder: Option<String>) -> Self {
        Self {
            byte_range,
            placeholder,
        }
    }

    pub fn set_placeholder(&mut self, placeholder: Option<String>) {
        self.placeholder = placeholder;
    }

    pub fn placeholder(&self) -> Option<&str> {
        self.placeholder.as_deref()
    }
}

impl From<CoreTextElement> for TextElement {
    fn from(value: CoreTextElement) -> Self {
        Self::new(
            value.byte_range.into(),
            value._placeholder_for_conversion_only().map(str::to_string),
        )
    }
}

impl From<TextElement> for CoreTextElement {
    fn from(value: TextElement) -> Self {
        Self::new(value.byte_range.into(), value.placeholder)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(tag = "type")]
#[ts(export_to = "v2/")]
pub enum UserInput {
    Text {
        text: String,
        /// UI-defined spans within `text` used to render or persist special elements.
        #[serde(default)]
        text_elements: Vec<TextElement>,
    },
    Image {
        url: String,
    },
    LocalImage {
        path: PathBuf,
    },
    Skill {
        name: String,
        path: PathBuf,
    },
    Mention {
        name: String,
        path: String,
    },
}

impl ToJson for UserInput {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(4);
        match self {
            UserInput::Text {
                text,
                text_elements,
            } => {
                object.push_field("type", "text");
                object.push_field("text", text.clone());
                object.push_field("textElements", text_elements.to_json());
            }
            UserInput::Image { url } => {
                object.push_field("type", "image");
                object.push_field("url", url.clone());
            }
            UserInput::LocalImage { path } => {
                object.push_field("type", "localImage");
                object.push_field("path", path.as_path().to_string_lossy().into_owned());
            }
            UserInput::Skill { name, path } => {
                object.push_field("type", "skill");
                object.push_field("name", name.clone());
                object.push_field("path", path.as_path().to_string_lossy().into_owned());
            }
            UserInput::Mention { name, path } => {
                object.push_field("type", "mention");
                object.push_field("name", name.clone());
                object.push_field("path", path.clone());
            }
        }
        JsonValue::Object(object)
    }
}

impl FromJson for UserInput {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("UserInput")?;
        match String::from_json(object.take_required("type")?)?.as_str() {
            "text" => Ok(UserInput::Text {
                text: object.take_required("text")?,
                text_elements: object.take_optional("textElements")?.unwrap_or_default(),
            }),
            "image" => Ok(UserInput::Image {
                url: object.take_required("url")?,
            }),
            "localImage" => Ok(UserInput::LocalImage {
                path: take_required_path_buf(&mut object, "path")?,
            }),
            "skill" => Ok(UserInput::Skill {
                name: object.take_required("name")?,
                path: take_required_path_buf(&mut object, "path")?,
            }),
            "mention" => Ok(UserInput::Mention {
                name: object.take_required("name")?,
                path: object.take_required("path")?,
            }),
            other => Err(JsonValueError::WrongType(format!(
                "unknown user input type `{other}`"
            ))),
        }
    }
}

fn take_optional_path_buf(object: &mut Map, key: &str) -> Result<Option<PathBuf>, JsonValueError> {
    match object.remove(key) {
        Some(JsonValue::Null) | None => Ok(None),
        Some(value) => take_path_buf_value(value).map(Some),
    }
}

fn take_required_path_buf(object: &mut Map, key: &str) -> Result<PathBuf, JsonValueError> {
    let value = object
        .remove(key)
        .ok_or_else(|| JsonValueError::WrongType(format!("missing required field `{key}`")))?;
    take_path_buf_value(value)
}

fn take_path_buf_value(value: JsonValue) -> Result<PathBuf, JsonValueError> {
    String::from_json(value).map(PathBuf::from)
}

impl UserInput {
    pub fn into_core(self) -> CoreUserInput {
        match self {
            UserInput::Text {
                text,
                text_elements,
            } => CoreUserInput::Text {
                text,
                text_elements: text_elements.into_iter().map(Into::into).collect(),
            },
            UserInput::Image { url } => CoreUserInput::Image { image_url: url },
            UserInput::LocalImage { path } => CoreUserInput::LocalImage { path },
            UserInput::Skill { name, path } => CoreUserInput::Skill { name, path },
            UserInput::Mention { name, path } => CoreUserInput::Mention { name, path },
        }
    }
}

impl From<CoreUserInput> for UserInput {
    fn from(value: CoreUserInput) -> Self {
        match value {
            CoreUserInput::Text {
                text,
                text_elements,
            } => UserInput::Text {
                text,
                text_elements: text_elements.into_iter().map(Into::into).collect(),
            },
            CoreUserInput::Image { image_url } => UserInput::Image { url: image_url },
            CoreUserInput::LocalImage { path } => UserInput::LocalImage { path },
            CoreUserInput::Skill { name, path } => UserInput::Skill { name, path },
            CoreUserInput::Mention { name, path } => UserInput::Mention { name, path },
            _ => unreachable!("unsupported user input variant"),
        }
    }
}

impl UserInput {
    pub fn text_char_count(&self) -> usize {
        match self {
            UserInput::Text { text, .. } => text.chars().count(),
            UserInput::Image { .. }
            | UserInput::LocalImage { .. }
            | UserInput::Skill { .. }
            | UserInput::Mention { .. } => 0,
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TurnStartedNotification {
    pub thread_id: String,
    pub turn: Turn,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct Usage {
    pub input_tokens: i32,
    pub cached_input_tokens: i32,
    pub output_tokens: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TurnCompletedNotification {
    pub thread_id: String,
    pub turn: Turn,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
/// Notification that the turn-level unified diff has changed.
/// Contains the latest aggregated diff across all file changes in the turn.
pub struct TurnDiffUpdatedNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub diff: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TurnPlanUpdatedNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub explanation: Option<String>,
    pub plan: Vec<TurnPlanStep>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TurnPlanStep {
    pub step: String,
    pub status: TurnPlanStepStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum TurnPlanStepStatus {
    Pending,
    InProgress,
    Completed,
}

impl From<CorePlanItemArg> for TurnPlanStep {
    fn from(value: CorePlanItemArg) -> Self {
        Self {
            step: value.step,
            status: value.status.into(),
        }
    }
}

impl From<CorePlanStepStatus> for TurnPlanStepStatus {
    fn from(value: CorePlanStepStatus) -> Self {
        match value {
            CorePlanStepStatus::Pending => Self::Pending,
            CorePlanStepStatus::InProgress => Self::InProgress,
            CorePlanStepStatus::Completed => Self::Completed,
        }
    }
}
