use super::AppSummary;
use super::HookEventName;
use super::HookHandlerType;
use super::HookSource;
use super::HookTrustStatus;
use codex_protocol::compat::absolute_path::AbsolutePathBuf;
use codex_protocol::protocol::SkillDependencies as CoreSkillDependencies;
use codex_protocol::protocol::SkillInterface as CoreSkillInterface;
use codex_protocol::protocol::SkillMetadata as CoreSkillMetadata;
use codex_protocol::protocol::SkillScope as CoreSkillScope;
use codex_protocol::protocol::SkillToolDependency as CoreSkillToolDependency;
use edgerun_json::FromJson;
use edgerun_json::JsonValueError;
use edgerun_json::Map;
use edgerun_json::ToJson;
use edgerun_json::Value as JsonValue;
use schemars::JsonSchema;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct SkillsListParams {
    /// When empty, defaults to the current session working directory.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cwds: Vec<PathBuf>,

    /// When true, bypass the skills cache and re-scan skills from disk.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub force_reload: bool,

    /// Optional per-cwd extra roots to scan as user-scoped skills.
    #[serde(default)]
    pub per_cwd_extra_user_roots: Option<Vec<SkillsListExtraRootsForCwd>>,
}

impl ToJson for SkillsListParams {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::new();
        if !self.cwds.is_empty() {
            object.push_field("cwds", path_bufs_json(&self.cwds));
        }
        if self.force_reload {
            object.push_field("forceReload", true);
        }
        object.push_field(
            "perCwdExtraUserRoots",
            self.per_cwd_extra_user_roots.to_json(),
        );
        JsonValue::Object(object)
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct SkillsListExtraRootsForCwd {
    pub cwd: PathBuf,
    pub extra_user_roots: Vec<PathBuf>,
}

impl ToJson for SkillsListExtraRootsForCwd {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(2);
        object.push_field("cwd", path_buf_json(&self.cwd));
        object.push_field("extraUserRoots", path_bufs_json(&self.extra_user_roots));
        JsonValue::Object(object)
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct SkillsListResponse {
    pub data: Vec<SkillsListEntry>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct HooksListParams {
    /// When empty, defaults to the current session working directory.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cwds: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct HooksListResponse {
    pub data: Vec<HooksListEntry>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceAddParams {
    pub source: String,
    pub ref_name: Option<String>,
    pub sparse_paths: Option<Vec<String>>,
}

impl ToJson for MarketplaceAddParams {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(3);
        object.push_field("source", self.source.clone());
        object.push_field("refName", self.ref_name.to_json());
        object.push_field("sparsePaths", self.sparse_paths.to_json());
        JsonValue::Object(object)
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceAddResponse {
    pub marketplace_name: String,
    pub installed_root: AbsolutePathBuf,
    pub already_added: bool,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceRemoveParams {
    pub marketplace_name: String,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceRemoveResponse {
    pub marketplace_name: String,
    pub installed_root: Option<AbsolutePathBuf>,
}

impl ToJson for MarketplaceRemoveResponse {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(2);
        object.push_field("marketplaceName", self.marketplace_name.clone());
        object.push_field(
            "installedRoot",
            self.installed_root
                .as_ref()
                .map(|path| path.as_path().display().to_string())
                .map(JsonValue::from)
                .unwrap_or(JsonValue::Null),
        );
        JsonValue::Object(object)
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceUpgradeParams {
    pub marketplace_name: Option<String>,
}

impl ToJson for MarketplaceUpgradeParams {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(1);
        object.push_field("marketplaceName", self.marketplace_name.to_json());
        JsonValue::Object(object)
    }
}

impl FromJson for MarketplaceUpgradeParams {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object: Map = Map::try_from(value)?;
        Ok(Self {
            marketplace_name: object.take_optional("marketplaceName")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceUpgradeResponse {
    pub selected_marketplaces: Vec<String>,
    pub upgraded_roots: Vec<AbsolutePathBuf>,
    pub errors: Vec<MarketplaceUpgradeErrorInfo>,
}

impl ToJson for MarketplaceUpgradeResponse {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(3);
        object.push_field("selectedMarketplaces", self.selected_marketplaces.to_json());
        object.push_field(
            "upgradedRoots",
            JsonValue::Array(
                self.upgraded_roots
                    .iter()
                    .map(|path| JsonValue::from(path.as_path().display().to_string()))
                    .collect(),
            ),
        );
        object.push_field("errors", self.errors.to_json());
        JsonValue::Object(object)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceUpgradeErrorInfo {
    pub marketplace_name: String,
    pub message: String,
}

impl ToJson for MarketplaceUpgradeErrorInfo {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(2);
        object.push_field("marketplaceName", self.marketplace_name.clone());
        object.push_field("message", self.message.clone());
        JsonValue::Object(object)
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginListParams {
    /// Optional working directories used to discover repo marketplaces. When omitted,
    /// only home-scoped marketplaces and the official curated marketplace are considered.
    pub cwds: Option<Vec<AbsolutePathBuf>>,
    /// Optional marketplace kind filter. When omitted, only local marketplaces are queried, plus
    /// the default remote catalog when enabled by feature flag.
    pub marketplace_kinds: Option<Vec<PluginListMarketplaceKind>>,
}

impl ToJson for PluginListParams {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(2);
        object.push_field(
            "cwds",
            self.cwds
                .as_ref()
                .map(|paths| JsonValue::Array(paths.iter().map(absolute_path_json).collect()))
                .unwrap_or(JsonValue::Null),
        );
        object.push_field("marketplaceKinds", self.marketplace_kinds.to_json());
        JsonValue::Object(object)
    }
}

impl FromJson for PluginListParams {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object: Map = Map::try_from(value)?;
        Ok(Self {
            cwds: optional_absolute_paths(object.remove("cwds"))?,
            marketplace_kinds: object.take_optional("marketplaceKinds")?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
pub enum PluginListMarketplaceKind {
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "workspace-directory")]
    WorkspaceDirectory,
    #[serde(rename = "shared-with-me")]
    SharedWithMe,
}

impl ToJson for PluginListMarketplaceKind {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            Self::Local => "local",
            Self::WorkspaceDirectory => "workspace-directory",
            Self::SharedWithMe => "shared-with-me",
        })
    }
}

impl FromJson for PluginListMarketplaceKind {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "local" => Ok(Self::Local),
            "workspace-directory" => Ok(Self::WorkspaceDirectory),
            "shared-with-me" => Ok(Self::SharedWithMe),
            other => Err(JsonValueError::WrongType(format!(
                "unknown plugin marketplace kind `{other}`"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct PluginListResponse {
    pub marketplaces: Vec<PluginMarketplaceEntry>,
    #[serde(default)]
    pub marketplace_load_errors: Vec<MarketplaceLoadErrorInfo>,
    #[serde(default)]
    pub featured_plugin_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceLoadErrorInfo {
    pub marketplace_path: AbsolutePathBuf,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginReadParams {
    pub marketplace_path: Option<AbsolutePathBuf>,
    pub remote_marketplace_name: Option<String>,
    pub plugin_name: String,
}

impl ToJson for PluginReadParams {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(3);
        object.push_field(
            "marketplacePath",
            self.marketplace_path
                .as_ref()
                .map(absolute_path_json)
                .unwrap_or(JsonValue::Null),
        );
        object.push_field(
            "remoteMarketplaceName",
            self.remote_marketplace_name.to_json(),
        );
        object.push_field("pluginName", self.plugin_name.clone());
        JsonValue::Object(object)
    }
}

impl FromJson for PluginReadParams {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object: Map = Map::try_from(value)?;
        Ok(Self {
            marketplace_path: optional_absolute_path(object.remove("marketplacePath"))?,
            remote_marketplace_name: object.take_optional("remoteMarketplaceName")?,
            plugin_name: object.take_required("pluginName")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct PluginReadResponse {
    pub plugin: PluginDetail,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct PluginSkillReadParams {
    pub remote_marketplace_name: String,
    pub remote_plugin_id: String,
    pub skill_name: String,
}

impl ToJson for PluginSkillReadParams {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(3);
        object.push_field(
            "remoteMarketplaceName",
            self.remote_marketplace_name.clone(),
        );
        object.push_field("remotePluginId", self.remote_plugin_id.clone());
        object.push_field("skillName", self.skill_name.clone());
        JsonValue::Object(object)
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct PluginSkillReadResponse {
    pub contents: Option<String>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct PluginShareSaveParams {
    pub plugin_path: AbsolutePathBuf,
    pub remote_plugin_id: Option<String>,
    pub discoverability: Option<PluginShareDiscoverability>,
    pub share_targets: Option<Vec<PluginShareTarget>>,
}

impl ToJson for PluginShareSaveParams {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(4);
        object.push_field("pluginPath", absolute_path_json(&self.plugin_path));
        object.push_field("remotePluginId", self.remote_plugin_id.to_json());
        object.push_field("discoverability", self.discoverability.to_json());
        object.push_field("shareTargets", self.share_targets.to_json());
        JsonValue::Object(object)
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct PluginShareSaveResponse {
    pub remote_plugin_id: String,
    pub share_url: String,
}

impl ToJson for PluginShareSaveResponse {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(2);
        object.push_field("remotePluginId", self.remote_plugin_id.clone());
        object.push_field("shareUrl", self.share_url.clone());
        JsonValue::Object(object)
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct PluginShareUpdateTargetsParams {
    pub remote_plugin_id: String,
    pub share_targets: Vec<PluginShareTarget>,
}

impl ToJson for PluginShareUpdateTargetsParams {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(2);
        object.push_field("remotePluginId", self.remote_plugin_id.clone());
        object.push_field("shareTargets", self.share_targets.to_json());
        JsonValue::Object(object)
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct PluginShareUpdateTargetsResponse {
    pub principals: Vec<PluginSharePrincipal>,
}

impl ToJson for PluginShareUpdateTargetsResponse {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(1);
        object.push_field("principals", self.principals.to_json());
        JsonValue::Object(object)
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson)]
#[serde(rename_all = "camelCase")]
pub struct PluginShareListParams {}

impl FromJson for PluginShareListParams {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let _object: Map = Map::try_from(value)?;
        Ok(Self {})
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct PluginShareListResponse {
    pub data: Vec<PluginShareListItem>,
}

impl ToJson for PluginShareListResponse {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(1);
        object.push_field("data", self.data.to_json());
        JsonValue::Object(object)
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct PluginShareDeleteParams {
    pub remote_plugin_id: String,
}

impl ToJson for PluginShareDeleteParams {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(1);
        object.push_field("remotePluginId", self.remote_plugin_id.clone());
        JsonValue::Object(object)
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct PluginShareDeleteResponse {}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct PluginShareListItem {
    pub plugin: PluginSummary,
    pub share_url: String,
    pub local_plugin_path: Option<AbsolutePathBuf>,
}

impl ToJson for PluginShareListItem {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(3);
        object.push_field("plugin", self.plugin.to_json());
        object.push_field("shareUrl", self.share_url.clone());
        object.push_field(
            "localPluginPath",
            self.local_plugin_path
                .as_ref()
                .map(absolute_path_json)
                .unwrap_or(JsonValue::Null),
        );
        JsonValue::Object(object)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema, edgerun_json::FromJson)]
pub enum PluginShareDiscoverability {
    #[serde(rename = "LISTED")]
    Listed,
    #[serde(rename = "UNLISTED")]
    Unlisted,
    #[serde(rename = "PRIVATE")]
    Private,
}

impl ToJson for PluginShareDiscoverability {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            Self::Listed => "LISTED",
            Self::Unlisted => "UNLISTED",
            Self::Private => "PRIVATE",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
pub enum PluginSharePrincipalType {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "group")]
    Group,
    #[serde(rename = "workspace")]
    Workspace,
}

impl ToJson for PluginSharePrincipalType {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            Self::User => "user",
            Self::Group => "group",
            Self::Workspace => "workspace",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct PluginShareTarget {
    pub principal_type: PluginSharePrincipalType,
    pub principal_id: String,
}

impl ToJson for PluginShareTarget {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(2);
        object.push_field("principalType", self.principal_type.to_json());
        object.push_field("principalId", self.principal_id.clone());
        JsonValue::Object(object)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginSharePrincipal {
    pub principal_type: PluginSharePrincipalType,
    pub principal_id: String,
    pub name: String,
}

impl ToJson for PluginSharePrincipal {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(3);
        object.push_field("principalType", self.principal_type.to_json());
        object.push_field("principalId", self.principal_id.clone());
        object.push_field("name", self.name.clone());
        JsonValue::Object(object)
    }
}

impl FromJson for PluginSharePrincipal {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object: Map = Map::try_from(value)?;
        Ok(Self {
            principal_type: object.take_required("principalType")?,
            principal_id: object.take_required("principalId")?,
            name: object.take_required("name")?,
        })
    }
}

impl FromJson for PluginSharePrincipalType {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "user" => Ok(Self::User),
            "group" => Ok(Self::Group),
            "workspace" => Ok(Self::Workspace),
            other => Err(JsonValueError::WrongType(format!(
                "unknown plugin share principal type `{other}`"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "snake_case")]
pub enum SkillScope {
    User,
    Repo,
    System,
    Admin,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Legacy short_description from SKILL.md. Prefer SKILL.json interface.short_description.
    pub short_description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interface: Option<SkillInterface>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<SkillDependencies>,
    pub path: AbsolutePathBuf,
    pub scope: SkillScope,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct SkillInterface {
    pub display_name: Option<String>,
    pub short_description: Option<String>,
    pub icon_small: Option<AbsolutePathBuf>,
    pub icon_large: Option<AbsolutePathBuf>,
    pub brand_color: Option<String>,
    pub default_prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct SkillDependencies {
    pub tools: Vec<SkillToolDependency>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct SkillToolDependency {
    #[serde(rename = "type")]
    pub r#type: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transport: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct SkillErrorInfo {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct SkillsListEntry {
    pub cwd: PathBuf,
    pub skills: Vec<SkillMetadata>,
    pub errors: Vec<SkillErrorInfo>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct HooksListEntry {
    pub cwd: PathBuf,
    pub hooks: Vec<HookMetadata>,
    pub warnings: Vec<String>,
    pub errors: Vec<HookErrorInfo>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct HookMetadata {
    pub key: String,
    pub event_name: HookEventName,
    pub handler_type: HookHandlerType,
    pub matcher: Option<String>,
    pub command: Option<String>,
    pub timeout_sec: u64,
    pub status_message: Option<String>,
    pub source_path: AbsolutePathBuf,
    pub source: HookSource,
    pub plugin_id: Option<String>,
    pub display_order: i64,
    pub enabled: bool,
    pub is_managed: bool,
    pub current_hash: String,
    pub trust_status: HookTrustStatus,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct HookErrorInfo {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct PluginMarketplaceEntry {
    pub name: String,
    /// Local marketplace file path when the marketplace is backed by a local file.
    /// Remote-only catalog marketplaces do not have a local path.
    pub path: Option<AbsolutePathBuf>,
    pub interface: Option<MarketplaceInterface>,
    pub plugins: Vec<PluginSummary>,
}

impl ToJson for PluginMarketplaceEntry {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(4);
        object.push_field("name", self.name.clone());
        object.push_field(
            "path",
            self.path
                .as_ref()
                .map(absolute_path_json)
                .unwrap_or(JsonValue::Null),
        );
        object.push_field("interface", self.interface.to_json());
        object.push_field("plugins", self.plugins.to_json());
        JsonValue::Object(object)
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceInterface {
    pub display_name: Option<String>,
}

impl ToJson for MarketplaceInterface {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(1);
        object.push_field("displayName", self.display_name.to_json());
        JsonValue::Object(object)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
pub enum PluginInstallPolicy {
    #[serde(rename = "NOT_AVAILABLE")]
    NotAvailable,
    #[serde(rename = "AVAILABLE")]
    Available,
    #[serde(rename = "INSTALLED_BY_DEFAULT")]
    InstalledByDefault,
}

impl ToJson for PluginInstallPolicy {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            Self::NotAvailable => "NOT_AVAILABLE",
            Self::Available => "AVAILABLE",
            Self::InstalledByDefault => "INSTALLED_BY_DEFAULT",
        })
    }
}

impl FromJson for PluginInstallPolicy {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "NOT_AVAILABLE" => Ok(Self::NotAvailable),
            "AVAILABLE" => Ok(Self::Available),
            "INSTALLED_BY_DEFAULT" => Ok(Self::InstalledByDefault),
            other => Err(JsonValueError::WrongType(format!(
                "unknown plugin install policy `{other}`"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
pub enum PluginAuthPolicy {
    #[serde(rename = "ON_INSTALL")]
    OnInstall,
    #[serde(rename = "ON_USE")]
    OnUse,
}

impl ToJson for PluginAuthPolicy {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            Self::OnInstall => "ON_INSTALL",
            Self::OnUse => "ON_USE",
        })
    }
}

impl FromJson for PluginAuthPolicy {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "ON_INSTALL" => Ok(Self::OnInstall),
            "ON_USE" => Ok(Self::OnUse),
            other => Err(JsonValueError::WrongType(format!(
                "unknown plugin auth policy `{other}`"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, JsonSchema)]
pub enum PluginAvailability {
    /// Plugin-service currently sends `"ENABLED"` for available remote plugins.
    /// Codex app-server exposes `"AVAILABLE"` in its API; the alias keeps
    /// decoding compatible with that upstream response.
    #[serde(rename = "AVAILABLE", alias = "ENABLED")]
    #[default]
    Available,
    #[serde(rename = "DISABLED_BY_ADMIN")]
    DisabledByAdmin,
}

impl ToJson for PluginAvailability {
    fn to_json(&self) -> JsonValue {
        JsonValue::from(match self {
            Self::Available => "AVAILABLE",
            Self::DisabledByAdmin => "DISABLED_BY_ADMIN",
        })
    }
}

impl FromJson for PluginAvailability {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "AVAILABLE" | "ENABLED" => Ok(Self::Available),
            "DISABLED_BY_ADMIN" => Ok(Self::DisabledByAdmin),
            other => Err(JsonValueError::WrongType(format!(
                "unknown plugin availability `{other}`"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginSummary {
    pub id: String,
    pub name: String,
    /// Remote sharing context associated with this plugin when available.
    pub share_context: Option<PluginShareContext>,
    pub source: PluginSource,
    pub installed: bool,
    pub enabled: bool,
    pub install_policy: PluginInstallPolicy,
    pub auth_policy: PluginAuthPolicy,
    /// Availability state for installing and using the plugin.
    #[serde(default)]
    pub availability: PluginAvailability,
    pub interface: Option<PluginInterface>,
    #[serde(default)]
    pub keywords: Vec<String>,
}

impl ToJson for PluginSummary {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(10);
        object.push_field("id", self.id.clone());
        object.push_field("name", self.name.clone());
        object.push_field("shareContext", self.share_context.to_json());
        object.push_field("source", self.source.to_json());
        object.push_field("installed", self.installed);
        object.push_field("enabled", self.enabled);
        object.push_field("installPolicy", self.install_policy.to_json());
        object.push_field("authPolicy", self.auth_policy.to_json());
        object.push_field("availability", self.availability.to_json());
        object.push_field("interface", self.interface.to_json());
        object.push_field("keywords", self.keywords.to_json());
        JsonValue::Object(object)
    }
}

impl FromJson for PluginSummary {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object: Map = Map::try_from(value)?;
        Ok(Self {
            id: object.take_required("id")?,
            name: object.take_required("name")?,
            share_context: object.take_optional("shareContext")?,
            source: object.take_required("source")?,
            installed: object.take_required("installed")?,
            enabled: object.take_required("enabled")?,
            install_policy: object.take_required("installPolicy")?,
            auth_policy: object.take_required("authPolicy")?,
            availability: object
                .take_optional("availability")?
                .unwrap_or(PluginAvailability::Available),
            interface: object.take_optional("interface")?,
            keywords: object.take_optional("keywords")?.unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginShareContext {
    pub remote_plugin_id: String,
    pub share_url: Option<String>,
    pub creator_account_user_id: Option<String>,
    pub creator_name: Option<String>,
    pub share_targets: Option<Vec<PluginSharePrincipal>>,
}

impl ToJson for PluginShareContext {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(5);
        object.push_field("remotePluginId", self.remote_plugin_id.clone());
        object.push_field("shareUrl", self.share_url.to_json());
        object.push_field(
            "creatorAccountUserId",
            self.creator_account_user_id.to_json(),
        );
        object.push_field("creatorName", self.creator_name.to_json());
        object.push_field("shareTargets", self.share_targets.to_json());
        JsonValue::Object(object)
    }
}

impl FromJson for PluginShareContext {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object: Map = Map::try_from(value)?;
        Ok(Self {
            remote_plugin_id: object.take_required("remotePluginId")?,
            share_url: object.take_optional("shareUrl")?,
            creator_account_user_id: object.take_optional("creatorAccountUserId")?,
            creator_name: object.take_optional("creatorName")?,
            share_targets: object.take_optional("shareTargets")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct PluginDetail {
    pub marketplace_name: String,
    pub marketplace_path: Option<AbsolutePathBuf>,
    pub summary: PluginSummary,
    pub description: Option<String>,
    pub skills: Vec<SkillSummary>,
    pub hooks: Vec<PluginHookSummary>,
    pub apps: Vec<AppSummary>,
    pub mcp_servers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct PluginHookSummary {
    pub key: String,
    pub event_name: HookEventName,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct SkillSummary {
    pub name: String,
    pub description: String,
    pub short_description: Option<String>,
    pub interface: Option<SkillInterface>,
    pub path: Option<AbsolutePathBuf>,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginInterface {
    pub display_name: Option<String>,
    pub short_description: Option<String>,
    pub long_description: Option<String>,
    pub developer_name: Option<String>,
    pub category: Option<String>,
    pub capabilities: Vec<String>,
    pub website_url: Option<String>,
    pub privacy_policy_url: Option<String>,
    pub terms_of_service_url: Option<String>,
    /// Starter prompts for the plugin. Capped at 3 entries with a maximum of
    /// 128 characters per entry.
    pub default_prompt: Option<Vec<String>>,
    pub brand_color: Option<String>,
    /// Local composer icon path, resolved from the installed plugin package.
    pub composer_icon: Option<AbsolutePathBuf>,
    /// Remote composer icon URL from the plugin catalog.
    pub composer_icon_url: Option<String>,
    /// Local logo path, resolved from the installed plugin package.
    pub logo: Option<AbsolutePathBuf>,
    /// Remote logo URL from the plugin catalog.
    pub logo_url: Option<String>,
    /// Local screenshot paths, resolved from the installed plugin package.
    pub screenshots: Vec<AbsolutePathBuf>,
    /// Remote screenshot URLs from the plugin catalog.
    pub screenshot_urls: Vec<String>,
}

impl ToJson for PluginInterface {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(17);
        object.push_field("displayName", self.display_name.to_json());
        object.push_field("shortDescription", self.short_description.to_json());
        object.push_field("longDescription", self.long_description.to_json());
        object.push_field("developerName", self.developer_name.to_json());
        object.push_field("category", self.category.to_json());
        object.push_field("capabilities", self.capabilities.to_json());
        object.push_field("websiteUrl", self.website_url.to_json());
        object.push_field("privacyPolicyUrl", self.privacy_policy_url.to_json());
        object.push_field("termsOfServiceUrl", self.terms_of_service_url.to_json());
        object.push_field("defaultPrompt", self.default_prompt.to_json());
        object.push_field("brandColor", self.brand_color.to_json());
        object.push_field(
            "composerIcon",
            self.composer_icon
                .as_ref()
                .map(absolute_path_json)
                .unwrap_or(JsonValue::Null),
        );
        object.push_field("composerIconUrl", self.composer_icon_url.to_json());
        object.push_field(
            "logo",
            self.logo
                .as_ref()
                .map(absolute_path_json)
                .unwrap_or(JsonValue::Null),
        );
        object.push_field("logoUrl", self.logo_url.to_json());
        object.push_field(
            "screenshots",
            JsonValue::Array(self.screenshots.iter().map(absolute_path_json).collect()),
        );
        object.push_field("screenshotUrls", self.screenshot_urls.to_json());
        JsonValue::Object(object)
    }
}

impl FromJson for PluginInterface {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object: Map = Map::try_from(value)?;
        Ok(Self {
            display_name: object.take_optional("displayName")?,
            short_description: object.take_optional("shortDescription")?,
            long_description: object.take_optional("longDescription")?,
            developer_name: object.take_optional("developerName")?,
            category: object.take_optional("category")?,
            capabilities: object.take_optional("capabilities")?.unwrap_or_default(),
            website_url: object.take_optional("websiteUrl")?,
            privacy_policy_url: object.take_optional("privacyPolicyUrl")?,
            terms_of_service_url: object.take_optional("termsOfServiceUrl")?,
            default_prompt: object.take_optional("defaultPrompt")?,
            brand_color: object.take_optional("brandColor")?,
            composer_icon: optional_absolute_path(object.remove("composerIcon"))?,
            composer_icon_url: object.take_optional("composerIconUrl")?,
            logo: optional_absolute_path(object.remove("logo"))?,
            logo_url: object.take_optional("logoUrl")?,
            screenshots: optional_absolute_paths(object.remove("screenshots"))?.unwrap_or_default(),
            screenshot_urls: object.take_optional("screenshotUrls")?.unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PluginSource {
    #[serde(rename_all = "camelCase")]
    Local { path: AbsolutePathBuf },
    #[serde(rename_all = "camelCase")]
    Git {
        url: String,
        path: Option<String>,
        ref_name: Option<String>,
        sha: Option<String>,
    },
    /// The plugin is available in the remote catalog. Download metadata is
    /// kept server-side and is not exposed through the app-server API.
    Remote,
}

impl ToJson for PluginSource {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::new();
        match self {
            Self::Local { path } => {
                object.push_field("type", "local");
                object.push_field("path", absolute_path_json(path));
            }
            Self::Git {
                url,
                path,
                ref_name,
                sha,
            } => {
                object.push_field("type", "git");
                object.push_field("url", url.clone());
                object.push_field("path", path.to_json());
                object.push_field("refName", ref_name.to_json());
                object.push_field("sha", sha.to_json());
            }
            Self::Remote => {
                object.push_field("type", "remote");
            }
        }
        JsonValue::Object(object)
    }
}

impl FromJson for PluginSource {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object: Map = Map::try_from(value)?;
        let ty: String = object.take_required("type")?;
        match ty.as_str() {
            "local" => Ok(Self::Local {
                path: required_absolute_path(object.remove("path"), "path")?,
            }),
            "git" => Ok(Self::Git {
                url: object.take_required("url")?,
                path: object.take_optional("path")?,
                ref_name: object.take_optional("refName")?,
                sha: object.take_optional("sha")?,
            }),
            "remote" => Ok(Self::Remote),
            other => Err(JsonValueError::WrongType(format!(
                "unknown plugin source type `{other}`"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct SkillsConfigWriteParams {
    /// Path-based selector.
    pub path: Option<AbsolutePathBuf>,
    /// Name-based selector.
    pub name: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct SkillsConfigWriteResponse {
    pub effective_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginInstallParams {
    pub marketplace_path: Option<AbsolutePathBuf>,
    pub remote_marketplace_name: Option<String>,
    pub plugin_name: String,
}

impl ToJson for PluginInstallParams {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(3);
        object.push_field(
            "marketplacePath",
            self.marketplace_path
                .as_ref()
                .map(absolute_path_json)
                .unwrap_or(JsonValue::Null),
        );
        object.push_field(
            "remoteMarketplaceName",
            self.remote_marketplace_name.to_json(),
        );
        object.push_field("pluginName", self.plugin_name.clone());
        JsonValue::Object(object)
    }
}

impl FromJson for PluginInstallParams {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object: Map = Map::try_from(value)?;
        Ok(Self {
            marketplace_path: optional_absolute_path(object.remove("marketplacePath"))?,
            remote_marketplace_name: object.take_optional("remoteMarketplaceName")?,
            plugin_name: object.take_required("pluginName")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct PluginInstallResponse {
    pub auth_policy: PluginAuthPolicy,
    pub apps_needing_auth: Vec<AppSummary>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginUninstallParams {
    pub plugin_id: String,
}

impl ToJson for PluginUninstallParams {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::with_capacity(1);
        object.push_field("pluginId", self.plugin_id.clone());
        JsonValue::Object(object)
    }
}

impl FromJson for PluginUninstallParams {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object: Map = Map::try_from(value)?;
        Ok(Self {
            plugin_id: object.take_required("pluginId")?,
        })
    }
}

fn absolute_path_json(path: &AbsolutePathBuf) -> JsonValue {
    JsonValue::from(path.as_path().display().to_string())
}

fn path_buf_json(path: &PathBuf) -> JsonValue {
    JsonValue::from(path.display().to_string())
}

fn path_bufs_json(paths: &[PathBuf]) -> JsonValue {
    JsonValue::Array(paths.iter().map(path_buf_json).collect())
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

fn required_absolute_path(
    value: Option<JsonValue>,
    field: &str,
) -> Result<AbsolutePathBuf, JsonValueError> {
    optional_absolute_path(value)?
        .ok_or_else(|| JsonValueError::WrongType(format!("missing required field `{field}`")))
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

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct PluginUninstallResponse {}

impl From<CoreSkillMetadata> for SkillMetadata {
    fn from(value: CoreSkillMetadata) -> Self {
        Self {
            name: value.name,
            description: value.description,
            short_description: value.short_description,
            interface: value.interface.map(SkillInterface::from),
            dependencies: value.dependencies.map(SkillDependencies::from),
            path: value.path,
            scope: value.scope.into(),
            enabled: true,
        }
    }
}

impl From<CoreSkillInterface> for SkillInterface {
    fn from(value: CoreSkillInterface) -> Self {
        Self {
            display_name: value.display_name,
            short_description: value.short_description,
            brand_color: value.brand_color,
            default_prompt: value.default_prompt,
            icon_small: value.icon_small,
            icon_large: value.icon_large,
        }
    }
}

impl From<CoreSkillDependencies> for SkillDependencies {
    fn from(value: CoreSkillDependencies) -> Self {
        Self {
            tools: value
                .tools
                .into_iter()
                .map(SkillToolDependency::from)
                .collect(),
        }
    }
}

impl From<CoreSkillToolDependency> for SkillToolDependency {
    fn from(value: CoreSkillToolDependency) -> Self {
        Self {
            r#type: value.r#type,
            value: value.value,
            description: value.description,
            transport: value.transport,
            command: value.command,
            url: value.url,
        }
    }
}

impl From<CoreSkillScope> for SkillScope {
    fn from(value: CoreSkillScope) -> Self {
        match value {
            CoreSkillScope::User => Self::User,
            CoreSkillScope::Repo => Self::Repo,
            CoreSkillScope::System => Self::System,
            CoreSkillScope::Admin => Self::Admin,
        }
    }
}
#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
/// Notification emitted when watched local skill files change.
///
/// Treat this as an invalidation signal and re-run `skills/list` with the
/// client's current parameters when refreshed skill metadata is needed.
pub struct SkillsChangedNotification {}
