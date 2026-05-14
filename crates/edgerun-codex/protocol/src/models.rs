use std::collections::BTreeMap;
use std::io;
use std::num::NonZeroUsize;
use std::path::Path;

use crate::compat::image::PromptImageMode;
use crate::compat::image::load_for_prompt_bytes;
use edgerun_json::FromJson;
use edgerun_json::JsonValueError;
use edgerun_json::Map;
use edgerun_json::ToJson;
use edgerun_json::Value;
use edgerun_serde::Deserialize;
use edgerun_serde::Deserializer;
use edgerun_serde::Serialize;
use edgerun_serde::ser::Serializer;
use ts_rs::TS;

use crate::compat::absolute_path::AbsolutePathBuf;
use crate::compat::image::ImageProcessingError;
use crate::permissions::FileSystemAccessMode;
use crate::permissions::FileSystemPath;
use crate::permissions::FileSystemSandboxEntry;
use crate::permissions::FileSystemSandboxKind;
use crate::permissions::FileSystemSandboxPolicy;
use crate::permissions::FileSystemSpecialPath;
use crate::permissions::NetworkSandboxPolicy;
use crate::protocol::SandboxPolicy;
use crate::user_input::UserInput;
use schemars::JsonSchema;

use crate::mcp::CallToolResult;

#[derive(Debug, Clone, Default, Eq, Hash, PartialEq, JsonSchema, TS)]
pub struct FileSystemPermissions {
    pub entries: Vec<FileSystemSandboxEntry>,
    pub glob_scan_max_depth: Option<NonZeroUsize>,
}

pub type LegacyReadWriteRoots = (Option<Vec<AbsolutePathBuf>>, Option<Vec<AbsolutePathBuf>>);

impl FileSystemPermissions {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn from_read_write_roots(
        read: Option<Vec<AbsolutePathBuf>>,
        write: Option<Vec<AbsolutePathBuf>>,
    ) -> Self {
        let mut entries = Vec::new();
        if let Some(read) = read {
            entries.extend(read.into_iter().map(|path| FileSystemSandboxEntry {
                path: FileSystemPath::Path { path },
                access: FileSystemAccessMode::Read,
            }));
        }
        if let Some(write) = write {
            entries.extend(write.into_iter().map(|path| FileSystemSandboxEntry {
                path: FileSystemPath::Path { path },
                access: FileSystemAccessMode::Write,
            }));
        }
        Self {
            entries,
            glob_scan_max_depth: None,
        }
    }

    pub fn explicit_path_entries(
        &self,
    ) -> impl Iterator<Item = (&AbsolutePathBuf, FileSystemAccessMode)> {
        self.entries.iter().filter_map(|entry| match &entry.path {
            FileSystemPath::Path { path } => Some((path, entry.access)),
            FileSystemPath::GlobPattern { .. } | FileSystemPath::Special { .. } => None,
        })
    }

    pub fn legacy_read_write_roots(&self) -> Option<LegacyReadWriteRoots> {
        self.as_legacy_permissions()
            .map(|legacy| (legacy.read, legacy.write))
    }

    fn as_legacy_permissions(&self) -> Option<LegacyFileSystemPermissions> {
        if self.glob_scan_max_depth.is_some() {
            return None;
        }

        let mut read = Vec::new();
        let mut write = Vec::new();

        for entry in &self.entries {
            let FileSystemPath::Path { path } = &entry.path else {
                return None;
            };
            match entry.access {
                FileSystemAccessMode::Read => read.push(path.clone()),
                FileSystemAccessMode::Write => write.push(path.clone()),
                FileSystemAccessMode::None => return None,
            }
        }

        Some(LegacyFileSystemPermissions {
            read: (!read.is_empty()).then_some(read),
            write: (!write.is_empty()).then_some(write),
        })
    }
}

#[derive(Debug, Clone, Default, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyFileSystemPermissions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    read: Option<Vec<AbsolutePathBuf>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    write: Option<Vec<AbsolutePathBuf>>,
}

#[derive(Debug, Clone, Default, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CanonicalFileSystemPermissions {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    entries: Vec<FileSystemSandboxEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    glob_scan_max_depth: Option<NonZeroUsize>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum FileSystemPermissionsDe {
    Canonical(CanonicalFileSystemPermissions),
    Legacy(LegacyFileSystemPermissions),
}

impl Serialize for FileSystemPermissions {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(legacy) = self.as_legacy_permissions() {
            legacy.serialize(serializer)
        } else {
            CanonicalFileSystemPermissions {
                entries: self.entries.clone(),
                glob_scan_max_depth: self.glob_scan_max_depth,
            }
            .serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for FileSystemPermissions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match FileSystemPermissionsDe::deserialize(deserializer)? {
            FileSystemPermissionsDe::Canonical(CanonicalFileSystemPermissions {
                entries,
                glob_scan_max_depth,
            }) => Ok(Self {
                entries,
                glob_scan_max_depth,
            }),
            FileSystemPermissionsDe::Legacy(LegacyFileSystemPermissions { read, write }) => {
                Ok(Self::from_read_write_roots(read, write))
            }
        }
    }
}

impl ToJson for FileSystemPermissions {
    fn to_json(&self) -> Value {
        if let Some(legacy) = self.as_legacy_permissions() {
            let mut object = Map::new();
            if let Some(read) = legacy.read {
                object.push_field("read", read.to_json());
            }
            if let Some(write) = legacy.write {
                object.push_field("write", write.to_json());
            }
            return Value::Object(object);
        }

        let mut object = Map::new();
        if !self.entries.is_empty() {
            object.push_field("entries", self.entries.to_json());
        }
        if let Some(depth) = self.glob_scan_max_depth {
            object.push_field("glob_scan_max_depth", depth.get());
        }
        Value::Object(object)
    }
}

impl FromJson for FileSystemPermissions {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("FileSystemPermissions")?;
        if object.get("entries").is_some() || object.get("glob_scan_max_depth").is_some() {
            let entries = object.take_optional("entries")?.unwrap_or_default();
            let glob_scan_max_depth = object
                .take_optional::<usize>("glob_scan_max_depth")?
                .map(|depth| {
                    NonZeroUsize::new(depth).ok_or_else(|| {
                        JsonValueError::WrongType(
                            "glob_scan_max_depth must be greater than zero".to_string(),
                        )
                    })
                })
                .transpose()?;
            return Ok(Self {
                entries,
                glob_scan_max_depth,
            });
        }

        let read = object.take_optional("read")?;
        let write = object.take_optional("write")?;
        Ok(Self::from_read_write_roots(read, write))
    }
}

#[derive(Debug, Clone, Default, Eq, Hash, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct NetworkPermissions {
    pub enabled: Option<bool>,
}

impl ToJson for NetworkPermissions {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        if let Some(enabled) = self.enabled {
            object.push_field("enabled", enabled);
        }
        Value::Object(object)
    }
}

impl FromJson for NetworkPermissions {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("NetworkPermissions")?;
        Ok(Self {
            enabled: object.take_optional("enabled")?,
        })
    }
}

impl NetworkPermissions {
    pub fn is_empty(&self) -> bool {
        self.enabled.is_none()
    }
}

#[derive(
    Debug, Clone, Copy, Default, Eq, Hash, PartialEq, Serialize, Deserialize, JsonSchema, TS,
)]
#[serde(rename_all = "snake_case")]
pub enum SandboxEnforcement {
    /// Codex owns sandbox construction for this profile.
    #[default]
    Managed,
    /// No outer filesystem sandbox should be applied.
    Disabled,
    /// Filesystem isolation is enforced by an external caller.
    External,
}

impl SandboxEnforcement {
    pub fn from_legacy_sandbox_policy(sandbox_policy: &SandboxPolicy) -> Self {
        match sandbox_policy {
            SandboxPolicy::DangerFullAccess => Self::Disabled,
            SandboxPolicy::ExternalSandbox { .. } => Self::External,
            SandboxPolicy::ReadOnly { .. } | SandboxPolicy::WorkspaceWrite { .. } => Self::Managed,
        }
    }
}

/// Filesystem permissions for profiles where Codex owns sandbox construction.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(tag = "type")]
pub enum ManagedFileSystemPermissions {
    /// Apply a managed filesystem sandbox from the listed entries.
    #[serde(rename_all = "snake_case")]
    #[ts(rename_all = "snake_case")]
    Restricted {
        entries: Vec<FileSystemSandboxEntry>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        glob_scan_max_depth: Option<NonZeroUsize>,
    },
    /// Apply a managed sandbox that allows all filesystem access.
    Unrestricted,
}

impl ManagedFileSystemPermissions {
    fn from_sandbox_policy(file_system_sandbox_policy: &FileSystemSandboxPolicy) -> Self {
        match file_system_sandbox_policy.kind {
            FileSystemSandboxKind::Restricted => Self::Restricted {
                entries: file_system_sandbox_policy.entries.clone(),
                glob_scan_max_depth: file_system_sandbox_policy
                    .glob_scan_max_depth
                    .and_then(NonZeroUsize::new),
            },
            FileSystemSandboxKind::Unrestricted => Self::Unrestricted,
            FileSystemSandboxKind::ExternalSandbox => unreachable!(
                "external filesystem policies are represented by PermissionProfile::External"
            ),
        }
    }

    pub fn to_sandbox_policy(&self) -> FileSystemSandboxPolicy {
        match self {
            Self::Restricted {
                entries,
                glob_scan_max_depth,
            } => FileSystemSandboxPolicy {
                kind: FileSystemSandboxKind::Restricted,
                glob_scan_max_depth: glob_scan_max_depth.map(usize::from),
                entries: entries.clone(),
            },
            Self::Unrestricted => FileSystemSandboxPolicy::unrestricted(),
        }
    }
}

/// Canonical active runtime permissions for a conversation, turn, or command.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(tag = "type")]
pub enum PermissionProfile {
    /// Codex owns sandbox construction for this profile.
    #[serde(rename_all = "snake_case")]
    #[ts(rename_all = "snake_case")]
    Managed {
        file_system: ManagedFileSystemPermissions,
        network: NetworkSandboxPolicy,
    },
    /// Do not apply an outer sandbox.
    Disabled,
    /// Filesystem isolation is enforced by an external caller.
    #[serde(rename_all = "snake_case")]
    #[ts(rename_all = "snake_case")]
    External { network: NetworkSandboxPolicy },
}

impl Default for PermissionProfile {
    fn default() -> Self {
        Self::Managed {
            file_system: ManagedFileSystemPermissions::Restricted {
                entries: Vec::new(),
                glob_scan_max_depth: None,
            },
            network: NetworkSandboxPolicy::Restricted,
        }
    }
}

impl PermissionProfile {
    /// Managed read-only filesystem access with restricted network access.
    pub fn read_only() -> Self {
        Self::Managed {
            file_system: ManagedFileSystemPermissions::Restricted {
                entries: vec![FileSystemSandboxEntry {
                    path: FileSystemPath::Special {
                        value: FileSystemSpecialPath::Root,
                    },
                    access: FileSystemAccessMode::Read,
                }],
                glob_scan_max_depth: None,
            },
            network: NetworkSandboxPolicy::Restricted,
        }
    }

    /// Managed workspace-write filesystem access with restricted network
    /// access.
    ///
    /// The returned profile contains symbolic `:project_roots` entries that
    /// must be resolved against the active permission root before enforcement.
    pub fn workspace_write() -> Self {
        Self::workspace_write_with(
            &[],
            NetworkSandboxPolicy::Restricted,
            /*exclude_tmpdir_env_var*/ false,
            /*exclude_slash_tmp*/ false,
        )
    }

    /// Managed workspace-write filesystem access with the legacy
    /// `sandbox_workspace_write` knobs applied directly to the profile.
    ///
    /// The returned profile contains symbolic `:project_roots` entries that
    /// must be resolved against the active permission root before enforcement.
    pub fn workspace_write_with(
        writable_roots: &[AbsolutePathBuf],
        network: NetworkSandboxPolicy,
        exclude_tmpdir_env_var: bool,
        exclude_slash_tmp: bool,
    ) -> Self {
        let file_system = FileSystemSandboxPolicy::workspace_write(
            writable_roots,
            exclude_tmpdir_env_var,
            exclude_slash_tmp,
        );
        Self::Managed {
            file_system: ManagedFileSystemPermissions::from_sandbox_policy(&file_system),
            network,
        }
    }

    pub fn from_runtime_permissions(
        file_system_sandbox_policy: &FileSystemSandboxPolicy,
        network_sandbox_policy: NetworkSandboxPolicy,
    ) -> Self {
        let enforcement = match file_system_sandbox_policy.kind {
            FileSystemSandboxKind::Restricted | FileSystemSandboxKind::Unrestricted => {
                SandboxEnforcement::Managed
            }
            FileSystemSandboxKind::ExternalSandbox => SandboxEnforcement::External,
        };
        Self::from_runtime_permissions_with_enforcement(
            enforcement,
            file_system_sandbox_policy,
            network_sandbox_policy,
        )
    }

    pub fn from_runtime_permissions_with_enforcement(
        enforcement: SandboxEnforcement,
        file_system_sandbox_policy: &FileSystemSandboxPolicy,
        network_sandbox_policy: NetworkSandboxPolicy,
    ) -> Self {
        match file_system_sandbox_policy.kind {
            FileSystemSandboxKind::ExternalSandbox => Self::External {
                network: network_sandbox_policy,
            },
            FileSystemSandboxKind::Unrestricted if enforcement == SandboxEnforcement::Disabled => {
                Self::Disabled
            }
            FileSystemSandboxKind::Restricted | FileSystemSandboxKind::Unrestricted => {
                Self::Managed {
                    file_system: ManagedFileSystemPermissions::from_sandbox_policy(
                        file_system_sandbox_policy,
                    ),
                    network: network_sandbox_policy,
                }
            }
        }
    }

    pub fn from_legacy_sandbox_policy(sandbox_policy: &SandboxPolicy) -> Self {
        Self::from_runtime_permissions_with_enforcement(
            SandboxEnforcement::from_legacy_sandbox_policy(sandbox_policy),
            &FileSystemSandboxPolicy::from(sandbox_policy),
            NetworkSandboxPolicy::from(sandbox_policy),
        )
    }

    pub fn from_legacy_sandbox_policy_for_cwd(sandbox_policy: &SandboxPolicy, cwd: &Path) -> Self {
        Self::from_runtime_permissions_with_enforcement(
            SandboxEnforcement::from_legacy_sandbox_policy(sandbox_policy),
            &FileSystemSandboxPolicy::from_legacy_sandbox_policy_for_cwd(sandbox_policy, cwd),
            NetworkSandboxPolicy::from(sandbox_policy),
        )
    }

    pub fn enforcement(&self) -> SandboxEnforcement {
        match self {
            Self::Managed { .. } => SandboxEnforcement::Managed,
            Self::Disabled => SandboxEnforcement::Disabled,
            Self::External { .. } => SandboxEnforcement::External,
        }
    }

    pub fn file_system_sandbox_policy(&self) -> FileSystemSandboxPolicy {
        match self {
            Self::Managed { file_system, .. } => file_system.to_sandbox_policy(),
            Self::Disabled => FileSystemSandboxPolicy::unrestricted(),
            Self::External { .. } => FileSystemSandboxPolicy::external_sandbox(),
        }
    }

    pub fn network_sandbox_policy(&self) -> NetworkSandboxPolicy {
        match self {
            Self::Managed { network, .. } | Self::External { network } => *network,
            Self::Disabled => NetworkSandboxPolicy::Enabled,
        }
    }

    pub fn to_legacy_sandbox_policy(&self, cwd: &Path) -> io::Result<SandboxPolicy> {
        match self {
            Self::Managed {
                file_system,
                network,
            } => file_system
                .to_sandbox_policy()
                .to_legacy_sandbox_policy(*network, cwd),
            Self::Disabled => Ok(SandboxPolicy::DangerFullAccess),
            Self::External { network } => Ok(SandboxPolicy::ExternalSandbox {
                network_access: if network.is_enabled() {
                    crate::protocol::NetworkAccess::Enabled
                } else {
                    crate::protocol::NetworkAccess::Restricted
                },
            }),
        }
    }

    pub fn to_runtime_permissions(&self) -> (FileSystemSandboxPolicy, NetworkSandboxPolicy) {
        (
            self.file_system_sandbox_policy(),
            self.network_sandbox_policy(),
        )
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum TaggedPermissionProfile {
    #[serde(rename_all = "snake_case")]
    Managed {
        file_system: ManagedFileSystemPermissions,
        network: NetworkSandboxPolicy,
    },
    Disabled,
    #[serde(rename_all = "snake_case")]
    External {
        network: NetworkSandboxPolicy,
    },
}

impl From<TaggedPermissionProfile> for PermissionProfile {
    fn from(value: TaggedPermissionProfile) -> Self {
        match value {
            TaggedPermissionProfile::Managed {
                file_system,
                network,
            } => Self::Managed {
                file_system,
                network,
            },
            TaggedPermissionProfile::Disabled => Self::Disabled,
            TaggedPermissionProfile::External { network } => Self::External { network },
        }
    }
}

/// Pre-tagged shape written to rollout files before `PermissionProfile`
/// represented enforcement explicitly.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyPermissionProfile {
    network: Option<NetworkPermissions>,
    file_system: Option<FileSystemPermissions>,
}

impl From<LegacyPermissionProfile> for PermissionProfile {
    fn from(value: LegacyPermissionProfile) -> Self {
        let file_system_sandbox_policy = value.file_system.as_ref().map_or_else(
            || FileSystemSandboxPolicy::restricted(Vec::new()),
            FileSystemSandboxPolicy::from,
        );
        let network_sandbox_policy = if value
            .network
            .as_ref()
            .and_then(|network| network.enabled)
            .unwrap_or(false)
        {
            NetworkSandboxPolicy::Enabled
        } else {
            NetworkSandboxPolicy::Restricted
        };
        Self::from_runtime_permissions(&file_system_sandbox_policy, network_sandbox_policy)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum PermissionProfileDe {
    Tagged(TaggedPermissionProfile),
    Legacy(LegacyPermissionProfile),
}

impl<'de> Deserialize<'de> for PermissionProfile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match PermissionProfileDe::deserialize(deserializer)? {
            PermissionProfileDe::Tagged(tagged) => tagged.into(),
            PermissionProfileDe::Legacy(legacy) => legacy.into(),
        })
    }
}

impl From<NetworkSandboxPolicy> for NetworkPermissions {
    fn from(value: NetworkSandboxPolicy) -> Self {
        Self {
            enabled: Some(value.is_enabled()),
        }
    }
}

impl From<&FileSystemSandboxPolicy> for FileSystemPermissions {
    fn from(value: &FileSystemSandboxPolicy) -> Self {
        let entries = match value.kind {
            FileSystemSandboxKind::Restricted => value.entries.clone(),
            FileSystemSandboxKind::Unrestricted | FileSystemSandboxKind::ExternalSandbox => {
                vec![FileSystemSandboxEntry {
                    path: FileSystemPath::Special {
                        value: FileSystemSpecialPath::Root,
                    },
                    access: FileSystemAccessMode::Write,
                }]
            }
        };
        Self {
            entries,
            glob_scan_max_depth: value.glob_scan_max_depth.and_then(NonZeroUsize::new),
        }
    }
}

impl From<&FileSystemPermissions> for FileSystemSandboxPolicy {
    fn from(value: &FileSystemPermissions) -> Self {
        let mut policy = FileSystemSandboxPolicy::restricted(value.entries.clone());
        policy.glob_scan_max_depth = value.glob_scan_max_depth.map(usize::from);
        policy
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseInputItem {
    Message {
        role: String,
        content: Vec<ContentItem>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        phase: Option<MessagePhase>,
    },
    FunctionCallOutput {
        call_id: String,
        #[ts(as = "FunctionCallOutputBody")]
        #[schemars(with = "FunctionCallOutputBody")]
        output: FunctionCallOutputPayload,
    },
    McpToolCallOutput {
        call_id: String,
        output: CallToolResult,
    },
    CustomToolCallOutput {
        call_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        name: Option<String>,
        #[ts(as = "FunctionCallOutputBody")]
        #[schemars(with = "FunctionCallOutputBody")]
        output: FunctionCallOutputPayload,
    },
    ToolSearchOutput {
        call_id: String,
        status: String,
        execution: String,
        #[ts(type = "unknown[]")]
        tools: Vec<edgerun_json::Value>,
    },
}

impl ToJson for ResponseInputItem {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        match self {
            Self::Message {
                role,
                content,
                phase,
            } => {
                object.push_field("type", "message");
                object.push_field("role", role.clone());
                object.push_field("content", content.to_json());
                object.push_opt_field("phase", phase.as_ref().map(ToJson::to_json));
            }
            Self::FunctionCallOutput { call_id, output } => {
                object.push_field("type", "function_call_output");
                object.push_field("call_id", call_id.clone());
                object.push_field("output", output.to_json());
            }
            Self::McpToolCallOutput { call_id, output } => {
                object.push_field("type", "mcp_tool_call_output");
                object.push_field("call_id", call_id.clone());
                object.push_field("output", call_tool_result_to_json(output));
            }
            Self::CustomToolCallOutput {
                call_id,
                name,
                output,
            } => {
                object.push_field("type", "custom_tool_call_output");
                object.push_field("call_id", call_id.clone());
                object.push_opt_field("name", name.clone());
                object.push_field("output", output.to_json());
            }
            Self::ToolSearchOutput {
                call_id,
                status,
                execution,
                tools,
            } => {
                object.push_field("type", "tool_search_output");
                object.push_field("call_id", call_id.clone());
                object.push_field("status", status.clone());
                object.push_field("execution", execution.clone());
                object.push_field("tools", tools.to_json());
            }
        }
        Value::Object(object)
    }
}

fn call_tool_result_to_json(output: &CallToolResult) -> Value {
    let mut object = Map::with_capacity(5);
    object.push_field("content", output.content.to_json());
    object.push_opt_field("structuredContent", output.structured_content.clone());
    object.push_opt_field("isError", output.is_error);
    object.push_opt_field("_meta", output.meta.clone());
    Value::Object(object)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentItem {
    InputText {
        text: String,
    },
    InputImage {
        image_url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        detail: Option<ImageDetail>,
    },
    OutputText {
        text: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
pub enum ImageDetail {
    Auto,
    Low,
    High,
    Original,
}

pub const DEFAULT_IMAGE_DETAIL: ImageDetail = ImageDetail::High;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
/// Classifies an assistant message as interim commentary or final answer text.
///
/// Providers do not emit this consistently, so callers must treat `None` as
/// "phase unknown" and keep compatibility behavior for legacy models.
pub enum MessagePhase {
    /// Mid-turn assistant text (for example preamble/progress narration).
    ///
    /// Additional tool calls or assistant output may follow before turn
    /// completion.
    Commentary,
    /// The assistant's terminal answer text for the current turn.
    FinalAnswer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseItem {
    Message {
        #[serde(default, skip_serializing)]
        #[ts(skip)]
        id: Option<String>,
        role: String,
        content: Vec<ContentItem>,
        // Optional output-message phase (for example: "commentary", "final_answer").
        // Availability varies by provider/model, so downstream consumers must
        // preserve fallback behavior when this is absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        phase: Option<MessagePhase>,
    },
    Reasoning {
        #[serde(default, skip_serializing)]
        #[ts(skip)]
        #[schemars(skip)]
        id: String,
        summary: Vec<ReasoningItemReasoningSummary>,
        #[serde(default, skip_serializing_if = "should_serialize_reasoning_content")]
        #[ts(optional)]
        content: Option<Vec<ReasoningItemContent>>,
        encrypted_content: Option<String>,
    },
    LocalShellCall {
        /// Legacy id field retained for compatibility with older payloads.
        #[serde(default, skip_serializing)]
        #[ts(skip)]
        id: Option<String>,
        /// Set when using the Responses API.
        call_id: Option<String>,
        status: LocalShellStatus,
        action: LocalShellAction,
    },
    FunctionCall {
        #[serde(default, skip_serializing)]
        #[ts(skip)]
        id: Option<String>,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        namespace: Option<String>,
        // The Responses API returns the function call arguments as a *string* that contains
        // JSON, not as an already‑parsed object. We keep it as a raw string here and let
        // Session::handle_function_call parse it into a Value.
        arguments: String,
        call_id: String,
    },
    ToolSearchCall {
        #[serde(default, skip_serializing)]
        #[ts(skip)]
        id: Option<String>,
        call_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        status: Option<String>,
        execution: String,
        #[ts(type = "unknown")]
        arguments: edgerun_json::Value,
    },
    // NOTE: The `output` field for `function_call_output` uses a dedicated payload type with
    // custom serialization. On the wire it is either:
    //   - a plain string (`content`)
    //   - an array of structured content items (`content_items`)
    // We keep this behavior centralized in `FunctionCallOutputPayload`.
    FunctionCallOutput {
        call_id: String,
        #[ts(as = "FunctionCallOutputBody")]
        #[schemars(with = "FunctionCallOutputBody")]
        output: FunctionCallOutputPayload,
    },
    CustomToolCall {
        #[serde(default, skip_serializing)]
        #[ts(skip)]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        status: Option<String>,

        call_id: String,
        name: String,
        input: String,
    },
    // `custom_tool_call_output.output` uses the same wire encoding as
    // `function_call_output.output` so freeform tools can return either plain
    // text or structured content items.
    CustomToolCallOutput {
        call_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        name: Option<String>,
        #[ts(as = "FunctionCallOutputBody")]
        #[schemars(with = "FunctionCallOutputBody")]
        output: FunctionCallOutputPayload,
    },
    ToolSearchOutput {
        call_id: Option<String>,
        status: String,
        execution: String,
        #[ts(type = "unknown[]")]
        tools: Vec<edgerun_json::Value>,
    },
    // Emitted by the Responses API when the agent triggers a web search.
    // Example payload (from SSE `response.output_item.done`):
    // {
    //   "id":"ws_...",
    //   "type":"web_search_call",
    //   "status":"completed",
    //   "action": {"type":"search","query":"weather: San Francisco, CA"}
    // }
    WebSearchCall {
        #[serde(default, skip_serializing)]
        #[ts(skip)]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        status: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        action: Option<WebSearchAction>,
    },
    // Emitted by the Responses API when the agent triggers image generation.
    // Example payload:
    // {
    //   "id":"ig_123",
    //   "type":"image_generation_call",
    //   "status":"completed",
    //   "revised_prompt":"A gray tabby cat hugging an otter...",
    //   "result":"..."
    // }
    ImageGenerationCall {
        id: String,
        status: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        revised_prompt: Option<String>,
        result: String,
    },
    #[serde(alias = "compaction_summary")]
    Compaction { encrypted_content: String },
    ContextCompaction {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        encrypted_content: Option<String>,
    },
    #[serde(other)]
    Other,
}

fn json_wrong_type(message: impl Into<String>) -> JsonValueError {
    JsonValueError::WrongType(message.into())
}

fn json_type(mut object: Map, context: &str) -> Result<(String, Map), JsonValueError> {
    let kind: String = object.take_required("type")?;
    if kind.is_empty() {
        return Err(json_wrong_type(format!("{context} type must not be empty")));
    }
    Ok((kind, object))
}

impl ToJson for MessagePhase {
    fn to_json(&self) -> Value {
        match self {
            Self::Commentary => "commentary",
            Self::FinalAnswer => "final_answer",
        }
        .to_json()
    }
}

impl FromJson for MessagePhase {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "commentary" => Ok(Self::Commentary),
            "final_answer" => Ok(Self::FinalAnswer),
            other => Err(json_wrong_type(format!("unknown message phase `{other}`"))),
        }
    }
}

impl ToJson for ImageDetail {
    fn to_json(&self) -> Value {
        match self {
            Self::Auto => "auto",
            Self::Low => "low",
            Self::High => "high",
            Self::Original => "original",
        }
        .to_json()
    }
}

impl FromJson for ImageDetail {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "auto" => Ok(Self::Auto),
            "low" => Ok(Self::Low),
            "high" => Ok(Self::High),
            "original" => Ok(Self::Original),
            other => Err(json_wrong_type(format!("unknown image detail `{other}`"))),
        }
    }
}

impl ToJson for ContentItem {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        match self {
            Self::InputText { text } => {
                object.push_field("type", "input_text");
                object.push_field("text", text.clone());
            }
            Self::InputImage { image_url, detail } => {
                object.push_field("type", "input_image");
                object.push_field("image_url", image_url.clone());
                object.push_opt_field("detail", detail.as_ref().map(ToJson::to_json));
            }
            Self::OutputText { text } => {
                object.push_field("type", "output_text");
                object.push_field("text", text.clone());
            }
        }
        object.into()
    }
}

impl FromJson for ContentItem {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let (kind, mut object) = json_type(value.into_object("ContentItem")?, "ContentItem")?;
        match kind.as_str() {
            "input_text" => Ok(Self::InputText {
                text: object.take_required("text")?,
            }),
            "input_image" => Ok(Self::InputImage {
                image_url: object.take_required("image_url")?,
                detail: object.take_optional("detail")?,
            }),
            "output_text" => Ok(Self::OutputText {
                text: object.take_required("text")?,
            }),
            other => Err(json_wrong_type(format!("unknown content item `{other}`"))),
        }
    }
}

impl ToJson for LocalShellStatus {
    fn to_json(&self) -> Value {
        match self {
            Self::Completed => "completed",
            Self::InProgress => "in_progress",
            Self::Incomplete => "incomplete",
        }
        .to_json()
    }
}

impl FromJson for LocalShellStatus {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "completed" => Ok(Self::Completed),
            "in_progress" => Ok(Self::InProgress),
            "incomplete" => Ok(Self::Incomplete),
            other => Err(json_wrong_type(format!(
                "unknown local shell status `{other}`"
            ))),
        }
    }
}

impl ToJson for LocalShellExecAction {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("command", self.command.to_json());
        object.push_field("timeout_ms", self.timeout_ms.to_json());
        object.push_field("working_directory", self.working_directory.to_json());
        object.push_field("env", self.env.to_json());
        object.push_field("user", self.user.to_json());
        object.into()
    }
}

impl FromJson for LocalShellExecAction {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("LocalShellExecAction")?;
        Ok(Self {
            command: object.take_required("command")?,
            timeout_ms: object.take_optional("timeout_ms")?,
            working_directory: object.take_optional("working_directory")?,
            env: object.take_optional("env")?,
            user: object.take_optional("user")?,
        })
    }
}

impl ToJson for LocalShellAction {
    fn to_json(&self) -> Value {
        match self {
            Self::Exec(action) => {
                let mut object = Map::new();
                object.push_field("type", "exec");
                object.push_field("command", action.command.to_json());
                object.push_field("timeout_ms", action.timeout_ms.to_json());
                object.push_field("working_directory", action.working_directory.to_json());
                object.push_field("env", action.env.to_json());
                object.push_field("user", action.user.to_json());
                object.into()
            }
        }
    }
}

impl FromJson for LocalShellAction {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let (kind, object) = json_type(value.into_object("LocalShellAction")?, "LocalShellAction")?;
        match kind.as_str() {
            "exec" => Ok(Self::Exec(LocalShellExecAction::from_json(object.into())?)),
            other => Err(json_wrong_type(format!(
                "unknown local shell action `{other}`"
            ))),
        }
    }
}

impl ToJson for WebSearchAction {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        match self {
            Self::Search { query, queries } => {
                object.push_field("type", "search");
                object.push_opt_field("query", query.clone());
                object.push_opt_field("queries", queries.clone());
            }
            Self::OpenPage { url } => {
                object.push_field("type", "open_page");
                object.push_opt_field("url", url.clone());
            }
            Self::FindInPage { url, pattern } => {
                object.push_field("type", "find_in_page");
                object.push_opt_field("url", url.clone());
                object.push_opt_field("pattern", pattern.clone());
            }
            Self::Other => {
                object.push_field("type", "other");
            }
        }
        object.into()
    }
}

impl FromJson for WebSearchAction {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let (kind, mut object) =
            json_type(value.into_object("WebSearchAction")?, "WebSearchAction")?;
        match kind.as_str() {
            "search" => Ok(Self::Search {
                query: object.take_optional("query")?,
                queries: object.take_optional("queries")?,
            }),
            "open_page" => Ok(Self::OpenPage {
                url: object.take_optional("url")?,
            }),
            "find_in_page" => Ok(Self::FindInPage {
                url: object.take_optional("url")?,
                pattern: object.take_optional("pattern")?,
            }),
            _ => Ok(Self::Other),
        }
    }
}

impl ToJson for ReasoningItemReasoningSummary {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        match self {
            Self::SummaryText { text } => {
                object.push_field("type", "summary_text");
                object.push_field("text", text.clone());
            }
        }
        object.into()
    }
}

impl FromJson for ReasoningItemReasoningSummary {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let (kind, mut object) = json_type(
            value.into_object("ReasoningItemReasoningSummary")?,
            "ReasoningItemReasoningSummary",
        )?;
        match kind.as_str() {
            "summary_text" => Ok(Self::SummaryText {
                text: object.take_required("text")?,
            }),
            other => Err(json_wrong_type(format!(
                "unknown reasoning summary `{other}`"
            ))),
        }
    }
}

impl ToJson for ReasoningItemContent {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        match self {
            Self::ReasoningText { text } => {
                object.push_field("type", "reasoning_text");
                object.push_field("text", text.clone());
            }
            Self::Text { text } => {
                object.push_field("type", "text");
                object.push_field("text", text.clone());
            }
        }
        object.into()
    }
}

impl FromJson for ReasoningItemContent {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let (kind, mut object) = json_type(
            value.into_object("ReasoningItemContent")?,
            "ReasoningItemContent",
        )?;
        match kind.as_str() {
            "reasoning_text" => Ok(Self::ReasoningText {
                text: object.take_required("text")?,
            }),
            "text" => Ok(Self::Text {
                text: object.take_required("text")?,
            }),
            other => Err(json_wrong_type(format!(
                "unknown reasoning content `{other}`"
            ))),
        }
    }
}

impl ToJson for FunctionCallOutputContentItem {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        match self {
            Self::InputText { text } => {
                object.push_field("type", "input_text");
                object.push_field("text", text.clone());
            }
            Self::InputImage { image_url, detail } => {
                object.push_field("type", "input_image");
                object.push_field("image_url", image_url.clone());
                object.push_opt_field("detail", detail.as_ref().map(ToJson::to_json));
            }
        }
        object.into()
    }
}

impl FromJson for FunctionCallOutputContentItem {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let (kind, mut object) = json_type(
            value.into_object("FunctionCallOutputContentItem")?,
            "FunctionCallOutputContentItem",
        )?;
        match kind.as_str() {
            "input_text" => Ok(Self::InputText {
                text: object.take_required("text")?,
            }),
            "input_image" => Ok(Self::InputImage {
                image_url: object.take_required("image_url")?,
                detail: object.take_optional("detail")?,
            }),
            other => Err(json_wrong_type(format!(
                "unknown function output content item `{other}`"
            ))),
        }
    }
}

impl ToJson for FunctionCallOutputBody {
    fn to_json(&self) -> Value {
        match self {
            Self::Text(text) => text.to_json(),
            Self::ContentItems(items) => items.to_json(),
        }
    }
}

impl FromJson for FunctionCallOutputBody {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        match value {
            Value::String(text) => Ok(Self::Text(text)),
            Value::Array(items) => items
                .into_iter()
                .map(FunctionCallOutputContentItem::from_json)
                .collect::<Result<Vec<_>, _>>()
                .map(Self::ContentItems),
            other => Err(json_wrong_type(format!(
                "expected string or content item array, found {}",
                other.variant_name()
            ))),
        }
    }
}

impl ToJson for FunctionCallOutputPayload {
    fn to_json(&self) -> Value {
        self.body.to_json()
    }
}

impl FromJson for FunctionCallOutputPayload {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        Ok(Self {
            body: FunctionCallOutputBody::from_json(value)?,
            success: None,
        })
    }
}

impl ToJson for ResponseItem {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        match self {
            Self::Message {
                role,
                content,
                phase,
                ..
            } => {
                object.push_field("type", "message");
                object.push_field("role", role.clone());
                object.push_field("content", content.to_json());
                object.push_opt_field("phase", phase.as_ref().map(ToJson::to_json));
            }
            Self::Reasoning {
                summary,
                content,
                encrypted_content,
                ..
            } => {
                object.push_field("type", "reasoning");
                object.push_field("summary", summary.to_json());
                object.push_opt_field("content", content.as_ref().map(ToJson::to_json));
                object.push_field("encrypted_content", encrypted_content.to_json());
            }
            Self::LocalShellCall {
                call_id,
                status,
                action,
                ..
            } => {
                object.push_field("type", "local_shell_call");
                object.push_field("call_id", call_id.to_json());
                object.push_field("status", status.to_json());
                object.push_field("action", action.to_json());
            }
            Self::FunctionCall {
                name,
                namespace,
                arguments,
                call_id,
                ..
            } => {
                object.push_field("type", "function_call");
                object.push_field("name", name.clone());
                object.push_opt_field("namespace", namespace.clone());
                object.push_field("arguments", arguments.clone());
                object.push_field("call_id", call_id.clone());
            }
            Self::ToolSearchCall {
                call_id,
                status,
                execution,
                arguments,
                ..
            } => {
                object.push_field("type", "tool_search_call");
                object.push_field("call_id", call_id.to_json());
                object.push_opt_field("status", status.clone());
                object.push_field("execution", execution.clone());
                object.push_field("arguments", arguments.clone());
            }
            Self::FunctionCallOutput { call_id, output } => {
                object.push_field("type", "function_call_output");
                object.push_field("call_id", call_id.clone());
                object.push_field("output", output.to_json());
            }
            Self::CustomToolCall {
                status,
                call_id,
                name,
                input,
                ..
            } => {
                object.push_field("type", "custom_tool_call");
                object.push_opt_field("status", status.clone());
                object.push_field("call_id", call_id.clone());
                object.push_field("name", name.clone());
                object.push_field("input", input.clone());
            }
            Self::CustomToolCallOutput {
                call_id,
                name,
                output,
            } => {
                object.push_field("type", "custom_tool_call_output");
                object.push_field("call_id", call_id.clone());
                object.push_opt_field("name", name.clone());
                object.push_field("output", output.to_json());
            }
            Self::ToolSearchOutput {
                call_id,
                status,
                execution,
                tools,
            } => {
                object.push_field("type", "tool_search_output");
                object.push_field("call_id", call_id.to_json());
                object.push_field("status", status.clone());
                object.push_field("execution", execution.clone());
                object.push_field("tools", tools.to_json());
            }
            Self::WebSearchCall { status, action, .. } => {
                object.push_field("type", "web_search_call");
                object.push_opt_field("status", status.clone());
                object.push_opt_field("action", action.as_ref().map(ToJson::to_json));
            }
            Self::ImageGenerationCall {
                id,
                status,
                revised_prompt,
                result,
            } => {
                object.push_field("type", "image_generation_call");
                object.push_field("id", id.clone());
                object.push_field("status", status.clone());
                object.push_opt_field("revised_prompt", revised_prompt.clone());
                object.push_field("result", result.clone());
            }
            Self::Compaction { encrypted_content } => {
                object.push_field("type", "compaction");
                object.push_field("encrypted_content", encrypted_content.clone());
            }
            Self::ContextCompaction { encrypted_content } => {
                object.push_field("type", "context_compaction");
                object.push_field("encrypted_content", encrypted_content.to_json());
            }
            Self::Other => {
                object.push_field("type", "other");
            }
        }
        object.into()
    }
}

impl FromJson for ResponseItem {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let (kind, mut object) = json_type(value.into_object("ResponseItem")?, "ResponseItem")?;
        match kind.as_str() {
            "message" => Ok(Self::Message {
                id: object.take_optional("id")?,
                role: object.take_required("role")?,
                content: object.take_required("content")?,
                phase: object.take_optional("phase")?,
            }),
            "reasoning" => Ok(Self::Reasoning {
                id: object.take_optional("id")?.unwrap_or_default(),
                summary: object.take_optional("summary")?.unwrap_or_default(),
                content: object.take_optional("content")?,
                encrypted_content: object.take_optional("encrypted_content")?,
            }),
            "local_shell_call" => Ok(Self::LocalShellCall {
                id: object.take_optional("id")?,
                call_id: object.take_optional("call_id")?,
                status: object.take_required("status")?,
                action: object.take_required("action")?,
            }),
            "function_call" => Ok(Self::FunctionCall {
                id: object.take_optional("id")?,
                name: object.take_required("name")?,
                namespace: object.take_optional("namespace")?,
                arguments: object.take_required("arguments")?,
                call_id: object.take_required("call_id")?,
            }),
            "tool_search_call" => Ok(Self::ToolSearchCall {
                id: object.take_optional("id")?,
                call_id: object.take_optional("call_id")?,
                status: object.take_optional("status")?,
                execution: object.take_required("execution")?,
                arguments: object.take_required("arguments")?,
            }),
            "function_call_output" => Ok(Self::FunctionCallOutput {
                call_id: object.take_required("call_id")?,
                output: object.take_required("output")?,
            }),
            "custom_tool_call" => Ok(Self::CustomToolCall {
                id: object.take_optional("id")?,
                status: object.take_optional("status")?,
                call_id: object.take_required("call_id")?,
                name: object.take_required("name")?,
                input: object.take_required("input")?,
            }),
            "custom_tool_call_output" => Ok(Self::CustomToolCallOutput {
                call_id: object.take_required("call_id")?,
                name: object.take_optional("name")?,
                output: object.take_required("output")?,
            }),
            "tool_search_output" => Ok(Self::ToolSearchOutput {
                call_id: object.take_optional("call_id")?,
                status: object.take_required("status")?,
                execution: object.take_required("execution")?,
                tools: object.take_required("tools")?,
            }),
            "web_search_call" => Ok(Self::WebSearchCall {
                id: object.take_optional("id")?,
                status: object.take_optional("status")?,
                action: object.take_optional("action")?,
            }),
            "image_generation_call" => Ok(Self::ImageGenerationCall {
                id: object.take_required("id")?,
                status: object.take_required("status")?,
                revised_prompt: object.take_optional("revised_prompt")?,
                result: object.take_required("result")?,
            }),
            "compaction" | "compaction_summary" => Ok(Self::Compaction {
                encrypted_content: object.take_required("encrypted_content")?,
            }),
            "context_compaction" => Ok(Self::ContextCompaction {
                encrypted_content: object.take_optional("encrypted_content")?,
            }),
            _ => Ok(Self::Other),
        }
    }
}

pub const BASE_INSTRUCTIONS_DEFAULT: &str = include_str!("prompts/base_instructions/default.md");

/// Base instructions for the model in a thread. Corresponds to the `instructions` field in the ResponsesAPI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
#[serde(rename = "base_instructions", rename_all = "snake_case")]
pub struct BaseInstructions {
    pub text: String,
}

impl Default for BaseInstructions {
    fn default() -> Self {
        Self {
            text: BASE_INSTRUCTIONS_DEFAULT.to_string(),
        }
    }
}

const MAX_RENDERED_PREFIXES: usize = 100;
const MAX_ALLOW_PREFIX_TEXT_BYTES: usize = 5000;
const TRUNCATED_MARKER: &str = "...\n[Some commands were truncated]";

pub fn format_allow_prefixes(prefixes: Vec<Vec<String>>) -> Option<String> {
    let mut truncated = false;
    if prefixes.len() > MAX_RENDERED_PREFIXES {
        truncated = true;
    }

    let mut prefixes = prefixes;
    prefixes.sort_by(|a, b| {
        a.len()
            .cmp(&b.len())
            .then_with(|| prefix_combined_str_len(a).cmp(&prefix_combined_str_len(b)))
            .then_with(|| a.cmp(b))
    });

    let full_text = prefixes
        .into_iter()
        .take(MAX_RENDERED_PREFIXES)
        .map(|prefix| format!("- {}", render_command_prefix(&prefix)))
        .collect::<Vec<_>>()
        .join("\n");

    // truncate to last UTF8 char
    let mut output = full_text;
    let byte_idx = output
        .char_indices()
        .nth(MAX_ALLOW_PREFIX_TEXT_BYTES)
        .map(|(i, _)| i);
    if let Some(byte_idx) = byte_idx {
        truncated = true;
        output = output[..byte_idx].to_string();
    }

    if truncated {
        Some(format!("{output}{TRUNCATED_MARKER}"))
    } else {
        Some(output)
    }
}

fn prefix_combined_str_len(prefix: &[String]) -> usize {
    prefix.iter().map(String::len).sum()
}

fn render_command_prefix(prefix: &[String]) -> String {
    let tokens = prefix
        .iter()
        .map(|token| edgerun_json::to_string(token).unwrap_or_else(|_| format!("{token:?}")))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{tokens}]")
}

fn should_serialize_reasoning_content(content: &Option<Vec<ReasoningItemContent>>) -> bool {
    match content {
        Some(content) => !content
            .iter()
            .any(|c| matches!(c, ReasoningItemContent::ReasoningText { .. })),
        None => false,
    }
}

fn local_image_error_placeholder(
    path: &std::path::Path,
    error: impl std::fmt::Display,
) -> ContentItem {
    ContentItem::InputText {
        text: format!(
            "Codex could not read the local image at `{}`: {}",
            path.display(),
            error
        ),
    }
}

pub const VIEW_IMAGE_TOOL_NAME: &str = "view_image";

const IMAGE_OPEN_TAG: &str = "<image>";
const IMAGE_CLOSE_TAG: &str = "</image>";
const LOCAL_IMAGE_OPEN_TAG_PREFIX: &str = "<image name=";
const LOCAL_IMAGE_OPEN_TAG_SUFFIX: &str = ">";
const LOCAL_IMAGE_CLOSE_TAG: &str = IMAGE_CLOSE_TAG;

pub fn image_open_tag_text() -> String {
    IMAGE_OPEN_TAG.to_string()
}

pub fn image_close_tag_text() -> String {
    IMAGE_CLOSE_TAG.to_string()
}

pub fn local_image_label_text(label_number: usize) -> String {
    format!("[Image #{label_number}]")
}

pub fn local_image_open_tag_text(label_number: usize) -> String {
    let label = local_image_label_text(label_number);
    format!("{LOCAL_IMAGE_OPEN_TAG_PREFIX}{label}{LOCAL_IMAGE_OPEN_TAG_SUFFIX}")
}

pub fn is_local_image_open_tag_text(text: &str) -> bool {
    text.strip_prefix(LOCAL_IMAGE_OPEN_TAG_PREFIX)
        .is_some_and(|rest| rest.ends_with(LOCAL_IMAGE_OPEN_TAG_SUFFIX))
}

pub fn is_local_image_close_tag_text(text: &str) -> bool {
    is_image_close_tag_text(text)
}

pub fn is_image_open_tag_text(text: &str) -> bool {
    text == IMAGE_OPEN_TAG
}

pub fn is_image_close_tag_text(text: &str) -> bool {
    text == IMAGE_CLOSE_TAG
}

fn invalid_image_error_placeholder(
    path: &std::path::Path,
    error: impl std::fmt::Display,
) -> ContentItem {
    ContentItem::InputText {
        text: format!(
            "Image located at `{}` is invalid: {}",
            path.display(),
            error
        ),
    }
}

fn unsupported_image_error_placeholder(path: &std::path::Path, mime: &str) -> ContentItem {
    ContentItem::InputText {
        text: format!(
            "Codex cannot attach image at `{}`: unsupported image `{}`.",
            path.display(),
            mime
        ),
    }
}

pub fn local_image_content_items_with_label_number(
    path: &std::path::Path,
    file_bytes: Vec<u8>,
    label_number: Option<usize>,
    mode: PromptImageMode,
) -> Vec<ContentItem> {
    match load_for_prompt_bytes(path, file_bytes, mode) {
        Ok(image) => {
            let mut items = Vec::with_capacity(3);
            if let Some(label_number) = label_number {
                items.push(ContentItem::InputText {
                    text: local_image_open_tag_text(label_number),
                });
            }
            items.push(ContentItem::InputImage {
                image_url: image.into_data_url(),
                detail: Some(DEFAULT_IMAGE_DETAIL),
            });
            if label_number.is_some() {
                items.push(ContentItem::InputText {
                    text: LOCAL_IMAGE_CLOSE_TAG.to_string(),
                });
            }
            items
        }
        Err(err) => match &err {
            ImageProcessingError::Read { .. } | ImageProcessingError::Encode { .. } => {
                vec![local_image_error_placeholder(path, &err)]
            }
            ImageProcessingError::Decode { .. } if err.is_invalid_image() => {
                vec![invalid_image_error_placeholder(path, &err)]
            }
            ImageProcessingError::Decode { .. } => {
                vec![local_image_error_placeholder(path, &err)]
            }
            ImageProcessingError::UnsupportedImageFormat { mime } => {
                vec![unsupported_image_error_placeholder(path, mime)]
            }
        },
    }
}

impl From<ResponseInputItem> for ResponseItem {
    fn from(item: ResponseInputItem) -> Self {
        match item {
            ResponseInputItem::Message {
                role,
                content,
                phase,
            } => Self::Message {
                role,
                content,
                id: None,
                phase,
            },
            ResponseInputItem::FunctionCallOutput { call_id, output } => {
                Self::FunctionCallOutput { call_id, output }
            }
            ResponseInputItem::McpToolCallOutput { call_id, output } => {
                let output = output.into_function_call_output_payload();
                Self::FunctionCallOutput { call_id, output }
            }
            ResponseInputItem::CustomToolCallOutput {
                call_id,
                name,
                output,
            } => Self::CustomToolCallOutput {
                call_id,
                name,
                output,
            },
            ResponseInputItem::ToolSearchOutput {
                call_id,
                status,
                execution,
                tools,
            } => Self::ToolSearchOutput {
                call_id: Some(call_id),
                status,
                execution,
                tools,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum LocalShellStatus {
    Completed,
    InProgress,
    Incomplete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LocalShellAction {
    Exec(LocalShellExecAction),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
pub struct LocalShellExecAction {
    pub command: Vec<String>,
    pub timeout_ms: Option<u64>,
    pub working_directory: Option<String>,
    pub env: Option<BTreeMap<String, String>>,
    pub user: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "ResponsesApiWebSearchAction")]
pub enum WebSearchAction {
    Search {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        query: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        queries: Option<Vec<String>>,
    },
    OpenPage {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        url: Option<String>,
    },
    FindInPage {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        url: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        pattern: Option<String>,
    },

    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReasoningItemReasoningSummary {
    SummaryText { text: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReasoningItemContent {
    ReasoningText { text: String },
    Text { text: String },
}

impl From<Vec<UserInput>> for ResponseInputItem {
    fn from(items: Vec<UserInput>) -> Self {
        let mut image_index = 0;
        Self::Message {
            role: "user".to_string(),
            content: items
                .into_iter()
                .flat_map(|c| match c {
                    UserInput::Text { text, .. } => vec![ContentItem::InputText { text }],
                    UserInput::Image { image_url } => {
                        image_index += 1;
                        vec![
                            ContentItem::InputText {
                                text: image_open_tag_text(),
                            },
                            ContentItem::InputImage {
                                image_url,
                                detail: Some(DEFAULT_IMAGE_DETAIL),
                            },
                            ContentItem::InputText {
                                text: image_close_tag_text(),
                            },
                        ]
                    }
                    UserInput::LocalImage { path } => {
                        image_index += 1;
                        match std::fs::read(&path) {
                            Ok(file_bytes) => local_image_content_items_with_label_number(
                                &path,
                                file_bytes,
                                Some(image_index),
                                PromptImageMode::ResizeToFit,
                            ),
                            Err(err) => vec![local_image_error_placeholder(&path, err)],
                        }
                    }
                    UserInput::Skill { .. } | UserInput::Mention { .. } => Vec::new(), // Tool bodies are injected later in core
                })
                .collect::<Vec<ContentItem>>(),
            phase: None,
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
pub struct SearchToolCallParams {
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub limit: Option<usize>,
}

/// If the `name` of a `ResponseItem::FunctionCall` is either `container.exec`
/// or `shell`, the `arguments` field should deserialize to this struct.
#[derive(Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
pub struct ShellToolCallParams {
    pub command: Vec<String>,
    pub workdir: Option<String>,

    /// This is the maximum time in milliseconds that the command is allowed to run.
    #[serde(alias = "timeout")]
    pub timeout_ms: Option<u64>,
    /// Suggests a command prefix to persist for future sessions
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub prefix_rule: Option<Vec<String>>,
}

impl FromJson for ShellToolCallParams {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("ShellToolCallParams")?;
        let timeout_ms = match object.remove("timeout_ms") {
            Some(Value::Null) | None => object.take_optional("timeout")?,
            Some(value) => Some(u64::from_json(value)?),
        };
        Ok(Self {
            command: object.take_required("command")?,
            workdir: object.take_optional("workdir")?,
            timeout_ms,
            prefix_rule: object.take_optional("prefix_rule")?,
        })
    }
}

/// If the `name` of a `ResponseItem::FunctionCall` is `shell_command`, the
/// `arguments` field should deserialize to this struct.
#[derive(Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
pub struct ShellCommandToolCallParams {
    pub command: String,
    pub workdir: Option<String>,

    /// Whether to run the shell with login shell semantics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login: Option<bool>,
    /// This is the maximum time in milliseconds that the command is allowed to run.
    #[serde(alias = "timeout")]
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub prefix_rule: Option<Vec<String>>,
}

impl FromJson for ShellCommandToolCallParams {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("ShellCommandToolCallParams")?;
        let timeout_ms = match object.remove("timeout_ms") {
            Some(Value::Null) | None => object.take_optional("timeout")?,
            Some(value) => Some(u64::from_json(value)?),
        };
        Ok(Self {
            command: object.take_required("command")?,
            workdir: object.take_optional("workdir")?,
            login: object.take_optional("login")?,
            timeout_ms,
            prefix_rule: object.take_optional("prefix_rule")?,
        })
    }
}

/// Responses API compatible content items that can be returned by a tool call.
/// This is a subset of ContentItem with the types we support as function call outputs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FunctionCallOutputContentItem {
    // Do not rename, these are serialized and used directly in the responses API.
    InputText {
        text: String,
    },
    // Do not rename, these are serialized and used directly in the responses API.
    InputImage {
        image_url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        detail: Option<ImageDetail>,
    },
}

/// Converts structured function-call output content into plain text for
/// human-readable surfaces.
///
/// This conversion is intentionally lossy:
/// - only `input_text` items are included
/// - image items are ignored
///
/// We use this helper where callers still need a string representation (for
/// example telemetry previews or legacy string-only output paths) while keeping
/// the original multimodal `content_items` as the authoritative payload sent to
/// the model.
pub fn function_call_output_content_items_to_text(
    content_items: &[FunctionCallOutputContentItem],
) -> Option<String> {
    let text_segments = content_items
        .iter()
        .filter_map(|item| match item {
            FunctionCallOutputContentItem::InputText { text } if !text.trim().is_empty() => {
                Some(text.as_str())
            }
            FunctionCallOutputContentItem::InputText { .. }
            | FunctionCallOutputContentItem::InputImage { .. } => None,
        })
        .collect::<Vec<_>>();

    if text_segments.is_empty() {
        None
    } else {
        Some(text_segments.join("\n"))
    }
}

impl From<crate::dynamic_tools::DynamicToolCallOutputContentItem>
    for FunctionCallOutputContentItem
{
    fn from(item: crate::dynamic_tools::DynamicToolCallOutputContentItem) -> Self {
        match item {
            crate::dynamic_tools::DynamicToolCallOutputContentItem::InputText { text } => {
                Self::InputText { text }
            }
            crate::dynamic_tools::DynamicToolCallOutputContentItem::InputImage { image_url } => {
                Self::InputImage {
                    image_url,
                    detail: Some(DEFAULT_IMAGE_DETAIL),
                }
            }
        }
    }
}

/// The payload we send back to OpenAI when reporting a tool call result.
///
/// `body` serializes directly as the wire value for `function_call_output.output`.
/// `success` remains internal metadata for downstream handling.
#[derive(Debug, Default, Clone, PartialEq, JsonSchema, TS)]
pub struct FunctionCallOutputPayload {
    pub body: FunctionCallOutputBody,
    pub success: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, TS)]
#[serde(untagged)]
pub enum FunctionCallOutputBody {
    Text(String),
    ContentItems(Vec<FunctionCallOutputContentItem>),
}

impl FunctionCallOutputBody {
    /// Best-effort conversion of a function-call output body to plain text for
    /// human-readable surfaces.
    ///
    /// This conversion is intentionally lossy when the body contains content
    /// items: image entries are dropped and text entries are joined with
    /// newlines.
    pub fn to_text(&self) -> Option<String> {
        match self {
            Self::Text(content) => Some(content.clone()),
            Self::ContentItems(items) => function_call_output_content_items_to_text(items),
        }
    }
}

impl Default for FunctionCallOutputBody {
    fn default() -> Self {
        Self::Text(String::new())
    }
}

impl FunctionCallOutputPayload {
    pub fn from_text(content: String) -> Self {
        Self {
            body: FunctionCallOutputBody::Text(content),
            success: None,
        }
    }

    pub fn from_content_items(content_items: Vec<FunctionCallOutputContentItem>) -> Self {
        Self {
            body: FunctionCallOutputBody::ContentItems(content_items),
            success: None,
        }
    }

    pub fn text_content(&self) -> Option<&str> {
        match &self.body {
            FunctionCallOutputBody::Text(content) => Some(content),
            FunctionCallOutputBody::ContentItems(_) => None,
        }
    }

    pub fn text_content_mut(&mut self) -> Option<&mut String> {
        match &mut self.body {
            FunctionCallOutputBody::Text(content) => Some(content),
            FunctionCallOutputBody::ContentItems(_) => None,
        }
    }

    pub fn content_items(&self) -> Option<&[FunctionCallOutputContentItem]> {
        match &self.body {
            FunctionCallOutputBody::Text(_) => None,
            FunctionCallOutputBody::ContentItems(items) => Some(items),
        }
    }

    pub fn content_items_mut(&mut self) -> Option<&mut Vec<FunctionCallOutputContentItem>> {
        match &mut self.body {
            FunctionCallOutputBody::Text(_) => None,
            FunctionCallOutputBody::ContentItems(items) => Some(items),
        }
    }
}

// `function_call_output.output` is encoded as either:
//   - an array of structured content items
//   - a plain string
impl Serialize for FunctionCallOutputPayload {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.body {
            FunctionCallOutputBody::Text(content) => serializer.serialize_str(content),
            FunctionCallOutputBody::ContentItems(items) => items.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for FunctionCallOutputPayload {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let body = FunctionCallOutputBody::deserialize(deserializer)?;
        Ok(FunctionCallOutputPayload {
            body,
            success: None,
        })
    }
}

impl CallToolResult {
    pub fn from_result(result: Result<Self, String>) -> Self {
        match result {
            Ok(result) => result,
            Err(error) => Self::from_error_text(error),
        }
    }

    pub fn from_error_text(text: String) -> Self {
        Self {
            content: vec![edgerun_json::json!({
                "type": "text",
                "text": text,
            })],
            structured_content: None,
            is_error: Some(true),
            meta: None,
        }
    }

    pub fn success(&self) -> bool {
        self.is_error != Some(true)
    }

    pub fn as_function_call_output_payload(&self) -> FunctionCallOutputPayload {
        if let Some(structured_content) = &self.structured_content
            && !structured_content.is_null()
        {
            match edgerun_json::to_string(structured_content) {
                Ok(serialized_structured_content) => {
                    return FunctionCallOutputPayload {
                        body: FunctionCallOutputBody::Text(serialized_structured_content),
                        success: Some(self.success()),
                    };
                }
                Err(err) => {
                    return FunctionCallOutputPayload {
                        body: FunctionCallOutputBody::Text(err.to_string()),
                        success: Some(false),
                    };
                }
            }
        }

        let serialized_content = match edgerun_json::to_string(&self.content) {
            Ok(serialized_content) => serialized_content,
            Err(err) => {
                return FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(err.to_string()),
                    success: Some(false),
                };
            }
        };

        let content_items = convert_mcp_content_to_items(&self.content);

        let body = match content_items {
            Some(content_items) => FunctionCallOutputBody::ContentItems(content_items),
            None => FunctionCallOutputBody::Text(serialized_content),
        };

        FunctionCallOutputPayload {
            body,
            success: Some(self.success()),
        }
    }

    pub fn into_function_call_output_payload(self) -> FunctionCallOutputPayload {
        self.as_function_call_output_payload()
    }
}

fn convert_mcp_content_to_items(
    contents: &[edgerun_json::Value],
) -> Option<Vec<FunctionCallOutputContentItem>> {
    const CODEX_IMAGE_DETAIL_META_KEY: &str = "codex/imageDetail";

    let mut saw_image = false;
    let mut items = Vec::with_capacity(contents.len());

    for content in contents {
        let item = match content
            .clone()
            .into_object("McpContent")
            .and_then(|mut object| {
                let kind: String = object.take_required("type")?;
                Ok((kind, object))
            }) {
            Ok((kind, mut object)) if kind == "text" => {
                let text = object
                    .take_required("text")
                    .unwrap_or_else(|_| edgerun_json::to_string(content).unwrap_or_default());
                FunctionCallOutputContentItem::InputText { text }
            }
            Ok((kind, mut object)) if kind == "image" => {
                saw_image = true;
                let data: String = object.take_required("data").unwrap_or_default();
                let mime_type: Option<String> = object
                    .take_optional_any(&["mimeType", "mime_type"])
                    .unwrap_or(None);
                let meta: Option<edgerun_json::Value> =
                    object.take_optional("_meta").unwrap_or(None);
                let image_url = if data.starts_with("data:") {
                    data
                } else {
                    let mime_type = mime_type.unwrap_or_else(|| "application/octet-stream".into());
                    format!("data:{mime_type};base64,{data}")
                };
                FunctionCallOutputContentItem::InputImage {
                    image_url,
                    detail: meta
                        .as_ref()
                        .and_then(edgerun_json::Value::as_object)
                        .and_then(|meta| meta.get(CODEX_IMAGE_DETAIL_META_KEY))
                        .and_then(edgerun_json::Value::as_str)
                        .and_then(|detail| match detail {
                            "auto" => Some(ImageDetail::Auto),
                            "low" => Some(ImageDetail::Low),
                            "high" => Some(ImageDetail::High),
                            "original" => Some(ImageDetail::Original),
                            _ => None,
                        })
                        .or(Some(DEFAULT_IMAGE_DETAIL)),
                }
            }
            _ => FunctionCallOutputContentItem::InputText {
                text: edgerun_json::to_string(content).unwrap_or_else(|_| "<content>".to_string()),
            },
        };
        items.push(item);
    }

    if saw_image { Some(items) } else { None }
}

// Implement Display so callers can treat the payload like a plain string when logging or doing
// trivial substring checks in tests (existing tests call `.contains()` on the output). For
// `ContentItems`, Display emits a JSON representation.

impl std::fmt::Display for FunctionCallOutputPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.body {
            FunctionCallOutputBody::Text(content) => f.write_str(content),
            FunctionCallOutputBody::ContentItems(items) => {
                let content = edgerun_json::to_string(items).unwrap_or_default();
                f.write_str(content.as_str())
            }
        }
    }
}

// (Moved event mapping logic into codex-core to avoid coupling protocol to UI-facing events.)

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use pretty_assertions::assert_eq;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn from_json_str<T: FromJson>(json: &str) -> T {
        edgerun_json::from_value(edgerun_json::from_str(json).expect("parse json"))
            .expect("deserialize json value")
    }

    #[test]
    fn response_input_message_conversion_preserves_phase() {
        let item = ResponseItem::from(ResponseInputItem::Message {
            role: "assistant".to_string(),
            content: vec![ContentItem::OutputText {
                text: "still working".to_string(),
            }],
            phase: Some(MessagePhase::Commentary),
        });

        assert_eq!(
            item,
            ResponseItem::Message {
                id: None,
                role: "assistant".to_string(),
                content: vec![ContentItem::OutputText {
                    text: "still working".to_string(),
                }],
                phase: Some(MessagePhase::Commentary),
            }
        );
    }

    #[test]
    fn convert_mcp_content_to_items_preserves_data_urls() {
        let contents = vec![edgerun_json::json!({
            "type": "image",
            "data": "data:image/png;base64,Zm9v",
            "mimeType": "image/png",
        })];

        let items = convert_mcp_content_to_items(&contents).expect("expected image items");
        assert_eq!(
            items,
            vec![FunctionCallOutputContentItem::InputImage {
                image_url: "data:image/png;base64,Zm9v".to_string(),
                detail: Some(DEFAULT_IMAGE_DETAIL),
            }]
        );
    }

    #[test]
    fn response_item_parses_image_generation_call() {
        let item = edgerun_json::from_value::<ResponseItem>(edgerun_json::json!({
            "id": "ig_123",
            "type": "image_generation_call",
            "status": "completed",
            "revised_prompt": "A small blue square",
            "result": "Zm9v",
        }))
        .expect("image generation item should deserialize");

        assert_eq!(
            item,
            ResponseItem::ImageGenerationCall {
                id: "ig_123".to_string(),
                status: "completed".to_string(),
                revised_prompt: Some("A small blue square".to_string()),
                result: "Zm9v".to_string(),
            }
        );
    }

    #[test]
    fn response_item_parses_image_generation_call_without_revised_prompt() {
        let item = edgerun_json::from_value::<ResponseItem>(edgerun_json::json!({
            "id": "ig_123",
            "type": "image_generation_call",
            "status": "completed",
            "result": "Zm9v",
        }))
        .expect("image generation item should deserialize");

        assert_eq!(
            item,
            ResponseItem::ImageGenerationCall {
                id: "ig_123".to_string(),
                status: "completed".to_string(),
                revised_prompt: None,
                result: "Zm9v".to_string(),
            }
        );
    }

    #[test]
    fn permission_profile_round_trip_preserves_glob_scan_max_depth() {
        let mut file_system_sandbox_policy =
            FileSystemSandboxPolicy::restricted(vec![FileSystemSandboxEntry {
                path: FileSystemPath::GlobPattern {
                    pattern: "**/*.env".to_string(),
                },
                access: FileSystemAccessMode::None,
            }]);
        file_system_sandbox_policy.glob_scan_max_depth = Some(2);

        let permission_profile = PermissionProfile::from_runtime_permissions(
            &file_system_sandbox_policy,
            NetworkSandboxPolicy::Restricted,
        );

        assert_eq!(
            permission_profile.file_system_sandbox_policy(),
            file_system_sandbox_policy
        );
    }

    #[test]
    fn permission_profile_deserializes_legacy_rollout_shape() -> Result<()> {
        let legacy = edgerun_json::json!({
            "network": {
                "enabled": true,
            },
            "file_system": {
                "entries": [{
                    "path": {
                        "type": "special",
                        "value": {
                            "kind": "root",
                        },
                    },
                    "access": "write",
                }],
                "glob_scan_max_depth": 2,
            },
        });

        let permission_profile: PermissionProfile = edgerun_json::from_serde_value(legacy)?;

        assert_eq!(
            permission_profile,
            PermissionProfile::Managed {
                file_system: ManagedFileSystemPermissions::Restricted {
                    entries: vec![FileSystemSandboxEntry {
                        path: FileSystemPath::Special {
                            value: FileSystemSpecialPath::Root,
                        },
                        access: FileSystemAccessMode::Write,
                    }],
                    glob_scan_max_depth: NonZeroUsize::new(2),
                },
                network: NetworkSandboxPolicy::Enabled,
            }
        );
        Ok(())
    }

    #[test]
    fn permission_profile_presets_match_legacy_defaults() {
        assert_eq!(
            PermissionProfile::read_only(),
            PermissionProfile::from_legacy_sandbox_policy(&SandboxPolicy::new_read_only_policy())
        );
        assert_eq!(
            PermissionProfile::workspace_write(),
            PermissionProfile::from_legacy_sandbox_policy(
                &SandboxPolicy::new_workspace_write_policy()
            )
        );
    }

    #[test]
    fn permission_profile_round_trip_preserves_disabled_sandbox() -> Result<()> {
        let cwd = tempdir()?;
        let permission_profile =
            PermissionProfile::from_legacy_sandbox_policy(&SandboxPolicy::DangerFullAccess);

        assert_eq!(permission_profile, PermissionProfile::Disabled);
        assert_eq!(
            permission_profile.to_legacy_sandbox_policy(cwd.path())?,
            SandboxPolicy::DangerFullAccess
        );
        assert_eq!(
            permission_profile.to_runtime_permissions(),
            (
                FileSystemSandboxPolicy::unrestricted(),
                NetworkSandboxPolicy::Enabled
            )
        );
        Ok(())
    }

    #[test]
    fn disabled_permission_profile_ignores_runtime_network_policy() {
        let permission_profile = PermissionProfile::from_runtime_permissions_with_enforcement(
            SandboxEnforcement::Disabled,
            &FileSystemSandboxPolicy::unrestricted(),
            NetworkSandboxPolicy::Restricted,
        );

        assert_eq!(permission_profile, PermissionProfile::Disabled);
    }

    #[test]
    fn permission_profile_from_runtime_permissions_preserves_external_sandbox() {
        let permission_profile = PermissionProfile::from_runtime_permissions(
            &FileSystemSandboxPolicy::external_sandbox(),
            NetworkSandboxPolicy::Restricted,
        );

        assert_eq!(
            permission_profile,
            PermissionProfile::External {
                network: NetworkSandboxPolicy::Restricted,
            }
        );
        assert_eq!(
            PermissionProfile::from_runtime_permissions_with_enforcement(
                SandboxEnforcement::Managed,
                &FileSystemSandboxPolicy::external_sandbox(),
                NetworkSandboxPolicy::Restricted,
            ),
            permission_profile,
        );
    }

    #[test]
    fn permission_profile_from_runtime_permissions_preserves_unrestricted_managed_network() {
        let permission_profile = PermissionProfile::from_runtime_permissions_with_enforcement(
            SandboxEnforcement::External,
            &FileSystemSandboxPolicy::unrestricted(),
            NetworkSandboxPolicy::Restricted,
        );

        assert_eq!(
            permission_profile,
            PermissionProfile::Managed {
                file_system: ManagedFileSystemPermissions::Unrestricted,
                network: NetworkSandboxPolicy::Restricted,
            },
            "the legacy ExternalSandbox projection must not hide a split unrestricted filesystem policy"
        );
        assert_eq!(
            permission_profile.to_runtime_permissions(),
            (
                FileSystemSandboxPolicy::unrestricted(),
                NetworkSandboxPolicy::Restricted,
            )
        );
    }

    #[test]
    fn permission_profile_round_trip_preserves_external_sandbox() -> Result<()> {
        let cwd = tempdir()?;
        let sandbox_policy = SandboxPolicy::ExternalSandbox {
            network_access: crate::protocol::NetworkAccess::Restricted,
        };
        let permission_profile = PermissionProfile::from_legacy_sandbox_policy(&sandbox_policy);

        assert_eq!(
            permission_profile,
            PermissionProfile::External {
                network: NetworkSandboxPolicy::Restricted,
            }
        );
        assert_eq!(
            permission_profile.to_legacy_sandbox_policy(cwd.path())?,
            sandbox_policy
        );
        assert_eq!(
            permission_profile.to_runtime_permissions(),
            (
                FileSystemSandboxPolicy::external_sandbox(),
                NetworkSandboxPolicy::Restricted
            )
        );
        Ok(())
    }

    #[test]
    fn file_system_permissions_with_glob_scan_depth_uses_canonical_json() -> Result<()> {
        let path = AbsolutePathBuf::try_from(PathBuf::from(if cfg!(windows) {
            r"C:\tmp\allowed"
        } else {
            "/tmp/allowed"
        }))
        .expect("absolute path");
        let file_system_permissions = FileSystemPermissions {
            entries: vec![FileSystemSandboxEntry {
                path: FileSystemPath::Path { path },
                access: FileSystemAccessMode::Read,
            }],
            glob_scan_max_depth: NonZeroUsize::new(2),
        };

        let serialized = edgerun_json::to_value(&file_system_permissions);

        assert_eq!(serialized.get("read"), None);
        assert_eq!(serialized.get("write"), None);
        assert_eq!(
            serialized.get("glob_scan_max_depth"),
            Some(&edgerun_json::json!(2))
        );
        assert!(serialized.get("entries").is_some());
        assert_eq!(
            edgerun_json::from_value::<FileSystemPermissions>(serialized)
                .expect("filesystem permissions"),
            file_system_permissions
        );
        Ok(())
    }

    #[test]
    fn file_system_permissions_rejects_zero_glob_scan_depth() {
        edgerun_json::from_value::<FileSystemPermissions>(edgerun_json::json!({
            "entries": [],
            "glob_scan_max_depth": 0,
        }))
        .expect_err("zero glob scan depth should fail deserialization");
    }

    #[test]
    fn convert_mcp_content_to_items_builds_data_urls_when_missing_prefix() {
        let contents = vec![edgerun_json::json!({
            "type": "image",
            "data": "Zm9v",
            "mimeType": "image/png",
        })];

        let items = convert_mcp_content_to_items(&contents).expect("expected image items");
        assert_eq!(
            items,
            vec![FunctionCallOutputContentItem::InputImage {
                image_url: "data:image/png;base64,Zm9v".to_string(),
                detail: Some(DEFAULT_IMAGE_DETAIL),
            }]
        );
    }

    #[test]
    fn convert_mcp_content_to_items_returns_none_without_images() {
        let contents = vec![edgerun_json::json!({
            "type": "text",
            "text": "hello",
        })];

        assert_eq!(convert_mcp_content_to_items(&contents), None);
    }

    #[test]
    fn function_call_output_content_items_to_text_joins_text_segments() {
        let content_items = vec![
            FunctionCallOutputContentItem::InputText {
                text: "line 1".to_string(),
            },
            FunctionCallOutputContentItem::InputImage {
                image_url: "data:image/png;base64,AAA".to_string(),
                detail: Some(DEFAULT_IMAGE_DETAIL),
            },
            FunctionCallOutputContentItem::InputText {
                text: "line 2".to_string(),
            },
        ];

        let text = function_call_output_content_items_to_text(&content_items);
        assert_eq!(text, Some("line 1\nline 2".to_string()));
    }

    #[test]
    fn function_call_output_content_items_to_text_ignores_blank_text_and_images() {
        let content_items = vec![
            FunctionCallOutputContentItem::InputText {
                text: "   ".to_string(),
            },
            FunctionCallOutputContentItem::InputImage {
                image_url: "data:image/png;base64,AAA".to_string(),
                detail: Some(DEFAULT_IMAGE_DETAIL),
            },
        ];

        let text = function_call_output_content_items_to_text(&content_items);
        assert_eq!(text, None);
    }

    #[test]
    fn function_call_output_body_to_text_returns_plain_text_content() {
        let body = FunctionCallOutputBody::Text("ok".to_string());
        let text = body.to_text();
        assert_eq!(text, Some("ok".to_string()));
    }

    #[test]
    fn function_call_output_body_to_text_uses_content_item_fallback() {
        let body = FunctionCallOutputBody::ContentItems(vec![
            FunctionCallOutputContentItem::InputText {
                text: "line 1".to_string(),
            },
            FunctionCallOutputContentItem::InputImage {
                image_url: "data:image/png;base64,AAA".to_string(),
                detail: Some(DEFAULT_IMAGE_DETAIL),
            },
        ]);

        let text = body.to_text();
        assert_eq!(text, Some("line 1".to_string()));
    }

    #[test]
    fn function_call_deserializes_optional_namespace() {
        let item: ResponseItem = edgerun_json::from_serde_value(edgerun_json::json!({
            "type": "function_call",
            "name": "mcp__codex_apps__gmail_get_recent_emails",
            "namespace": "mcp__codex_apps__gmail",
            "arguments": "{\"top_k\":5}",
            "call_id": "call-1",
        }))
        .expect("function_call should deserialize");

        assert_eq!(
            item,
            ResponseItem::FunctionCall {
                id: None,
                name: "mcp__codex_apps__gmail_get_recent_emails".to_string(),
                namespace: Some("mcp__codex_apps__gmail".to_string()),
                arguments: "{\"top_k\":5}".to_string(),
                call_id: "call-1".to_string(),
            }
        );
    }

    #[test]
    fn render_command_prefix_list_sorts_by_len_then_total_len_then_alphabetical() {
        let prefixes = vec![
            vec!["b".to_string(), "zz".to_string()],
            vec!["aa".to_string()],
            vec!["b".to_string()],
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            vec!["a".to_string()],
            vec!["b".to_string(), "a".to_string()],
        ];

        let output = format_allow_prefixes(prefixes).expect("rendered list");
        assert_eq!(
            output,
            r#"- ["a"]
- ["b"]
- ["aa"]
- ["b", "a"]
- ["b", "zz"]
- ["a", "b", "c"]"#
                .to_string(),
        );
    }

    #[test]
    fn render_command_prefix_list_limits_output_to_max_prefixes() {
        let prefixes = (0..(MAX_RENDERED_PREFIXES + 5))
            .map(|i| vec![format!("{i:03}")])
            .collect::<Vec<_>>();

        let output = format_allow_prefixes(prefixes).expect("rendered list");
        assert_eq!(output.ends_with(TRUNCATED_MARKER), true);
        eprintln!("output: {output}");
        assert_eq!(output.lines().count(), MAX_RENDERED_PREFIXES + 1);
    }

    #[test]
    fn format_allow_prefixes_limits_output() {
        let prefixes = (0..200)
            .map(|i| vec![format!("tool-{i:03}"), "x".repeat(500)])
            .collect();

        let output = format_allow_prefixes(prefixes).expect("formatted prefixes");
        assert!(
            output.len() <= MAX_ALLOW_PREFIX_TEXT_BYTES + TRUNCATED_MARKER.len(),
            "output length exceeds expected limit: {output}",
        );
    }

    #[test]
    fn serializes_success_as_plain_string() -> Result<()> {
        let item = ResponseInputItem::FunctionCallOutput {
            call_id: "call1".into(),
            output: FunctionCallOutputPayload::from_text("ok".into()),
        };

        let json = edgerun_json::to_string(&item)?;
        let v: edgerun_json::Value = edgerun_json::from_str(&json)?;

        // Success case -> output should be a plain string
        assert_eq!(v.get("output").unwrap().as_str().unwrap(), "ok");
        Ok(())
    }

    #[test]
    fn serializes_failure_as_string() -> Result<()> {
        let item = ResponseInputItem::FunctionCallOutput {
            call_id: "call1".into(),
            output: FunctionCallOutputPayload {
                body: FunctionCallOutputBody::Text("bad".into()),
                success: Some(false),
            },
        };

        let json = edgerun_json::to_string(&item)?;
        let v: edgerun_json::Value = edgerun_json::from_str(&json)?;

        assert_eq!(v.get("output").unwrap().as_str().unwrap(), "bad");
        Ok(())
    }

    #[test]
    fn serializes_image_outputs_as_array() -> Result<()> {
        let call_tool_result = CallToolResult {
            content: vec![
                edgerun_json::json!({"type":"text","text":"caption"}),
                edgerun_json::json!({"type":"image","data":"BASE64","mimeType":"image/png"}),
            ],
            structured_content: None,
            is_error: Some(false),
            meta: None,
        };

        let payload = call_tool_result.into_function_call_output_payload();
        assert_eq!(payload.success, Some(true));
        let Some(items) = payload.content_items() else {
            panic!("expected content items");
        };
        let items = items.to_vec();
        assert_eq!(
            items,
            vec![
                FunctionCallOutputContentItem::InputText {
                    text: "caption".into(),
                },
                FunctionCallOutputContentItem::InputImage {
                    image_url: "data:image/png;base64,BASE64".into(),
                    detail: Some(DEFAULT_IMAGE_DETAIL),
                },
            ]
        );

        let item = ResponseInputItem::FunctionCallOutput {
            call_id: "call1".into(),
            output: payload,
        };

        let json = edgerun_json::to_string(&item)?;
        let v: edgerun_json::Value = edgerun_json::from_str(&json)?;

        let output = v.get("output").expect("output field");
        assert!(output.is_array(), "expected array output");

        Ok(())
    }

    #[test]
    fn serializes_custom_tool_image_outputs_as_array() -> Result<()> {
        let item = ResponseInputItem::CustomToolCallOutput {
            call_id: "call1".into(),
            name: None,
            output: FunctionCallOutputPayload::from_content_items(vec![
                FunctionCallOutputContentItem::InputImage {
                    image_url: "data:image/png;base64,BASE64".into(),
                    detail: Some(DEFAULT_IMAGE_DETAIL),
                },
            ]),
        };

        let json = edgerun_json::to_string(&item)?;
        let v: edgerun_json::Value = edgerun_json::from_str(&json)?;

        let output = v.get("output").expect("output field");
        assert!(output.is_array(), "expected array output");

        Ok(())
    }

    #[test]
    fn preserves_existing_image_data_urls() -> Result<()> {
        let call_tool_result = CallToolResult {
            content: vec![edgerun_json::json!({
                "type": "image",
                "data": "data:image/png;base64,BASE64",
                "mimeType": "image/png"
            })],
            structured_content: None,
            is_error: Some(false),
            meta: None,
        };

        let payload = call_tool_result.into_function_call_output_payload();
        let Some(items) = payload.content_items() else {
            panic!("expected content items");
        };
        let items = items.to_vec();
        assert_eq!(
            items,
            vec![FunctionCallOutputContentItem::InputImage {
                image_url: "data:image/png;base64,BASE64".into(),
                detail: Some(DEFAULT_IMAGE_DETAIL),
            }]
        );

        Ok(())
    }

    #[test]
    fn preserves_original_detail_metadata_on_mcp_images() -> Result<()> {
        let call_tool_result = CallToolResult {
            content: vec![edgerun_json::json!({
                "type": "image",
                "data": "BASE64",
                "mimeType": "image/png",
                "_meta": {
                    "codex/imageDetail": "original",
                },
            })],
            structured_content: None,
            is_error: Some(false),
            meta: None,
        };

        let payload = call_tool_result.into_function_call_output_payload();
        let Some(items) = payload.content_items() else {
            panic!("expected content items");
        };
        let items = items.to_vec();
        assert_eq!(
            items,
            vec![FunctionCallOutputContentItem::InputImage {
                image_url: "data:image/png;base64,BASE64".into(),
                detail: Some(ImageDetail::Original),
            }]
        );

        Ok(())
    }

    #[test]
    fn preserves_standard_detail_metadata_on_mcp_images() -> Result<()> {
        let call_tool_result = CallToolResult {
            content: vec![edgerun_json::json!({
                "type": "image",
                "data": "BASE64",
                "mimeType": "image/png",
                "_meta": {
                    "codex/imageDetail": "high",
                },
            })],
            structured_content: None,
            is_error: Some(false),
            meta: None,
        };

        let payload = call_tool_result.into_function_call_output_payload();
        let Some(items) = payload.content_items() else {
            panic!("expected content items");
        };
        let items = items.to_vec();
        assert_eq!(
            items,
            vec![FunctionCallOutputContentItem::InputImage {
                image_url: "data:image/png;base64,BASE64".into(),
                detail: Some(ImageDetail::High),
            }]
        );

        Ok(())
    }

    #[test]
    fn deserializes_array_payload_into_items() -> Result<()> {
        let json = r#"[
            {"type": "input_text", "text": "note"},
            {"type": "input_image", "image_url": "data:image/png;base64,XYZ"}
        ]"#;

        let payload: FunctionCallOutputPayload = from_json_str(json);

        assert_eq!(payload.success, None);
        let expected_items = vec![
            FunctionCallOutputContentItem::InputText {
                text: "note".into(),
            },
            FunctionCallOutputContentItem::InputImage {
                image_url: "data:image/png;base64,XYZ".into(),
                detail: None,
            },
        ];
        assert_eq!(
            payload.body,
            FunctionCallOutputBody::ContentItems(expected_items.clone())
        );
        assert_eq!(
            edgerun_json::to_string(&payload)?,
            edgerun_json::to_string(&expected_items)?
        );

        Ok(())
    }

    #[test]
    fn deserializes_compaction_alias() -> Result<()> {
        let json = r#"{"type":"compaction_summary","encrypted_content":"abc"}"#;

        let item: ResponseItem = from_json_str(json);

        assert_eq!(
            item,
            ResponseItem::Compaction {
                encrypted_content: "abc".into(),
            }
        );
        Ok(())
    }

    #[test]
    fn deserializes_context_compaction() -> Result<()> {
        let json = r#"{"type":"context_compaction","encrypted_content":"abc"}"#;

        let item: ResponseItem = from_json_str(json);

        assert_eq!(
            item,
            ResponseItem::ContextCompaction {
                encrypted_content: Some("abc".into()),
            }
        );
        Ok(())
    }

    #[test]
    fn serializes_context_compaction_trigger_without_payload() -> Result<()> {
        let item = ResponseItem::ContextCompaction {
            encrypted_content: None,
        };

        assert_eq!(
            edgerun_json::to_value(&item),
            edgerun_json::json!({
                "type": "context_compaction",
            })
        );
        Ok(())
    }

    #[test]
    fn deserializes_legacy_ghost_snapshot_as_other() -> Result<()> {
        let json = r#"{
            "type":"ghost_snapshot",
            "ghost_commit":{
                "id":"ghost-1",
                "parent":null,
                "preexisting_untracked_files":[],
                "preexisting_untracked_dirs":[]
            }
        }"#;

        let item: ResponseItem = from_json_str(json);

        assert_eq!(item, ResponseItem::Other);
        Ok(())
    }

    #[test]
    fn roundtrips_web_search_call_actions() -> Result<()> {
        let cases = vec![
            (
                r#"{
                    "type": "web_search_call",
                    "status": "completed",
                    "action": {
                        "type": "search",
                        "query": "weather seattle",
                        "queries": ["weather seattle", "seattle weather now"]
                    }
                }"#,
                None,
                Some(WebSearchAction::Search {
                    query: Some("weather seattle".into()),
                    queries: Some(vec!["weather seattle".into(), "seattle weather now".into()]),
                }),
                Some("completed".into()),
                true,
            ),
            (
                r#"{
                    "type": "web_search_call",
                    "status": "open",
                    "action": {
                        "type": "open_page",
                        "url": "https://example.com"
                    }
                }"#,
                None,
                Some(WebSearchAction::OpenPage {
                    url: Some("https://example.com".into()),
                }),
                Some("open".into()),
                true,
            ),
            (
                r#"{
                    "type": "web_search_call",
                    "status": "in_progress",
                    "action": {
                        "type": "find_in_page",
                        "url": "https://example.com/docs",
                        "pattern": "installation"
                    }
                }"#,
                None,
                Some(WebSearchAction::FindInPage {
                    url: Some("https://example.com/docs".into()),
                    pattern: Some("installation".into()),
                }),
                Some("in_progress".into()),
                true,
            ),
            (
                r#"{
                    "type": "web_search_call",
                    "status": "in_progress",
                    "id": "ws_partial"
                }"#,
                Some("ws_partial".into()),
                None,
                Some("in_progress".into()),
                false,
            ),
        ];

        for (json_literal, expected_id, expected_action, expected_status, expect_roundtrip) in cases
        {
            let parsed: ResponseItem = from_json_str(json_literal);
            let expected = ResponseItem::WebSearchCall {
                id: expected_id.clone(),
                status: expected_status.clone(),
                action: expected_action.clone(),
            };
            assert_eq!(parsed, expected);

            let serialized = edgerun_json::to_value(&parsed);
            let mut expected_serialized: edgerun_json::Value =
                edgerun_json::from_str(json_literal)?;
            if !expect_roundtrip && let Some(obj) = expected_serialized.as_object_mut() {
                obj.remove("id");
            }
            assert_eq!(serialized, expected_serialized);
        }

        Ok(())
    }

    #[test]
    fn deserialize_shell_tool_call_params() -> Result<()> {
        let json = r#"{
            "command": ["ls", "-l"],
            "workdir": "/tmp",
            "timeout": 1000
        }"#;

        let params: ShellToolCallParams = from_json_str(json);
        assert_eq!(
            ShellToolCallParams {
                command: vec!["ls".to_string(), "-l".to_string()],
                workdir: Some("/tmp".to_string()),
                timeout_ms: Some(1000),
                prefix_rule: None,
            },
            params
        );
        Ok(())
    }

    #[test]
    fn wraps_image_user_input_with_tags() -> Result<()> {
        let image_url = "data:image/png;base64,abc".to_string();

        let item = ResponseInputItem::from(vec![UserInput::Image {
            image_url: image_url.clone(),
        }]);

        match item {
            ResponseInputItem::Message { content, .. } => {
                let expected = vec![
                    ContentItem::InputText {
                        text: image_open_tag_text(),
                    },
                    ContentItem::InputImage {
                        image_url,
                        detail: Some(DEFAULT_IMAGE_DETAIL),
                    },
                    ContentItem::InputText {
                        text: image_close_tag_text(),
                    },
                ];
                assert_eq!(content, expected);
            }
            other => panic!("expected message response but got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn tool_search_call_roundtrips() -> Result<()> {
        let parsed: ResponseItem = from_json_str(
            r#"{
                "type": "tool_search_call",
                "call_id": "search-1",
                "execution": "client",
                "arguments": {
                    "query": "calendar create",
                    "limit": 1
                }
            }"#,
        );

        assert_eq!(
            parsed,
            ResponseItem::ToolSearchCall {
                id: None,
                call_id: Some("search-1".to_string()),
                status: None,
                execution: "client".to_string(),
                arguments: edgerun_json::json!({
                    "query": "calendar create",
                    "limit": 1,
                }),
            }
        );

        assert_eq!(
            edgerun_json::to_value(&parsed),
            edgerun_json::json!({
                "type": "tool_search_call",
                "call_id": "search-1",
                "execution": "client",
                "arguments": {
                    "query": "calendar create",
                    "limit": 1,
                }
            })
        );

        Ok(())
    }

    #[test]
    fn tool_search_output_roundtrips() -> Result<()> {
        let input = ResponseInputItem::ToolSearchOutput {
            call_id: "search-1".to_string(),
            status: "completed".to_string(),
            execution: "client".to_string(),
            tools: vec![edgerun_json::json!({
                "type": "function",
                "name": "mcp__codex_apps__calendar_create_event",
                "description": "Create a calendar event.",
                "defer_loading": true,
                "parameters": {
                    "type": "object",
                    "properties": {
                        "title": {"type": "string"}
                    },
                    "required": ["title"],
                    "additionalProperties": false,
                }
            })],
        };
        assert_eq!(
            ResponseItem::from(input.clone()),
            ResponseItem::ToolSearchOutput {
                call_id: Some("search-1".to_string()),
                status: "completed".to_string(),
                execution: "client".to_string(),
                tools: vec![edgerun_json::json!({
                    "type": "function",
                    "name": "mcp__codex_apps__calendar_create_event",
                    "description": "Create a calendar event.",
                    "defer_loading": true,
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "title": {"type": "string"}
                        },
                        "required": ["title"],
                        "additionalProperties": false,
                    }
                })],
            }
        );

        assert_eq!(
            edgerun_json::to_value(&input),
            edgerun_json::json!({
                "type": "tool_search_output",
                "call_id": "search-1",
                "status": "completed",
                "execution": "client",
                "tools": [{
                    "type": "function",
                    "name": "mcp__codex_apps__calendar_create_event",
                    "description": "Create a calendar event.",
                    "defer_loading": true,
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "title": {"type": "string"}
                        },
                        "required": ["title"],
                        "additionalProperties": false,
                    }
                }]
            })
        );

        Ok(())
    }

    #[test]
    fn tool_search_server_items_allow_null_call_id() -> Result<()> {
        let parsed_call: ResponseItem = from_json_str(
            r#"{
                "type": "tool_search_call",
                "execution": "server",
                "call_id": null,
                "status": "completed",
                "arguments": {
                    "paths": ["crm"]
                }
            }"#,
        );
        assert_eq!(
            parsed_call,
            ResponseItem::ToolSearchCall {
                id: None,
                call_id: None,
                status: Some("completed".to_string()),
                execution: "server".to_string(),
                arguments: edgerun_json::json!({
                    "paths": ["crm"],
                }),
            }
        );

        let parsed_output: ResponseItem = from_json_str(
            r#"{
                "type": "tool_search_output",
                "execution": "server",
                "call_id": null,
                "status": "completed",
                "tools": []
            }"#,
        );
        assert_eq!(
            parsed_output,
            ResponseItem::ToolSearchOutput {
                call_id: None,
                status: "completed".to_string(),
                execution: "server".to_string(),
                tools: vec![],
            }
        );

        Ok(())
    }

    #[test]
    fn mixed_remote_and_local_images_share_label_sequence() -> Result<()> {
        let image_url = "data:image/png;base64,abc".to_string();
        let dir = tempdir()?;
        let local_path = dir.path().join("local.png");
        // A tiny valid PNG (1x1) so this test doesn't depend on cross-crate file paths, which
        // break under Bazel sandboxing.
        const TINY_PNG_BYTES: &[u8] = &[
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1,
            8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 11, 73, 68, 65, 84, 120, 156, 99, 96, 0, 2,
            0, 0, 5, 0, 1, 122, 94, 171, 63, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
        ];
        std::fs::write(&local_path, TINY_PNG_BYTES)?;

        let item = ResponseInputItem::from(vec![
            UserInput::Image {
                image_url: image_url.clone(),
            },
            UserInput::LocalImage { path: local_path },
        ]);

        match item {
            ResponseInputItem::Message { content, .. } => {
                assert_eq!(
                    content.first(),
                    Some(&ContentItem::InputText {
                        text: image_open_tag_text(),
                    })
                );
                assert_eq!(
                    content.get(1),
                    Some(&ContentItem::InputImage {
                        image_url,
                        detail: Some(DEFAULT_IMAGE_DETAIL),
                    })
                );
                assert_eq!(
                    content.get(2),
                    Some(&ContentItem::InputText {
                        text: image_close_tag_text(),
                    })
                );
                assert_eq!(
                    content.get(3),
                    Some(&ContentItem::InputText {
                        text: local_image_open_tag_text(/*label_number*/ 2),
                    })
                );
                assert!(matches!(
                    content.get(4),
                    Some(ContentItem::InputImage { .. })
                ));
                assert_eq!(
                    content.get(5),
                    Some(&ContentItem::InputText {
                        text: image_close_tag_text(),
                    })
                );
            }
            other => panic!("expected message response but got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn local_image_read_error_adds_placeholder() -> Result<()> {
        let dir = tempdir()?;
        let missing_path = dir.path().join("missing-image.png");

        let item = ResponseInputItem::from(vec![UserInput::LocalImage {
            path: missing_path.clone(),
        }]);

        match item {
            ResponseInputItem::Message { content, .. } => {
                assert_eq!(content.len(), 1);
                match &content[0] {
                    ContentItem::InputText { text } => {
                        let display_path = missing_path.display().to_string();
                        assert!(
                            text.contains(&display_path),
                            "placeholder should mention missing path: {text}"
                        );
                        assert!(
                            text.contains("could not read"),
                            "placeholder should mention read issue: {text}"
                        );
                    }
                    other => panic!("expected placeholder text but found {other:?}"),
                }
            }
            other => panic!("expected message response but got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn local_image_non_image_adds_placeholder() -> Result<()> {
        let dir = tempdir()?;
        let json_path = dir.path().join("example.json");
        std::fs::write(&json_path, br#"{"hello":"world"}"#)?;

        let item = ResponseInputItem::from(vec![UserInput::LocalImage {
            path: json_path.clone(),
        }]);

        match item {
            ResponseInputItem::Message { content, .. } => {
                assert_eq!(content.len(), 1);
                match &content[0] {
                    ContentItem::InputText { text } => {
                        assert!(
                            text.contains("unsupported image `application/json`"),
                            "placeholder should mention unsupported image MIME: {text}"
                        );
                        assert!(
                            text.contains(&json_path.display().to_string()),
                            "placeholder should mention path: {text}"
                        );
                    }
                    other => panic!("expected placeholder text but found {other:?}"),
                }
            }
            other => panic!("expected message response but got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn local_image_unsupported_image_format_adds_placeholder() -> Result<()> {
        let dir = tempdir()?;
        let svg_path = dir.path().join("example.svg");
        std::fs::write(
            &svg_path,
            br#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"></svg>"#,
        )?;

        let item = ResponseInputItem::from(vec![UserInput::LocalImage {
            path: svg_path.clone(),
        }]);

        match item {
            ResponseInputItem::Message { content, .. } => {
                assert_eq!(content.len(), 1);
                let expected = format!(
                    "Codex cannot attach image at `{}`: unsupported image `image/svg+xml`.",
                    svg_path.display()
                );
                match &content[0] {
                    ContentItem::InputText { text } => assert_eq!(text, &expected),
                    other => panic!("expected placeholder text but found {other:?}"),
                }
            }
            other => panic!("expected message response but got {other:?}"),
        }

        Ok(())
    }
}
