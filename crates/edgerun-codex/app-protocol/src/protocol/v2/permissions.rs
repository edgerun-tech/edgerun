use super::shared::v2_enum_from_core;
use codex_protocol::approvals::ExecPolicyAmendment as CoreExecPolicyAmendment;
use codex_protocol::approvals::NetworkApprovalContext as CoreNetworkApprovalContext;
use codex_protocol::approvals::NetworkApprovalProtocol as CoreNetworkApprovalProtocol;
use codex_protocol::approvals::NetworkPolicyAmendment as CoreNetworkPolicyAmendment;
use codex_protocol::approvals::NetworkPolicyRuleAction as CoreNetworkPolicyRuleAction;
use codex_protocol::compat::absolute_path::AbsolutePathBuf;
use codex_protocol::models::ActivePermissionProfile as CoreActivePermissionProfile;
use codex_protocol::models::ActivePermissionProfileModification as CoreActivePermissionProfileModification;
use codex_protocol::models::AdditionalPermissionProfile as CoreAdditionalPermissionProfile;
use codex_protocol::models::FileSystemPermissions as CoreFileSystemPermissions;
use codex_protocol::models::ManagedFileSystemPermissions as CoreManagedFileSystemPermissions;
use codex_protocol::models::NetworkPermissions as CoreNetworkPermissions;
use codex_protocol::models::PermissionProfile as CorePermissionProfile;
use codex_protocol::permissions::FileSystemAccessMode as CoreFileSystemAccessMode;
use codex_protocol::permissions::FileSystemPath as CoreFileSystemPath;
use codex_protocol::permissions::FileSystemSandboxEntry as CoreFileSystemSandboxEntry;
use codex_protocol::permissions::FileSystemSpecialPath as CoreFileSystemSpecialPath;
use codex_protocol::permissions::NetworkSandboxPolicy as CoreNetworkSandboxPolicy;
use codex_protocol::protocol::NetworkAccess as CoreNetworkAccess;
use codex_protocol::request_permissions::PermissionGrantScope as CorePermissionGrantScope;
use codex_protocol::request_permissions::RequestPermissionProfile as CoreRequestPermissionProfile;
use edgerun_json::FromJson;
use edgerun_json::JsonValueError;
use edgerun_json::Map;
use edgerun_json::ToJson;
use edgerun_json::Value;
use edgerun_serde::Deserialize;
use edgerun_serde::Serialize;
use schemars::JsonSchema;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use ts_rs::TS;

v2_enum_from_core! {
    pub enum NetworkApprovalProtocol from CoreNetworkApprovalProtocol {
        Http,
        Https,
        Socks5Tcp,
        Socks5Udp,
    }
}

impl ToJson for NetworkApprovalProtocol {
    fn to_json(&self) -> Value {
        Value::from(match self {
            Self::Http => "http",
            Self::Https => "https",
            Self::Socks5Tcp => "socks5Tcp",
            Self::Socks5Udp => "socks5Udp",
        })
    }
}

impl FromJson for NetworkApprovalProtocol {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "http" => Ok(Self::Http),
            "https" => Ok(Self::Https),
            "socks5Tcp" => Ok(Self::Socks5Tcp),
            "socks5Udp" => Ok(Self::Socks5Udp),
            other => Err(JsonValueError::WrongType(format!(
                "unknown network approval protocol `{other}`"
            ))),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct NetworkApprovalContext {
    pub host: String,
    pub protocol: NetworkApprovalProtocol,
}

impl From<CoreNetworkApprovalContext> for NetworkApprovalContext {
    fn from(value: CoreNetworkApprovalContext) -> Self {
        Self {
            host: value.host,
            protocol: value.protocol.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct AdditionalFileSystemPermissions {
    /// This will be removed in favor of `entries`.
    pub read: Option<Vec<AbsolutePathBuf>>,
    /// This will be removed in favor of `entries`.
    pub write: Option<Vec<AbsolutePathBuf>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub glob_scan_max_depth: Option<NonZeroUsize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub entries: Option<Vec<FileSystemSandboxEntry>>,
}

impl ToJson for AdditionalFileSystemPermissions {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("read", absolute_paths_option_json(self.read.as_deref()));
        object.push_field("write", absolute_paths_option_json(self.write.as_deref()));
        if let Some(depth) = self.glob_scan_max_depth {
            object.push_field("globScanMaxDepth", depth.get() as u64);
        }
        if let Some(entries) = &self.entries {
            object.push_field("entries", entries.to_json());
        }
        Value::Object(object)
    }
}

impl FromJson for AdditionalFileSystemPermissions {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("AdditionalFileSystemPermissions")?;
        Ok(Self {
            read: optional_absolute_paths(object.remove("read"))?,
            write: optional_absolute_paths(object.remove("write"))?,
            glob_scan_max_depth: optional_nonzero_usize(object.remove("globScanMaxDepth"))?,
            entries: object.take_optional("entries")?,
        })
    }
}

impl From<CoreFileSystemPermissions> for AdditionalFileSystemPermissions {
    fn from(value: CoreFileSystemPermissions) -> Self {
        if let Some((read, write)) = value.legacy_read_write_roots() {
            let mut entries = Vec::with_capacity(
                read.as_ref().map_or(0, Vec::len) + write.as_ref().map_or(0, Vec::len),
            );
            if let Some(paths) = read.as_ref() {
                entries.extend(paths.iter().map(|path| FileSystemSandboxEntry {
                    path: FileSystemPath::Path { path: path.clone() },
                    access: FileSystemAccessMode::Read,
                }));
            }
            if let Some(paths) = write.as_ref() {
                entries.extend(paths.iter().map(|path| FileSystemSandboxEntry {
                    path: FileSystemPath::Path { path: path.clone() },
                    access: FileSystemAccessMode::Write,
                }));
            }
            Self {
                read,
                write,
                glob_scan_max_depth: None,
                entries: Some(entries),
            }
        } else {
            Self {
                read: None,
                write: None,
                glob_scan_max_depth: value.glob_scan_max_depth,
                entries: Some(
                    value
                        .entries
                        .into_iter()
                        .map(FileSystemSandboxEntry::from)
                        .collect(),
                ),
            }
        }
    }
}

impl From<AdditionalFileSystemPermissions> for CoreFileSystemPermissions {
    fn from(value: AdditionalFileSystemPermissions) -> Self {
        let mut permissions = if let Some(entries) = value.entries {
            Self {
                entries: entries
                    .into_iter()
                    .map(CoreFileSystemSandboxEntry::from)
                    .collect(),
                glob_scan_max_depth: None,
            }
        } else {
            CoreFileSystemPermissions::from_read_write_roots(value.read, value.write)
        };
        permissions.glob_scan_max_depth = value.glob_scan_max_depth;
        permissions
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct AdditionalNetworkPermissions {
    pub enabled: Option<bool>,
}

impl ToJson for AdditionalNetworkPermissions {
    fn to_json(&self) -> Value {
        let mut object = Map::with_capacity(1);
        object.push_field("enabled", self.enabled.to_json());
        Value::Object(object)
    }
}

impl FromJson for AdditionalNetworkPermissions {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("AdditionalNetworkPermissions")?;
        Ok(Self {
            enabled: object.take_optional("enabled")?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct PermissionProfileNetworkPermissions {
    pub enabled: bool,
}

impl ToJson for PermissionProfileNetworkPermissions {
    fn to_json(&self) -> Value {
        let mut object = Map::with_capacity(1);
        object.push_field("enabled", self.enabled);
        Value::Object(object)
    }
}

impl FromJson for PermissionProfileNetworkPermissions {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("PermissionProfileNetworkPermissions")?;
        Ok(Self {
            enabled: object.take_required("enabled")?,
        })
    }
}

impl From<CoreNetworkPermissions> for AdditionalNetworkPermissions {
    fn from(value: CoreNetworkPermissions) -> Self {
        Self {
            enabled: value.enabled,
        }
    }
}

impl From<AdditionalNetworkPermissions> for CoreNetworkPermissions {
    fn from(value: AdditionalNetworkPermissions) -> Self {
        Self {
            enabled: value.enabled,
        }
    }
}

impl From<CoreNetworkSandboxPolicy> for PermissionProfileNetworkPermissions {
    fn from(value: CoreNetworkSandboxPolicy) -> Self {
        Self {
            enabled: value.is_enabled(),
        }
    }
}

impl From<PermissionProfileNetworkPermissions> for CoreNetworkSandboxPolicy {
    fn from(value: PermissionProfileNetworkPermissions) -> Self {
        if value.enabled {
            Self::Enabled
        } else {
            Self::Restricted
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
#[ts(export_to = "v2/")]
pub struct RequestPermissionProfile {
    pub network: Option<AdditionalNetworkPermissions>,
    pub file_system: Option<AdditionalFileSystemPermissions>,
}

impl ToJson for RequestPermissionProfile {
    fn to_json(&self) -> Value {
        let mut object = Map::with_capacity(2);
        object.push_field("network", self.network.to_json());
        object.push_field("fileSystem", self.file_system.to_json());
        Value::Object(object)
    }
}

impl FromJson for RequestPermissionProfile {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("RequestPermissionProfile")?;
        Ok(Self {
            network: object.take_optional("network")?,
            file_system: object.take_optional("fileSystem")?,
        })
    }
}

impl From<CoreRequestPermissionProfile> for RequestPermissionProfile {
    fn from(value: CoreRequestPermissionProfile) -> Self {
        Self {
            network: value.network.map(AdditionalNetworkPermissions::from),
            file_system: value.file_system.map(AdditionalFileSystemPermissions::from),
        }
    }
}

impl From<RequestPermissionProfile> for CoreRequestPermissionProfile {
    fn from(value: RequestPermissionProfile) -> Self {
        Self {
            network: value.network.map(CoreNetworkPermissions::from),
            file_system: value.file_system.map(CoreFileSystemPermissions::from),
        }
    }
}

v2_enum_from_core!(
    pub enum FileSystemAccessMode from CoreFileSystemAccessMode {
        Read,
        Write,
        None
    }
);

impl ToJson for FileSystemAccessMode {
    fn to_json(&self) -> Value {
        Value::from(match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::None => "none",
        })
    }
}

impl FromJson for FileSystemAccessMode {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "read" => Ok(Self::Read),
            "write" => Ok(Self::Write),
            "none" => Ok(Self::None),
            other => Err(JsonValueError::WrongType(format!(
                "unknown file system access mode `{other}`"
            ))),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(tag = "kind")]
#[ts(export_to = "v2/")]
pub enum FileSystemSpecialPath {
    Root,
    Minimal,
    #[serde(alias = "current_working_directory")]
    ProjectRoots {
        subpath: Option<PathBuf>,
    },
    Tmpdir,
    SlashTmp,
    Unknown {
        path: String,
        subpath: Option<PathBuf>,
    },
}

impl ToJson for FileSystemSpecialPath {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        match self {
            Self::Root => object.push_field("kind", "root"),
            Self::Minimal => object.push_field("kind", "minimal"),
            Self::ProjectRoots { subpath } => {
                object.push_field("kind", "project_roots");
                object.push_field(
                    "subpath",
                    subpath
                        .as_ref()
                        .map(|path| path.to_string_lossy().into_owned())
                        .to_json(),
                );
            }
            Self::Tmpdir => object.push_field("kind", "tmpdir"),
            Self::SlashTmp => object.push_field("kind", "slash_tmp"),
            Self::Unknown { path, subpath } => {
                object.push_field("kind", "unknown");
                object.push_field("path", path);
                object.push_field(
                    "subpath",
                    subpath
                        .as_ref()
                        .map(|path| path.to_string_lossy().into_owned())
                        .to_json(),
                );
            }
        }
        Value::Object(object)
    }
}

impl FromJson for FileSystemSpecialPath {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("FileSystemSpecialPath")?;
        let kind: String = object.take_required("kind")?;
        let subpath = object
            .take_optional::<String>("subpath")?
            .map(PathBuf::from);
        match kind.as_str() {
            "root" => Ok(Self::Root),
            "minimal" => Ok(Self::Minimal),
            "project_roots" | "current_working_directory" => Ok(Self::ProjectRoots { subpath }),
            "tmpdir" => Ok(Self::Tmpdir),
            "slash_tmp" => Ok(Self::SlashTmp),
            "unknown" => Ok(Self::Unknown {
                path: object.take_required("path")?,
                subpath,
            }),
            other => Ok(Self::Unknown {
                path: other.to_string(),
                subpath,
            }),
        }
    }
}

impl From<CoreFileSystemSpecialPath> for FileSystemSpecialPath {
    fn from(value: CoreFileSystemSpecialPath) -> Self {
        match value {
            CoreFileSystemSpecialPath::Root => Self::Root,
            CoreFileSystemSpecialPath::Minimal => Self::Minimal,
            CoreFileSystemSpecialPath::ProjectRoots { subpath } => Self::ProjectRoots { subpath },
            CoreFileSystemSpecialPath::Tmpdir => Self::Tmpdir,
            CoreFileSystemSpecialPath::SlashTmp => Self::SlashTmp,
            CoreFileSystemSpecialPath::Unknown { path, subpath } => Self::Unknown { path, subpath },
        }
    }
}

impl From<FileSystemSpecialPath> for CoreFileSystemSpecialPath {
    fn from(value: FileSystemSpecialPath) -> Self {
        match value {
            FileSystemSpecialPath::Root => Self::Root,
            FileSystemSpecialPath::Minimal => Self::Minimal,
            FileSystemSpecialPath::ProjectRoots { subpath } => Self::ProjectRoots { subpath },
            FileSystemSpecialPath::Tmpdir => Self::Tmpdir,
            FileSystemSpecialPath::SlashTmp => Self::SlashTmp,
            FileSystemSpecialPath::Unknown { path, subpath } => Self::Unknown { path, subpath },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(tag = "type")]
#[ts(export_to = "v2/")]
pub enum FileSystemPath {
    Path { path: AbsolutePathBuf },
    GlobPattern { pattern: String },
    Special { value: FileSystemSpecialPath },
}

impl ToJson for FileSystemPath {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        match self {
            Self::Path { path } => {
                object.push_field("type", "path");
                object.push_field("path", absolute_path_json(path));
            }
            Self::GlobPattern { pattern } => {
                object.push_field("type", "glob_pattern");
                object.push_field("pattern", pattern.clone());
            }
            Self::Special { value } => {
                object.push_field("type", "special");
                object.push_field("value", value.to_json());
            }
        }
        Value::Object(object)
    }
}

impl FromJson for FileSystemPath {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("FileSystemPath")?;
        let ty: String = object.take_required("type")?;
        match ty.as_str() {
            "path" => Ok(Self::Path {
                path: required_absolute_path(object.remove("path"), "path")?,
            }),
            "glob_pattern" => Ok(Self::GlobPattern {
                pattern: object.take_required("pattern")?,
            }),
            "special" => Ok(Self::Special {
                value: object.take_required("value")?,
            }),
            other => Err(JsonValueError::WrongType(format!(
                "unknown file system path type `{other}`"
            ))),
        }
    }
}

impl From<CoreFileSystemPath> for FileSystemPath {
    fn from(value: CoreFileSystemPath) -> Self {
        match value {
            CoreFileSystemPath::Path { path } => Self::Path { path },
            CoreFileSystemPath::GlobPattern { pattern } => Self::GlobPattern { pattern },
            CoreFileSystemPath::Special { value } => Self::Special {
                value: value.into(),
            },
        }
    }
}

impl From<FileSystemPath> for CoreFileSystemPath {
    fn from(value: FileSystemPath) -> Self {
        match value {
            FileSystemPath::Path { path } => Self::Path { path },
            FileSystemPath::GlobPattern { pattern } => Self::GlobPattern { pattern },
            FileSystemPath::Special { value } => Self::Special {
                value: value.into(),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct FileSystemSandboxEntry {
    pub path: FileSystemPath,
    pub access: FileSystemAccessMode,
}

impl ToJson for FileSystemSandboxEntry {
    fn to_json(&self) -> Value {
        let mut object = Map::with_capacity(2);
        object.push_field("path", self.path.to_json());
        object.push_field("access", self.access.to_json());
        Value::Object(object)
    }
}

impl FromJson for FileSystemSandboxEntry {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("FileSystemSandboxEntry")?;
        Ok(Self {
            path: object.take_required("path")?,
            access: object.take_required("access")?,
        })
    }
}

impl From<CoreFileSystemSandboxEntry> for FileSystemSandboxEntry {
    fn from(value: CoreFileSystemSandboxEntry) -> Self {
        Self {
            path: value.path.into(),
            access: value.access.into(),
        }
    }
}

impl From<FileSystemSandboxEntry> for CoreFileSystemSandboxEntry {
    fn from(value: FileSystemSandboxEntry) -> Self {
        Self {
            path: value.path.into(),
            access: value.access.to_core(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(tag = "type")]
#[ts(export_to = "v2/")]
pub enum PermissionProfileFileSystemPermissions {
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Restricted {
        entries: Vec<FileSystemSandboxEntry>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        glob_scan_max_depth: Option<NonZeroUsize>,
    },
    Unrestricted,
}

impl ToJson for PermissionProfileFileSystemPermissions {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        match self {
            Self::Restricted {
                entries,
                glob_scan_max_depth,
            } => {
                object.push_field("type", "restricted");
                object.push_field("entries", entries.to_json());
                if let Some(depth) = glob_scan_max_depth {
                    object.push_field("globScanMaxDepth", depth.get() as u64);
                }
            }
            Self::Unrestricted => {
                object.push_field("type", "unrestricted");
            }
        }
        Value::Object(object)
    }
}

impl FromJson for PermissionProfileFileSystemPermissions {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("PermissionProfileFileSystemPermissions")?;
        let ty: String = object.take_required("type")?;
        match ty.as_str() {
            "restricted" => Ok(Self::Restricted {
                entries: object.take_required("entries")?,
                glob_scan_max_depth: optional_nonzero_usize(object.remove("globScanMaxDepth"))?,
            }),
            "unrestricted" => Ok(Self::Unrestricted),
            other => Err(JsonValueError::WrongType(format!(
                "unknown permission profile file system type `{other}`"
            ))),
        }
    }
}

impl From<CoreManagedFileSystemPermissions> for PermissionProfileFileSystemPermissions {
    fn from(value: CoreManagedFileSystemPermissions) -> Self {
        match value {
            CoreManagedFileSystemPermissions::Restricted {
                entries,
                glob_scan_max_depth,
            } => Self::Restricted {
                entries: entries
                    .into_iter()
                    .map(FileSystemSandboxEntry::from)
                    .collect(),
                glob_scan_max_depth,
            },
            CoreManagedFileSystemPermissions::Unrestricted => Self::Unrestricted,
        }
    }
}

impl From<PermissionProfileFileSystemPermissions> for CoreManagedFileSystemPermissions {
    fn from(value: PermissionProfileFileSystemPermissions) -> Self {
        match value {
            PermissionProfileFileSystemPermissions::Restricted {
                entries,
                glob_scan_max_depth,
            } => Self::Restricted {
                entries: entries
                    .into_iter()
                    .map(CoreFileSystemSandboxEntry::from)
                    .collect(),
                glob_scan_max_depth,
            },
            PermissionProfileFileSystemPermissions::Unrestricted => Self::Unrestricted,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(tag = "type")]
#[ts(export_to = "v2/")]
pub enum PermissionProfile {
    /// Codex owns sandbox construction for this profile.
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Managed {
        network: PermissionProfileNetworkPermissions,
        file_system: PermissionProfileFileSystemPermissions,
    },
    /// Do not apply an outer sandbox.
    Disabled,
    /// Filesystem isolation is enforced by an external caller.
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    External {
        network: PermissionProfileNetworkPermissions,
    },
}

impl ToJson for PermissionProfile {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        match self {
            Self::Managed {
                network,
                file_system,
            } => {
                object.push_field("type", "managed");
                object.push_field("network", network.to_json());
                object.push_field("fileSystem", file_system.to_json());
            }
            Self::Disabled => {
                object.push_field("type", "disabled");
            }
            Self::External { network } => {
                object.push_field("type", "external");
                object.push_field("network", network.to_json());
            }
        }
        Value::Object(object)
    }
}

impl FromJson for PermissionProfile {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("PermissionProfile")?;
        let ty: String = object.take_required("type")?;
        match ty.as_str() {
            "managed" => Ok(Self::Managed {
                network: object.take_required("network")?,
                file_system: object.take_required("fileSystem")?,
            }),
            "disabled" => Ok(Self::Disabled),
            "external" => Ok(Self::External {
                network: object.take_required("network")?,
            }),
            other => Err(JsonValueError::WrongType(format!(
                "unknown permission profile type `{other}`"
            ))),
        }
    }
}

impl From<CorePermissionProfile> for PermissionProfile {
    fn from(value: CorePermissionProfile) -> Self {
        match value {
            CorePermissionProfile::Managed {
                file_system,
                network,
            } => Self::Managed {
                network: network.into(),
                file_system: file_system.into(),
            },
            CorePermissionProfile::Disabled => Self::Disabled,
            CorePermissionProfile::External { network } => Self::External {
                network: network.into(),
            },
        }
    }
}

impl From<PermissionProfile> for CorePermissionProfile {
    fn from(value: PermissionProfile) -> Self {
        match value {
            PermissionProfile::Managed {
                file_system,
                network,
            } => Self::Managed {
                file_system: file_system.into(),
                network: network.into(),
            },
            PermissionProfile::Disabled => Self::Disabled,
            PermissionProfile::External { network } => Self::External {
                network: network.into(),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ActivePermissionProfile {
    /// Identifier from `default_permissions` or the implicit built-in default,
    /// such as `:workspace` or a user-defined `[permissions.<id>]` profile.
    pub id: String,
    /// Parent profile identifier once permissions profiles support
    /// inheritance. This is currently always `null`.
    #[serde(default)]
    pub extends: Option<String>,
    /// Bounded user-requested modifications applied on top of the named
    /// profile, if any.
    #[serde(default)]
    pub modifications: Vec<ActivePermissionProfileModification>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(tag = "type")]
#[ts(export_to = "v2/")]
pub enum ActivePermissionProfileModification {
    /// Additional concrete directory that should be writable.
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    AdditionalWritableRoot { path: AbsolutePathBuf },
}

impl From<CoreActivePermissionProfileModification> for ActivePermissionProfileModification {
    fn from(value: CoreActivePermissionProfileModification) -> Self {
        match value {
            CoreActivePermissionProfileModification::AdditionalWritableRoot { path } => {
                Self::AdditionalWritableRoot { path }
            }
        }
    }
}

impl From<ActivePermissionProfileModification> for CoreActivePermissionProfileModification {
    fn from(value: ActivePermissionProfileModification) -> Self {
        match value {
            ActivePermissionProfileModification::AdditionalWritableRoot { path } => {
                Self::AdditionalWritableRoot { path }
            }
        }
    }
}

impl From<CoreActivePermissionProfile> for ActivePermissionProfile {
    fn from(value: CoreActivePermissionProfile) -> Self {
        Self {
            id: value.id,
            extends: value.extends,
            modifications: value
                .modifications
                .into_iter()
                .map(ActivePermissionProfileModification::from)
                .collect(),
        }
    }
}

impl From<ActivePermissionProfile> for CoreActivePermissionProfile {
    fn from(value: ActivePermissionProfile) -> Self {
        Self {
            id: value.id,
            extends: value.extends,
            modifications: value
                .modifications
                .into_iter()
                .map(CoreActivePermissionProfileModification::from)
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(tag = "type")]
#[ts(export_to = "v2/")]
pub enum PermissionProfileSelectionParams {
    /// Select a named built-in or user-defined profile and optionally apply
    /// bounded modifications that Codex knows how to validate.
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Profile {
        id: String,
        #[ts(optional = nullable)]
        modifications: Option<Vec<PermissionProfileModificationParams>>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(tag = "type")]
#[ts(export_to = "v2/")]
pub enum PermissionProfileModificationParams {
    /// Additional concrete directory that should be writable.
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    AdditionalWritableRoot { path: AbsolutePathBuf },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct AdditionalPermissionProfile {
    /// Partial overlay used for per-command permission requests.
    pub network: Option<AdditionalNetworkPermissions>,
    pub file_system: Option<AdditionalFileSystemPermissions>,
}

impl From<CoreAdditionalPermissionProfile> for AdditionalPermissionProfile {
    fn from(value: CoreAdditionalPermissionProfile) -> Self {
        Self {
            network: value.network.map(AdditionalNetworkPermissions::from),
            file_system: value.file_system.map(AdditionalFileSystemPermissions::from),
        }
    }
}

impl From<AdditionalPermissionProfile> for CoreAdditionalPermissionProfile {
    fn from(value: AdditionalPermissionProfile) -> Self {
        Self {
            network: value.network.map(CoreNetworkPermissions::from),
            file_system: value.file_system.map(CoreFileSystemPermissions::from),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct GrantedPermissionProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub network: Option<AdditionalNetworkPermissions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub file_system: Option<AdditionalFileSystemPermissions>,
}

impl From<GrantedPermissionProfile> for CoreAdditionalPermissionProfile {
    fn from(value: GrantedPermissionProfile) -> Self {
        Self {
            network: value.network.map(CoreNetworkPermissions::from),
            file_system: value.file_system.map(CoreFileSystemPermissions::from),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum NetworkAccess {
    #[default]
    Restricted,
    Enabled,
}

impl ToJson for NetworkAccess {
    fn to_json(&self) -> Value {
        Value::from(match self {
            Self::Restricted => "restricted",
            Self::Enabled => "enabled",
        })
    }
}

impl FromJson for NetworkAccess {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "restricted" => Ok(Self::Restricted),
            "enabled" => Ok(Self::Enabled),
            other => Err(JsonValueError::WrongType(format!(
                "unknown network access `{other}`"
            ))),
        }
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(tag = "type")]
#[ts(export_to = "v2/")]
pub enum SandboxPolicy {
    DangerFullAccess,
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    ReadOnly {
        #[serde(default)]
        network_access: bool,
    },
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    ExternalSandbox {
        #[serde(default)]
        network_access: NetworkAccess,
    },
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    WorkspaceWrite {
        #[serde(default)]
        writable_roots: Vec<AbsolutePathBuf>,
        #[serde(default)]
        network_access: bool,
        #[serde(default)]
        exclude_tmpdir_env_var: bool,
        #[serde(default)]
        exclude_slash_tmp: bool,
    },
}

impl ToJson for SandboxPolicy {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        match self {
            Self::DangerFullAccess => {
                object.push_field("type", "dangerFullAccess");
            }
            Self::ReadOnly { network_access } => {
                object.push_field("type", "readOnly");
                object.push_field("networkAccess", *network_access);
            }
            Self::ExternalSandbox { network_access } => {
                object.push_field("type", "externalSandbox");
                object.push_field("networkAccess", network_access.to_json());
            }
            Self::WorkspaceWrite {
                writable_roots,
                network_access,
                exclude_tmpdir_env_var,
                exclude_slash_tmp,
            } => {
                object.push_field("type", "workspaceWrite");
                object.push_field(
                    "writableRoots",
                    Value::Array(
                        writable_roots
                            .iter()
                            .map(|root| Value::from(root.as_path().display().to_string()))
                            .collect(),
                    ),
                );
                object.push_field("networkAccess", *network_access);
                object.push_field("excludeTmpdirEnvVar", *exclude_tmpdir_env_var);
                object.push_field("excludeSlashTmp", *exclude_slash_tmp);
            }
        }
        Value::Object(object)
    }
}

impl FromJson for SandboxPolicy {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("SandboxPolicy")?;
        let ty: String = object.take_required("type")?;
        match ty.as_str() {
            "dangerFullAccess" => Ok(Self::DangerFullAccess),
            "readOnly" => {
                reject_restricted_legacy_access(object.remove("access"), "readOnly.access")?;
                Ok(Self::ReadOnly {
                    network_access: object.take_optional("networkAccess")?.unwrap_or(false),
                })
            }
            "externalSandbox" => Ok(Self::ExternalSandbox {
                network_access: object
                    .take_optional("networkAccess")?
                    .unwrap_or(NetworkAccess::Restricted),
            }),
            "workspaceWrite" => {
                reject_restricted_legacy_access(
                    object.remove("readOnlyAccess"),
                    "workspaceWrite.readOnlyAccess",
                )?;
                Ok(Self::WorkspaceWrite {
                    writable_roots: absolute_paths_from_json(
                        object
                            .remove("writableRoots")
                            .unwrap_or_else(|| Value::Array(Vec::new())),
                    )?,
                    network_access: object.take_optional("networkAccess")?.unwrap_or(false),
                    exclude_tmpdir_env_var: object
                        .take_optional("excludeTmpdirEnvVar")?
                        .unwrap_or(false),
                    exclude_slash_tmp: object.take_optional("excludeSlashTmp")?.unwrap_or(false),
                })
            }
            other => Err(JsonValueError::WrongType(format!(
                "unknown sandbox policy type `{other}`"
            ))),
        }
    }
}

fn reject_restricted_legacy_access(
    value: Option<Value>,
    field: &str,
) -> Result<(), JsonValueError> {
    let Some(value) = value else {
        return Ok(());
    };
    let mut object = value.into_object(field)?;
    let ty: String = object.take_required("type")?;
    if ty == "restricted" {
        return Err(JsonValueError::WrongType(format!(
            "{field} is no longer supported; use permissionProfile for restricted reads"
        )));
    }
    Ok(())
}

fn absolute_paths_from_json(value: Value) -> Result<Vec<AbsolutePathBuf>, JsonValueError> {
    Vec::<String>::from_json(value)?
        .into_iter()
        .map(|path| {
            AbsolutePathBuf::try_from(PathBuf::from(&path)).map_err(|_| {
                JsonValueError::WrongType(format!("expected absolute path, found `{path}`"))
            })
        })
        .collect()
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum SandboxPolicyDeserialize {
    DangerFullAccess,
    #[serde(rename_all = "camelCase")]
    ReadOnly {
        #[serde(default)]
        network_access: bool,
        #[serde(default)]
        access: Option<LegacyReadOnlyAccess>,
    },
    #[serde(rename_all = "camelCase")]
    ExternalSandbox {
        #[serde(default)]
        network_access: NetworkAccess,
    },
    #[serde(rename_all = "camelCase")]
    WorkspaceWrite {
        #[serde(default)]
        writable_roots: Vec<AbsolutePathBuf>,
        #[serde(default)]
        read_only_access: Option<LegacyReadOnlyAccess>,
        #[serde(default)]
        network_access: bool,
        #[serde(default)]
        exclude_tmpdir_env_var: bool,
        #[serde(default)]
        exclude_slash_tmp: bool,
    },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum LegacyReadOnlyAccess {
    FullAccess,
    Restricted,
}

impl<'de> Deserialize<'de> for SandboxPolicy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: edgerun_serde::Deserializer<'de>,
    {
        match SandboxPolicyDeserialize::deserialize(deserializer)? {
            SandboxPolicyDeserialize::DangerFullAccess => Ok(SandboxPolicy::DangerFullAccess),
            SandboxPolicyDeserialize::ReadOnly {
                network_access,
                access,
            } => {
                if matches!(access, Some(LegacyReadOnlyAccess::Restricted)) {
                    return Err(edgerun_serde::de::Error::custom(
                        "readOnly.access is no longer supported; use permissionProfile for restricted reads",
                    ));
                }
                Ok(SandboxPolicy::ReadOnly { network_access })
            }
            SandboxPolicyDeserialize::ExternalSandbox { network_access } => {
                Ok(SandboxPolicy::ExternalSandbox { network_access })
            }
            SandboxPolicyDeserialize::WorkspaceWrite {
                writable_roots,
                read_only_access,
                network_access,
                exclude_tmpdir_env_var,
                exclude_slash_tmp,
            } => {
                if matches!(read_only_access, Some(LegacyReadOnlyAccess::Restricted)) {
                    return Err(edgerun_serde::de::Error::custom(
                        "workspaceWrite.readOnlyAccess is no longer supported; use permissionProfile for restricted reads",
                    ));
                }
                Ok(SandboxPolicy::WorkspaceWrite {
                    writable_roots,
                    network_access,
                    exclude_tmpdir_env_var,
                    exclude_slash_tmp,
                })
            }
        }
    }
}

impl SandboxPolicy {
    pub fn to_core(&self) -> codex_protocol::protocol::SandboxPolicy {
        match self {
            SandboxPolicy::DangerFullAccess => {
                codex_protocol::protocol::SandboxPolicy::DangerFullAccess
            }
            SandboxPolicy::ReadOnly { network_access } => {
                codex_protocol::protocol::SandboxPolicy::ReadOnly {
                    network_access: *network_access,
                }
            }
            SandboxPolicy::ExternalSandbox { network_access } => {
                codex_protocol::protocol::SandboxPolicy::ExternalSandbox {
                    network_access: match network_access {
                        NetworkAccess::Restricted => CoreNetworkAccess::Restricted,
                        NetworkAccess::Enabled => CoreNetworkAccess::Enabled,
                    },
                }
            }
            SandboxPolicy::WorkspaceWrite {
                writable_roots,
                network_access,
                exclude_tmpdir_env_var,
                exclude_slash_tmp,
            } => codex_protocol::protocol::SandboxPolicy::WorkspaceWrite {
                writable_roots: writable_roots.clone(),
                network_access: *network_access,
                exclude_tmpdir_env_var: *exclude_tmpdir_env_var,
                exclude_slash_tmp: *exclude_slash_tmp,
            },
        }
    }
}

impl From<codex_protocol::protocol::SandboxPolicy> for SandboxPolicy {
    fn from(value: codex_protocol::protocol::SandboxPolicy) -> Self {
        match value {
            codex_protocol::protocol::SandboxPolicy::DangerFullAccess => {
                SandboxPolicy::DangerFullAccess
            }
            codex_protocol::protocol::SandboxPolicy::ReadOnly { network_access } => {
                SandboxPolicy::ReadOnly { network_access }
            }
            codex_protocol::protocol::SandboxPolicy::ExternalSandbox { network_access } => {
                SandboxPolicy::ExternalSandbox {
                    network_access: match network_access {
                        CoreNetworkAccess::Restricted => NetworkAccess::Restricted,
                        CoreNetworkAccess::Enabled => NetworkAccess::Enabled,
                    },
                }
            }
            codex_protocol::protocol::SandboxPolicy::WorkspaceWrite {
                writable_roots,
                network_access,
                exclude_tmpdir_env_var,
                exclude_slash_tmp,
            } => SandboxPolicy::WorkspaceWrite {
                writable_roots,
                network_access,
                exclude_tmpdir_env_var,
                exclude_slash_tmp,
            },
        }
    }
}

fn absolute_path_json(path: &AbsolutePathBuf) -> Value {
    Value::from(path.as_path().display().to_string())
}

fn absolute_paths_option_json(paths: Option<&[AbsolutePathBuf]>) -> Value {
    paths
        .map(|paths| Value::Array(paths.iter().map(absolute_path_json).collect()))
        .unwrap_or(Value::Null)
}

fn optional_absolute_path(value: Option<Value>) -> Result<Option<AbsolutePathBuf>, JsonValueError> {
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
    value: Option<Value>,
    field: &str,
) -> Result<AbsolutePathBuf, JsonValueError> {
    optional_absolute_path(value)?
        .ok_or_else(|| JsonValueError::WrongType(format!("missing required field `{field}`")))
}

fn optional_absolute_paths(value: Option<Value>) -> Result<Option<Vec<AbsolutePathBuf>>, JsonValueError> {
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

fn optional_nonzero_usize(value: Option<Value>) -> Result<Option<NonZeroUsize>, JsonValueError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let value = usize::from_json(value)?;
    NonZeroUsize::new(value)
        .map(Some)
        .ok_or_else(|| JsonValueError::WrongType("expected non-zero usize".to_string()))
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(transparent)]
#[ts(type = "Array<string>", export_to = "v2/")]
pub struct ExecPolicyAmendment {
    pub command: Vec<String>,
}

impl ExecPolicyAmendment {
    pub fn into_core(self) -> CoreExecPolicyAmendment {
        CoreExecPolicyAmendment::new(self.command)
    }
}

impl From<CoreExecPolicyAmendment> for ExecPolicyAmendment {
    fn from(value: CoreExecPolicyAmendment) -> Self {
        Self {
            command: value.command().to_vec(),
        }
    }
}

v2_enum_from_core!(
    pub enum NetworkPolicyRuleAction from CoreNetworkPolicyRuleAction {
        Allow, Deny
    }
);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct NetworkPolicyAmendment {
    pub host: String,
    pub action: NetworkPolicyRuleAction,
}

impl NetworkPolicyAmendment {
    pub fn into_core(self) -> CoreNetworkPolicyAmendment {
        CoreNetworkPolicyAmendment {
            host: self.host,
            action: self.action.to_core(),
        }
    }
}

impl From<CoreNetworkPolicyAmendment> for NetworkPolicyAmendment {
    fn from(value: CoreNetworkPolicyAmendment) -> Self {
        Self {
            host: value.host,
            action: NetworkPolicyRuleAction::from(value.action),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct PermissionsRequestApprovalParams {
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    /// Unix timestamp (in milliseconds) when this approval request started.
    #[ts(type = "number")]
    pub started_at_ms: i64,
    pub cwd: AbsolutePathBuf,
    pub reason: Option<String>,
    pub permissions: RequestPermissionProfile,
}

v2_enum_from_core!(
    #[derive(Default)]
    pub enum PermissionGrantScope from CorePermissionGrantScope {
        #[default]
        Turn,
        Session
    }
);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct PermissionsRequestApprovalResponse {
    pub permissions: GrantedPermissionProfile,
    #[serde(default)]
    pub scope: PermissionGrantScope,
    /// Review every subsequent command in this turn before normal sandboxed execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub strict_auto_review: Option<bool>,
}
