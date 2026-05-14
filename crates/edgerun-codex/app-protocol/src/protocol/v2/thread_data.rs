use super::CodexErrorInfo;
use super::ThreadItem;
use super::ThreadStatus;
use super::TurnStatus;
use codex_protocol::compat::absolute_path::AbsolutePathBuf;
use codex_protocol::protocol::SessionSource as CoreSessionSource;
use codex_protocol::protocol::SubAgentSource as CoreSubAgentSource;
use codex_protocol::protocol::ThreadSource as CoreThreadSource;
use edgerun_error::Error;
use edgerun_json::FromJson;
use edgerun_json::JsonValueError;
use edgerun_json::Map;
use edgerun_json::ToJson;
use edgerun_json::Value as JsonValue;
use schemars::JsonSchema;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub enum SessionSource {
    Cli,
    #[serde(rename = "vscode")]
    #[default]
    VsCode,
    Exec,
    AppServer,
    Custom(String),
    SubAgent(CoreSubAgentSource),
    #[serde(other)]
    Unknown,
}

impl From<CoreSessionSource> for SessionSource {
    fn from(value: CoreSessionSource) -> Self {
        match value {
            CoreSessionSource::Cli => SessionSource::Cli,
            CoreSessionSource::VSCode => SessionSource::VsCode,
            CoreSessionSource::Exec => SessionSource::Exec,
            CoreSessionSource::Mcp => SessionSource::AppServer,
            CoreSessionSource::Custom(source) => SessionSource::Custom(source),
            // We do not want to render those at the app-server level.
            CoreSessionSource::Internal(_) => SessionSource::Unknown,
            CoreSessionSource::SubAgent(sub) => SessionSource::SubAgent(sub),
            CoreSessionSource::Unknown => SessionSource::Unknown,
        }
    }
}

impl From<SessionSource> for CoreSessionSource {
    fn from(value: SessionSource) -> Self {
        match value {
            SessionSource::Cli => CoreSessionSource::Cli,
            SessionSource::VsCode => CoreSessionSource::VSCode,
            SessionSource::Exec => CoreSessionSource::Exec,
            SessionSource::AppServer => CoreSessionSource::Mcp,
            SessionSource::Custom(source) => CoreSessionSource::Custom(source),
            SessionSource::SubAgent(sub) => CoreSessionSource::SubAgent(sub),
            SessionSource::Unknown => CoreSessionSource::Unknown,
        }
    }
}

impl ToJson for SessionSource {
    fn to_json(&self) -> JsonValue {
        match self {
            SessionSource::Cli => JsonValue::from("cli"),
            SessionSource::VsCode => JsonValue::from("vscode"),
            SessionSource::Exec => JsonValue::from("exec"),
            SessionSource::AppServer => JsonValue::from("appServer"),
            SessionSource::Custom(source) => JsonValue::from(source.clone()),
            SessionSource::SubAgent(_) => JsonValue::from("subAgent"),
            SessionSource::Unknown => JsonValue::from("unknown"),
        }
    }
}

impl FromJson for SessionSource {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match value {
            JsonValue::String(source) => Ok(match source.as_str() {
                "cli" => SessionSource::Cli,
                "vscode" => SessionSource::VsCode,
                "exec" => SessionSource::Exec,
                "appServer" => SessionSource::AppServer,
                "unknown" => SessionSource::Unknown,
                _ => SessionSource::Custom(source),
            }),
            JsonValue::Object(_) => Err(JsonValueError::WrongType(
                "native SessionSource parsing expects a string source".to_string(),
            )),
            other => Err(JsonValueError::WrongType(format!(
                "expected session source string or object, found {}",
                other.variant_name()
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ThreadSource {
    User,
    Subagent,
    MemoryConsolidation,
}

impl From<CoreThreadSource> for ThreadSource {
    fn from(value: CoreThreadSource) -> Self {
        match value {
            CoreThreadSource::User => ThreadSource::User,
            CoreThreadSource::Subagent => ThreadSource::Subagent,
            CoreThreadSource::MemoryConsolidation => ThreadSource::MemoryConsolidation,
        }
    }
}

impl From<ThreadSource> for CoreThreadSource {
    fn from(value: ThreadSource) -> Self {
        match value {
            ThreadSource::User => CoreThreadSource::User,
            ThreadSource::Subagent => CoreThreadSource::Subagent,
            ThreadSource::MemoryConsolidation => CoreThreadSource::MemoryConsolidation,
        }
    }
}

impl ToJson for ThreadSource {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            ThreadSource::User => "user",
            ThreadSource::Subagent => "subagent",
            ThreadSource::MemoryConsolidation => "memory_consolidation",
        })
    }
}

impl FromJson for ThreadSource {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "user" => Ok(ThreadSource::User),
            "subagent" => Ok(ThreadSource::Subagent),
            "memory_consolidation" => Ok(ThreadSource::MemoryConsolidation),
            other => Err(JsonValueError::WrongType(format!(
                "unknown thread source `{other}`"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitInfo {
    pub sha: Option<String>,
    pub branch: Option<String>,
    pub origin_url: Option<String>,
}

impl ToJson for GitInfo {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(3);
        object.push_field("sha", self.sha.to_json());
        object.push_field("branch", self.branch.to_json());
        object.push_field("originUrl", self.origin_url.to_json());
        JsonValue::Object(object)
    }
}

impl FromJson for GitInfo {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("GitInfo")?;
        Ok(Self {
            sha: object.take_optional("sha")?,
            branch: object.take_optional("branch")?,
            origin_url: object.take_optional("originUrl")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    pub id: String,
    /// Session id shared by threads that belong to the same session tree.
    pub session_id: String,
    /// Source thread id when this thread was created by forking another thread.
    pub forked_from_id: Option<String>,
    /// Usually the first user message in the thread, if available.
    pub preview: String,
    /// Whether the thread is ephemeral and should not be materialized on disk.
    pub ephemeral: bool,
    /// Model provider used for this thread (for example, 'openai').
    pub model_provider: String,
    /// Unix timestamp (in seconds) when the thread was created.
    pub created_at: i64,
    /// Unix timestamp (in seconds) when the thread was last updated.
    pub updated_at: i64,
    /// Current runtime status for the thread.
    pub status: ThreadStatus,
    /// [UNSTABLE] Path to the thread on disk.
    pub path: Option<PathBuf>,
    /// Working directory captured for the thread.
    pub cwd: AbsolutePathBuf,
    /// Version of the CLI that created the thread.
    pub cli_version: String,
    /// Origin of the thread (CLI, VSCode, codex exec, codex app-server, etc.).
    pub source: SessionSource,
    /// Optional analytics source classification for this thread.
    pub thread_source: Option<ThreadSource>,
    /// Optional random unique nickname assigned to an AgentControl-spawned sub-agent.
    pub agent_nickname: Option<String>,
    /// Optional role (agent_role) assigned to an AgentControl-spawned sub-agent.
    pub agent_role: Option<String>,
    /// Optional Git metadata captured when the thread was created.
    pub git_info: Option<GitInfo>,
    /// Optional user-facing thread title.
    pub name: Option<String>,
    /// Only populated on `thread/resume`, `thread/rollback`, `thread/fork`, and `thread/read`
    /// (when `includeTurns` is true) responses.
    /// For all other responses and notifications returning a Thread,
    /// the turns field will be an empty list.
    pub turns: Vec<Turn>,
}

impl FromJson for Thread {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("Thread")?;
        Ok(Self {
            id: object.take_required("id")?,
            session_id: object.take_required("sessionId")?,
            forked_from_id: object.take_optional("forkedFromId")?,
            preview: object.take_required("preview")?,
            ephemeral: object.take_required("ephemeral")?,
            model_provider: object.take_required("modelProvider")?,
            created_at: object.take_required("createdAt")?,
            updated_at: object.take_required("updatedAt")?,
            status: object.take_required("status")?,
            path: take_optional_path_buf(&mut object, "path")?,
            cwd: object.take_required("cwd")?,
            cli_version: object.take_required("cliVersion")?,
            source: object.take_required("source")?,
            thread_source: object.take_optional("threadSource")?,
            agent_nickname: object.take_optional("agentNickname")?,
            agent_role: object.take_optional("agentRole")?,
            git_info: object.take_optional("gitInfo")?,
            name: object.take_optional("name")?,
            turns: object.take_optional("turns")?.unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson)]
#[serde(rename_all = "camelCase")]
pub struct Turn {
    pub id: String,
    /// Thread items currently included in this turn payload.
    pub items: Vec<ThreadItem>,
    /// Describes how much of `items` has been loaded for this turn.
    #[serde(default)]
    pub items_view: TurnItemsView,
    pub status: TurnStatus,
    /// Only populated when the Turn's status is failed.
    pub error: Option<TurnError>,
    /// Unix timestamp (in seconds) when the turn started.
    pub started_at: Option<i64>,
    /// Unix timestamp (in seconds) when the turn completed.
    pub completed_at: Option<i64>,
    /// Duration between turn start and completion in milliseconds, if known.
    pub duration_ms: Option<i64>,
}

impl FromJson for Turn {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("Turn")?;
        Ok(Self {
            id: object.take_required("id")?,
            items: thread_items_from_json(object.remove("items"))?,
            items_view: object.take_optional("itemsView")?.unwrap_or_default(),
            status: object.take_required("status")?,
            error: object.take_optional("error")?,
            started_at: object.take_optional("startedAt")?,
            completed_at: object.take_optional("completedAt")?,
            duration_ms: object.take_optional("durationMs")?,
        })
    }
}

fn thread_items_from_json(value: Option<JsonValue>) -> Result<Vec<ThreadItem>, JsonValueError> {
    match value {
        None | Some(JsonValue::Null) => Ok(Vec::new()),
        Some(JsonValue::Array(items)) if items.is_empty() => Ok(Vec::new()),
        Some(JsonValue::Array(_)) => Err(JsonValueError::WrongType(
            "native ThreadItem parsing is not implemented for non-empty turn items".to_string(),
        )),
        Some(other) => Err(JsonValueError::WrongType(format!(
            "expected turn items array, found {}",
            other.variant_name()
        ))),
    }
}

fn take_optional_path_buf(object: &mut Map, key: &str) -> Result<Option<PathBuf>, JsonValueError> {
    match object.remove(key) {
        Some(JsonValue::Null) | None => Ok(None),
        Some(value) => String::from_json(value).map(PathBuf::from).map(Some),
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TurnItemsView {
    /// `items` was not loaded for this turn. The field is intentionally empty.
    NotLoaded,
    /// `items` contains only a display summary for this turn.
    Summary,
    /// `items` contains every ThreadItem available from persisted app-server history for this turn.
    #[default]
    Full,
}

impl ToJson for TurnItemsView {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            Self::NotLoaded => "notLoaded",
            Self::Summary => "summary",
            Self::Full => "full",
        })
    }
}

impl FromJson for TurnItemsView {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "notLoaded" => Ok(Self::NotLoaded),
            "summary" => Ok(Self::Summary),
            "full" => Ok(Self::Full),
            other => Err(JsonValueError::WrongType(format!(
                "unknown turn items view `{other}`"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, Error, edgerun_json::ToJson)]
#[serde(rename_all = "camelCase")]
#[error("{message}")]
pub struct TurnError {
    pub message: String,
    pub codex_error_info: Option<CodexErrorInfo>,
    #[serde(default)]
    pub additional_details: Option<String>,
}

impl FromJson for TurnError {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("TurnError")?;
        let codex_error_info = match object.remove("codexErrorInfo") {
            Some(JsonValue::Null) | None => None,
            Some(_) => {
                return Err(JsonValueError::WrongType(
                    "native TurnError parsing does not support codexErrorInfo yet".to_string(),
                ));
            }
        };

        Ok(Self {
            message: object.take_required("message")?,
            codex_error_info,
            additional_details: object.take_optional("additionalDetails")?,
        })
    }
}
