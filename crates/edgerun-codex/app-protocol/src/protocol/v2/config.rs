use super::ApprovalsReviewer;
use super::shared::default_enabled;
use codex_protocol::compat::absolute_path::AbsolutePathBuf;
use codex_protocol::config_types::ForcedLoginMethod;
use codex_protocol::config_types::ReasoningSummary;
use codex_protocol::config_types::Verbosity;
use codex_protocol::config_types::WebSearchMode;
use codex_protocol::config_types::WebSearchToolConfig;
use codex_protocol::openai_models::ReasoningEffort;
use edgerun_json::FromJson;
use edgerun_json::JsonValueError;
use edgerun_json::Map;
use edgerun_json::ToJson;
use edgerun_json::Value as JsonValue;
use schemars::JsonSchema;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(tag = "type", rename_all = "camelCase")]
pub enum ConfigLayerSource {
    /// Managed preferences layer delivered by MDM (macOS only).
    #[schemars(rename_all = "camelCase")]
    Mdm {
        domain: String,
        key: String,
    },

    /// Managed config layer from a file (usually `managed_config.toml`).
    #[schemars(rename_all = "camelCase")]
    System {
        /// This is the path to the system config.toml file, though it is not
        /// guaranteed to exist.
        file: AbsolutePathBuf,
    },

    /// User config layer from $CODEX_HOME/config.toml. This layer is special
    /// in that it is expected to be:
    /// - writable by the user
    /// - generally outside the workspace directory
    #[schemars(rename_all = "camelCase")]
    User {
        /// This is the path to the user's config.toml file, though it is not
        /// guaranteed to exist.
        file: AbsolutePathBuf,
    },

    /// Path to a .codex/ folder within a project. There could be multiple of
    /// these between `cwd` and the project/repo root.
    #[schemars(rename_all = "camelCase")]
    Project {
        dot_codex_folder: AbsolutePathBuf,
    },

    /// Session-layer overrides supplied via `-c`/`--config`.
    SessionFlags,

    /// `managed_config.toml` was designed to be a config that was loaded
    /// as the last layer on top of everything else. This scheme did not quite
    /// work out as intended, but we keep this variant as a "best effort" while
    /// we phase out `managed_config.toml` in favor of `requirements.toml`.
    #[schemars(rename_all = "camelCase")]
    LegacyManagedConfigTomlFromFile {
        file: AbsolutePathBuf,
    },

    LegacyManagedConfigTomlFromMdm,
}

impl ConfigLayerSource {
    /// A settings from a layer with a higher precedence will override a setting
    /// from a layer with a lower precedence.
    pub fn precedence(&self) -> i16 {
        match self {
            ConfigLayerSource::Mdm { .. } => 0,
            ConfigLayerSource::System { .. } => 10,
            ConfigLayerSource::User { .. } => 20,
            ConfigLayerSource::Project { .. } => 25,
            ConfigLayerSource::SessionFlags => 30,
            ConfigLayerSource::LegacyManagedConfigTomlFromFile { .. } => 40,
            ConfigLayerSource::LegacyManagedConfigTomlFromMdm => 50,
        }
    }
}

/// Compares [ConfigLayerSource] by precedence, so `A < B` means settings from
/// layer `A` will be overridden by settings from layer `B`.
impl PartialOrd for ConfigLayerSource {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.precedence().cmp(&other.precedence()))
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "snake_case")]
pub struct ToolsV2 {
    pub web_search: Option<WebSearchToolConfig>,
    pub view_image: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "snake_case")]
pub struct ProfileV2 {
    pub model: Option<String>,
    pub model_provider: Option<String>,
    /// [UNSTABLE] Optional profile-level override for where approval requests
    /// are routed for review. If omitted, the enclosing config default is
    /// used.
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    pub service_tier: Option<String>,
    pub model_reasoning_effort: Option<ReasoningEffort>,
    pub model_reasoning_summary: Option<ReasoningSummary>,
    pub model_verbosity: Option<Verbosity>,
    pub web_search: Option<WebSearchMode>,
    pub tools: Option<ToolsV2>,
    pub chatgpt_base_url: Option<String>,
    #[schemars(default, flatten)]
    pub additional: HashMap<String, JsonValue>,
}

impl crate::experimental_api::ExperimentalApi for ProfileV2 {
    fn experimental_reason(&self) -> Option<&'static str> {
        self.approvals_reviewer
            .is_some()
            .then_some("config/read.approvalsReviewer")
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "snake_case")]
pub struct AnalyticsConfig {
    pub enabled: Option<bool>,
    #[schemars(default, flatten)]
    pub additional: HashMap<String, JsonValue>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson,
)]
#[schemars(rename_all = "snake_case")]
pub enum AppToolApproval {
    Auto,
    Prompt,
    Approve,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "snake_case")]
pub struct AppsDefaultConfig {
    #[schemars(default = "default_enabled")]
    pub enabled: bool,
    #[schemars(default = "default_enabled")]
    pub destructive_enabled: bool,
    #[schemars(default = "default_enabled")]
    pub open_world_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "snake_case")]
pub struct AppToolConfig {
    pub enabled: Option<bool>,
    pub approval_mode: Option<AppToolApproval>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "snake_case")]
pub struct AppToolsConfig {
    #[schemars(default, flatten)]
    pub tools: HashMap<String, AppToolConfig>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "snake_case")]
pub struct AppConfig {
    #[schemars(default = "default_enabled")]
    pub enabled: bool,
    pub destructive_enabled: Option<bool>,
    pub open_world_enabled: Option<bool>,
    pub default_tools_approval_mode: Option<AppToolApproval>,
    pub default_tools_enabled: Option<bool>,
    pub tools: Option<AppToolsConfig>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "snake_case")]
pub struct AppsConfig {
    #[schemars(default, rename = "_default")]
    pub default: Option<AppsDefaultConfig>,
    #[schemars(default, flatten)]
    pub apps: HashMap<String, AppConfig>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "snake_case")]
pub struct Config {
    pub model: Option<String>,
    pub review_model: Option<String>,
    pub model_context_window: Option<i64>,
    pub model_auto_compact_token_limit: Option<i64>,
    pub model_provider: Option<String>,
    /// [UNSTABLE] Optional default for where approval requests are routed for
    /// review.
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    pub forced_chatgpt_workspace_id: Option<String>,
    pub forced_login_method: Option<ForcedLoginMethod>,
    pub web_search: Option<WebSearchMode>,
    pub tools: Option<ToolsV2>,
    pub profile: Option<String>,
    #[schemars(default)]
    pub profiles: HashMap<String, ProfileV2>,
    pub instructions: Option<String>,
    pub developer_instructions: Option<String>,
    pub compact_prompt: Option<String>,
    pub model_reasoning_effort: Option<ReasoningEffort>,
    pub model_reasoning_summary: Option<ReasoningSummary>,
    pub model_verbosity: Option<Verbosity>,
    pub service_tier: Option<String>,
    pub analytics: Option<AnalyticsConfig>,
    #[schemars(default)]
    pub apps: Option<AppsConfig>,
    #[schemars(default, flatten)]
    pub additional: HashMap<String, JsonValue>,
}

impl crate::experimental_api::ExperimentalApi for Config {
    fn experimental_reason(&self) -> Option<&'static str> {
        if self.approvals_reviewer.is_some() {
            return Some("config/read.approvalsReviewer");
        }
        self.profiles
            .values()
            .find_map(crate::experimental_api::ExperimentalApi::experimental_reason)
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct ConfigLayerMetadata {
    pub name: ConfigLayerSource,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct ConfigLayer {
    pub name: ConfigLayerSource,
    pub version: String,
    pub config: JsonValue,
    #[schemars(skip_serializing_if = "Option::is_none")]
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub enum MergeStrategy {
    Replace,
    Upsert,
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub enum WriteStatus {
    Ok,
    OkOverridden,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct OverriddenMetadata {
    pub message: String,
    pub overriding_layer: ConfigLayerMetadata,
    pub effective_value: JsonValue,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct ConfigWriteResponse {
    pub status: WriteStatus,
    pub version: String,
    /// Canonical path to the config file that was written.
    pub file_path: AbsolutePathBuf,
    pub overridden_metadata: Option<OverriddenMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub enum ConfigWriteErrorCode {
    ConfigLayerReadonly,
    ConfigVersionConflict,
    ConfigValidationError,
    ConfigPathNotFound,
    ConfigSchemaUnknownKey,
    UserLayerNotFound,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct ConfigReadParams {
    #[schemars(default)]
    pub include_layers: bool,
    /// Optional working directory to resolve project config layers. If specified,
    /// return the effective config as seen from that directory (i.e., including any
    /// project layers between `cwd` and the project/repo root).
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct ConfigReadResponse {
    pub config: Config,
    pub origins: HashMap<String, ConfigLayerMetadata>,
    #[schemars(skip_serializing_if = "Option::is_none")]
    pub layers: Option<Vec<ConfigLayer>>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct ConfigRequirements {
    pub allowed_approvals_reviewers: Option<Vec<ApprovalsReviewer>>,
    pub allowed_web_search_modes: Option<Vec<WebSearchMode>>,
    pub feature_requirements: Option<BTreeMap<String, bool>>,
    pub hooks: Option<ManagedHooksRequirements>,
    pub enforce_residency: Option<ResidencyRequirement>,
    pub network: Option<NetworkRequirements>,
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct ManagedHooksRequirements {
    pub managed_dir: Option<PathBuf>,
    #[schemars(rename = "PreToolUse")]
    pub pre_tool_use: Vec<ConfiguredHookMatcherGroup>,
    #[schemars(rename = "PostToolUse")]
    pub post_tool_use: Vec<ConfiguredHookMatcherGroup>,
    #[schemars(rename = "PreCompact")]
    pub pre_compact: Vec<ConfiguredHookMatcherGroup>,
    #[schemars(rename = "PostCompact")]
    pub post_compact: Vec<ConfiguredHookMatcherGroup>,
    #[schemars(rename = "SessionStart")]
    pub session_start: Vec<ConfiguredHookMatcherGroup>,
    #[schemars(rename = "UserPromptSubmit")]
    pub user_prompt_submit: Vec<ConfiguredHookMatcherGroup>,
    #[schemars(rename = "Stop")]
    pub stop: Vec<ConfiguredHookMatcherGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct ConfiguredHookMatcherGroup {
    pub matcher: Option<String>,
    pub hooks: Vec<ConfiguredHookHandler>,
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(tag = "type")]
pub enum ConfiguredHookHandler {
    #[schemars(rename = "command")]
    Command {
        command: String,
        #[schemars(rename = "timeoutSec")]
        timeout_sec: Option<u64>,
        r#async: bool,
        #[schemars(rename = "statusMessage")]
        status_message: Option<String>,
    },
    #[schemars(rename = "prompt")]
    Prompt {},
    #[schemars(rename = "agent")]
    Agent {},
}

#[derive(Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename_all = "camelCase")]
pub struct NetworkRequirements {
    pub enabled: Option<bool>,
    pub http_port: Option<u16>,
    pub socks_port: Option<u16>,
    pub allow_upstream_proxy: Option<bool>,
    pub dangerously_allow_non_loopback_proxy: Option<bool>,
    pub dangerously_allow_all_unix_sockets: Option<bool>,
    /// Canonical network permission map for `experimental_network`.
    pub domains: Option<BTreeMap<String, NetworkDomainPermission>>,
    /// When true, only managed allowlist entries are respected while managed
    /// network enforcement is active.
    pub managed_allowed_domains_only: Option<bool>,
    /// Legacy compatibility view derived from `domains`.
    pub allowed_domains: Option<Vec<String>>,
    /// Legacy compatibility view derived from `domains`.
    pub denied_domains: Option<Vec<String>>,
    /// Canonical unix socket permission map for `experimental_network`.
    pub unix_sockets: Option<BTreeMap<String, NetworkUnixSocketPermission>>,
    /// Legacy compatibility view derived from `unix_sockets`.
    pub allow_unix_sockets: Option<Vec<String>>,
    pub allow_local_binding: Option<bool>,
}

impl ToJson for NetworkRequirements {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(14);
        object.push_field("enabled", self.enabled.to_json());
        object.push_field("httpPort", self.http_port.to_json());
        object.push_field("socksPort", self.socks_port.to_json());
        object.push_field("allowUpstreamProxy", self.allow_upstream_proxy.to_json());
        object.push_field(
            "dangerouslyAllowNonLoopbackProxy",
            self.dangerously_allow_non_loopback_proxy.to_json(),
        );
        object.push_field(
            "dangerouslyAllowAllUnixSockets",
            self.dangerously_allow_all_unix_sockets.to_json(),
        );
        object.push_field("domains", self.domains.to_json());
        object.push_field(
            "managedAllowedDomainsOnly",
            self.managed_allowed_domains_only.to_json(),
        );
        object.push_field("allowedDomains", self.allowed_domains.to_json());
        object.push_field("deniedDomains", self.denied_domains.to_json());
        object.push_field("unixSockets", self.unix_sockets.to_json());
        object.push_field("allowUnixSockets", self.allow_unix_sockets.to_json());
        object.push_field("allowLocalBinding", self.allow_local_binding.to_json());
        JsonValue::Object(object)
    }
}

impl FromJson for NetworkRequirements {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("NetworkRequirements")?;
        Ok(Self {
            enabled: object.take_optional("enabled")?,
            http_port: object.take_optional("httpPort")?,
            socks_port: object.take_optional("socksPort")?,
            allow_upstream_proxy: object.take_optional("allowUpstreamProxy")?,
            dangerously_allow_non_loopback_proxy: object
                .take_optional("dangerouslyAllowNonLoopbackProxy")?,
            dangerously_allow_all_unix_sockets: object
                .take_optional("dangerouslyAllowAllUnixSockets")?,
            domains: object.take_optional("domains")?,
            managed_allowed_domains_only: object.take_optional("managedAllowedDomainsOnly")?,
            allowed_domains: object.take_optional("allowedDomains")?,
            denied_domains: object.take_optional("deniedDomains")?,
            unix_sockets: object.take_optional("unixSockets")?,
            allow_unix_sockets: object.take_optional("allowUnixSockets")?,
            allow_local_binding: object.take_optional("allowLocalBinding")?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
#[schemars(rename_all = "lowercase")]
pub enum NetworkDomainPermission {
    Allow,
    Deny,
}

impl ToJson for NetworkDomainPermission {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
        })
    }
}

impl FromJson for NetworkDomainPermission {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "allow" => Ok(Self::Allow),
            "deny" => Ok(Self::Deny),
            other => Err(JsonValueError::WrongType(format!(
                "unknown network domain permission `{other}`"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
#[schemars(rename_all = "lowercase")]
pub enum NetworkUnixSocketPermission {
    Allow,
    None,
}

impl ToJson for NetworkUnixSocketPermission {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            Self::Allow => "allow",
            Self::None => "none",
        })
    }
}

impl FromJson for NetworkUnixSocketPermission {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "allow" => Ok(Self::Allow),
            "none" => Ok(Self::None),
            other => Err(JsonValueError::WrongType(format!(
                "unknown network unix socket permission `{other}`"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub enum ResidencyRequirement {
    Us,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct ConfigRequirementsReadResponse {
    /// Null if no requirements are configured (e.g. no requirements.toml/MDM entries).
    pub requirements: Option<ConfigRequirements>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, edgerun_json::ToJson)]
pub enum ExternalAgentConfigMigrationItemType {
    #[schemars(rename = "AGENTS_MD")]
    AgentsMd,
    #[schemars(rename = "CONFIG")]
    Config,
    #[schemars(rename = "SKILLS")]
    Skills,
    #[schemars(rename = "PLUGINS")]
    Plugins,
    #[schemars(rename = "MCP_SERVER_CONFIG")]
    McpServerConfig,
    #[schemars(rename = "SUBAGENTS")]
    Subagents,
    #[schemars(rename = "HOOKS")]
    Hooks,
    #[schemars(rename = "COMMANDS")]
    Commands,
    #[schemars(rename = "SESSIONS")]
    Sessions,
}

impl FromJson for ExternalAgentConfigMigrationItemType {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "AGENTS_MD" => Ok(Self::AgentsMd),
            "CONFIG" => Ok(Self::Config),
            "SKILLS" => Ok(Self::Skills),
            "PLUGINS" => Ok(Self::Plugins),
            "MCP_SERVER_CONFIG" => Ok(Self::McpServerConfig),
            "SUBAGENTS" => Ok(Self::Subagents),
            "HOOKS" => Ok(Self::Hooks),
            "COMMANDS" => Ok(Self::Commands),
            "SESSIONS" => Ok(Self::Sessions),
            other => Err(JsonValueError::WrongType(format!(
                "unknown external agent migration item type `{other}`"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson)]
#[schemars(rename_all = "camelCase")]
pub struct PluginsMigration {
    #[schemars(rename = "marketplaceName")]
    pub marketplace_name: String,
    #[schemars(rename = "pluginNames")]
    pub plugin_names: Vec<String>,
}

impl FromJson for PluginsMigration {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("PluginsMigration")?;
        Ok(Self {
            marketplace_name: object.take_required("marketplaceName")?,
            plugin_names: object.take_required("pluginNames")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson)]
#[schemars(rename_all = "camelCase")]
pub struct SessionMigration {
    pub path: PathBuf,
    pub cwd: PathBuf,
    pub title: Option<String>,
}

impl FromJson for SessionMigration {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("SessionMigration")?;
        Ok(Self {
            path: json_path_buf(object.take_required("path")?),
            cwd: json_path_buf(object.take_required("cwd")?),
            title: object.take_optional("title")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson)]
#[schemars(rename_all = "camelCase")]
pub struct McpServerMigration {
    pub name: String,
}

impl FromJson for McpServerMigration {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("McpServerMigration")?;
        Ok(Self {
            name: object.take_required("name")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson)]
#[schemars(rename_all = "camelCase")]
pub struct HookMigration {
    pub name: String,
}

impl FromJson for HookMigration {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("HookMigration")?;
        Ok(Self {
            name: object.take_required("name")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson)]
#[schemars(rename_all = "camelCase")]
pub struct SubagentMigration {
    pub name: String,
}

impl FromJson for SubagentMigration {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("SubagentMigration")?;
        Ok(Self {
            name: object.take_required("name")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson)]
#[schemars(rename_all = "camelCase")]
pub struct CommandMigration {
    pub name: String,
}

impl FromJson for CommandMigration {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("CommandMigration")?;
        Ok(Self {
            name: object.take_required("name")?,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, JsonSchema, edgerun_json::ToJson)]
#[schemars(rename_all = "camelCase")]
pub struct MigrationDetails {
    #[schemars(default)]
    pub plugins: Vec<PluginsMigration>,
    #[schemars(default)]
    pub sessions: Vec<SessionMigration>,
    #[schemars(default)]
    pub mcp_servers: Vec<McpServerMigration>,
    #[schemars(default)]
    pub hooks: Vec<HookMigration>,
    #[schemars(default)]
    pub subagents: Vec<SubagentMigration>,
    #[schemars(default)]
    pub commands: Vec<CommandMigration>,
}

impl FromJson for MigrationDetails {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("MigrationDetails")?;
        Ok(Self {
            plugins: object.take_optional("plugins")?.unwrap_or_default(),
            sessions: object.take_optional("sessions")?.unwrap_or_default(),
            mcp_servers: object.take_optional("mcpServers")?.unwrap_or_default(),
            hooks: object.take_optional("hooks")?.unwrap_or_default(),
            subagents: object.take_optional("subagents")?.unwrap_or_default(),
            commands: object.take_optional("commands")?.unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson)]
#[schemars(rename_all = "camelCase")]
pub struct ExternalAgentConfigMigrationItem {
    pub item_type: ExternalAgentConfigMigrationItemType,
    pub description: String,
    /// Null or empty means home-scoped migration; non-empty means repo-scoped migration.
    pub cwd: Option<PathBuf>,
    pub details: Option<MigrationDetails>,
}

impl FromJson for ExternalAgentConfigMigrationItem {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("ExternalAgentConfigMigrationItem")?;
        Ok(Self {
            item_type: object.take_required("itemType")?,
            description: object.take_required("description")?,
            cwd: match object.remove("cwd") {
                Some(JsonValue::Null) | None => None,
                Some(value) => Some(json_path_buf(String::from_json(value)?)),
            },
            details: object.take_optional("details")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct ExternalAgentConfigDetectResponse {
    pub items: Vec<ExternalAgentConfigMigrationItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct ExternalAgentConfigDetectParams {
    /// If true, include detection under the user's home (~/.claude, ~/.codex, etc.).
    #[schemars(default, skip_serializing_if = "std::ops::Not::not")]
    pub include_home: bool,
    /// Zero or more working directories to include for repo-scoped detection.
    pub cwds: Option<Vec<PathBuf>>,
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson)]
#[schemars(rename_all = "camelCase")]
pub struct ExternalAgentConfigImportParams {
    pub migration_items: Vec<ExternalAgentConfigMigrationItem>,
}

impl FromJson for ExternalAgentConfigImportParams {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("ExternalAgentConfigImportParams")?;
        Ok(Self {
            migration_items: object.take_required("migrationItems")?,
        })
    }
}

fn json_path_buf(value: String) -> PathBuf {
    PathBuf::from(value)
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct ExternalAgentConfigImportResponse {}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct ExternalAgentConfigImportCompletedNotification {}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct ConfigValueWriteParams {
    pub key_path: String,
    pub value: JsonValue,
    pub merge_strategy: MergeStrategy,
    /// Path to the config file to write; defaults to the user's `config.toml` when omitted.
    pub file_path: Option<String>,
    pub expected_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct ConfigBatchWriteParams {
    pub edits: Vec<ConfigEdit>,
    /// Path to the config file to write; defaults to the user's `config.toml` when omitted.
    pub file_path: Option<String>,
    pub expected_version: Option<String>,
    /// When true, hot-reload the updated user config into all loaded threads after writing.
    #[schemars(default, skip_serializing_if = "std::ops::Not::not")]
    pub reload_user_config: bool,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct ConfigEdit {
    pub key_path: String,
    pub value: JsonValue,
    pub merge_strategy: MergeStrategy,
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct TextPosition {
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number (in Unicode scalar values).
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct TextRange {
    pub start: TextPosition,
    pub end: TextPosition,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct ConfigWarningNotification {
    /// Concise summary of the warning.
    pub summary: String,
    /// Optional extra guidance or error details.
    pub details: Option<String>,
    /// Optional path to the config file that triggered the warning.
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Optional range for the error location inside the config file.
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<TextRange>,
}
