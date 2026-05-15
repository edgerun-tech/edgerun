use super::shared::v2_enum_from_core;
use codex_protocol::approvals::ExecPolicyAmendment as CoreExecPolicyAmendment;
use codex_protocol::approvals::NetworkApprovalContext as CoreNetworkApprovalContext;
use codex_protocol::approvals::NetworkApprovalProtocol as CoreNetworkApprovalProtocol;
use codex_protocol::approvals::NetworkPolicyAmendment as CoreNetworkPolicyAmendment;
use codex_protocol::approvals::NetworkPolicyRuleAction as CoreNetworkPolicyRuleAction;
use codex_protocol::compat::absolute_path::AbsolutePathBuf;
use codex_protocol::models::FileSystemPermissions as CoreFileSystemPermissions;
use codex_protocol::models::NetworkPermissions as CoreNetworkPermissions;
use codex_protocol::permissions::FileSystemAccessMode as CoreFileSystemAccessMode;
use codex_protocol::permissions::FileSystemPath as CoreFileSystemPath;
use codex_protocol::permissions::FileSystemSandboxEntry as CoreFileSystemSandboxEntry;
use codex_protocol::permissions::FileSystemSpecialPath as CoreFileSystemSpecialPath;
use edgerun_json::FromJson;
use edgerun_json::JsonValueError;
use edgerun_json::Map;
use edgerun_json::ToJson;
use edgerun_json::Value;
use schemars::JsonSchema;
use std::num::NonZeroUsize;
use std::path::PathBuf;

v2_enum_from_core! {
    pub enum NetworkApprovalProtocol from CoreNetworkApprovalProtocol {
        Http,
        Https,
        Socks5Tcp,
        Socks5Udp,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson)]
#[schemars(rename_all = "camelCase")]
pub struct NetworkApprovalContext {
    pub host: String,
    pub protocol: NetworkApprovalProtocol,
}

impl FromJson for NetworkApprovalContext {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("NetworkApprovalContext")?;
        Ok(Self {
            host: object.take_required("host")?,
            protocol: object.take_required("protocol")?,
        })
    }
}

impl From<CoreNetworkApprovalContext> for NetworkApprovalContext {
    fn from(value: CoreNetworkApprovalContext) -> Self {
        Self {
            host: value.host,
            protocol: value.protocol.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[schemars(rename_all = "camelCase")]
pub struct AdditionalFileSystemPermissions {
    /// This will be removed in favor of `entries`.
    pub read: Option<Vec<AbsolutePathBuf>>,
    /// This will be removed in favor of `entries`.
    pub write: Option<Vec<AbsolutePathBuf>>,
    #[schemars(default, skip_serializing_if = "Option::is_none")]
    pub glob_scan_max_depth: Option<NonZeroUsize>,
    #[schemars(default, skip_serializing_if = "Option::is_none")]
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

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[schemars(rename_all = "camelCase")]
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

v2_enum_from_core!(
    pub enum FileSystemAccessMode from CoreFileSystemAccessMode {
        Read,
        Write,
        None
    }
);

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[schemars(tag = "kind", rename_all = "snake_case")]
pub enum FileSystemSpecialPath {
    Root,
    Minimal,
    #[schemars(alias = "current_working_directory")]
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

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[schemars(tag = "type", rename_all = "snake_case")]
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

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[schemars(rename_all = "camelCase")]
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

fn optional_absolute_paths(
    value: Option<Value>,
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

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[schemars(transparent)]
pub struct ExecPolicyAmendment {
    pub command: Vec<String>,
}

impl ToJson for ExecPolicyAmendment {
    fn to_json(&self) -> Value {
        self.command.to_json()
    }
}

impl FromJson for ExecPolicyAmendment {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        Ok(Self {
            command: Vec::<String>::from_json(value)?,
        })
    }
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

#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[schemars(rename_all = "camelCase")]
pub struct NetworkPolicyAmendment {
    pub host: String,
    pub action: NetworkPolicyRuleAction,
}

impl ToJson for NetworkPolicyAmendment {
    fn to_json(&self) -> Value {
        let mut object = Map::with_capacity(2);
        object.push_field("host", self.host.clone());
        object.push_field("action", self.action.to_json());
        Value::Object(object)
    }
}

impl FromJson for NetworkPolicyAmendment {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("NetworkPolicyAmendment")?;
        Ok(Self {
            host: object.take_required("host")?,
            action: object.take_required("action")?,
        })
    }
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
