//! Stubbed `codex-core` facade for the lifted workspace.
//!
//! The original `codex-core` crate is the product integration layer. It wires
//! config loading, auth persistence, telemetry, rollout storage, plugins,
//! sandboxing, MCP, hooks, state DBs, and terminal behavior into the agent
//! loop. Those are deliberately outside this lift.
//!
//! This facade keeps the crate name available while exposing the useful lower
//! layers that have been separated and made to compile without the product
//! platform.

pub use codex_api as api;
pub use codex_api::ResponseEvent;
pub use codex_api::ResponseStream;
pub use codex_apply_patch as apply_patch;
pub use codex_model_provider as model_provider;
pub use codex_model_provider_info as model_provider_info;
pub use codex_models_manager as models_manager;
pub use codex_protocol as protocol;
pub use codex_shell_command as shell_command;
pub use codex_tools as tools;

pub use codex_protocol::config_types::ModelProviderAuthInfo;
pub use codex_protocol::config_types::Personality;
pub use codex_protocol::models::BaseInstructions;
pub use codex_protocol::models::ResponseItem;
pub use codex_tools::ToolSpec;
pub use edgerun_json::JsonValue;

/// Review thread system prompt placeholder.
pub const REVIEW_PROMPT: &str = "";

/// Header names retained for callers that used the old core constants.
pub const X_CODEX_INSTALLATION_ID_HEADER: &str = "x-codex-installation-id";
pub const X_CODEX_TURN_METADATA_HEADER: &str = "x-codex-turn-metadata";
pub const X_RESPONSESAPI_INCLUDE_TIMING_METRICS_HEADER: &str =
    "x-responsesapi-include-timing-metrics";

/// API request payload for a single model turn.
#[derive(Debug, Clone)]
pub struct Prompt {
    pub input: Vec<ResponseItem>,
    pub tools: Vec<ToolSpec>,
    pub parallel_tool_calls: bool,
    pub base_instructions: BaseInstructions,
    pub personality: Option<Personality>,
    pub output_schema: Option<JsonValue>,
    pub output_schema_strict: bool,
}

impl Default for Prompt {
    fn default() -> Self {
        Self {
            input: Vec::new(),
            tools: Vec::new(),
            parallel_tool_calls: false,
            base_instructions: BaseInstructions::default(),
            personality: None,
            output_schema: None,
            output_schema_strict: true,
        }
    }
}

/// Placeholder for the full session-scoped model client.
///
/// The original implementation still depends on auth refresh, telemetry,
/// rollout tracing, and product transport policy. Build a new client on top of
/// `codex-api` and `codex-model-provider` instead of reviving those deps.
#[derive(Debug, Clone, Copy, Default)]
pub struct ModelClient;

/// Placeholder for the old per-turn model client session.
#[derive(Debug, Clone, Copy, Default)]
pub struct ModelClientSession;

/// Placeholder for the old thread manager.
#[derive(Debug, Clone, Copy, Default)]
pub struct ThreadManager;

#[deprecated(note = "use ThreadManager")]
pub type ConversationManager = ThreadManager;

/// Placeholder for a new thread request.
#[derive(Debug, Clone, Copy, Default)]
pub struct NewThread;

#[deprecated(note = "use NewThread")]
pub type NewConversation = NewThread;

/// Placeholder for a running Codex thread.
#[derive(Debug, Clone, Copy, Default)]
pub struct CodexThread;

#[deprecated(note = "use CodexThread")]
pub type CodexConversation = CodexThread;

/// Stub error retained for callers that reference steering APIs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteerInputError;

impl std::fmt::Display for SteerInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("steer input is unavailable in the lifted core stub")
    }
}

impl std::error::Error for SteerInputError {}

/// Stub event parser. The real mapping code can be lifted later if needed.
pub fn parse_turn_item(_item: &ResponseItem) -> Option<codex_protocol::items::TurnItem> {
    None
}

/// Minimal text extraction helper retained from the old public surface.
pub fn content_items_to_text(items: &[codex_protocol::models::ContentItem]) -> String {
    items
        .iter()
        .filter_map(|item| match item {
            codex_protocol::models::ContentItem::InputText { text }
            | codex_protocol::models::ContentItem::OutputText { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Stub model manager builder. Call `codex_model_provider::create_model_provider`
/// directly in the lifted client.
pub fn build_models_manager() {}
