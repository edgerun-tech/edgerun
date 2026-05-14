use codex_protocol::compat::absolute_path::AbsolutePathBuf;
use edgerun_json::FromJson;
use edgerun_json::JsonValueError;
use edgerun_json::Map;
use edgerun_json::ToJson;
use edgerun_json::Value;
use schemars::JsonSchema;

/// Read a file from the host filesystem.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FsReadFileParams {
    /// Absolute path to read.
    pub path: AbsolutePathBuf,
}

impl ToJson for FsReadFileParams {
    fn to_json(&self) -> Value {
        object([("path", self.path.to_json())])
    }
}

impl FromJson for FsReadFileParams {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("FsReadFileParams")?;
        Ok(Self {
            path: object.take_required("path")?,
        })
    }
}

/// Base64-encoded file contents returned by `fs/readFile`.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FsReadFileResponse {
    /// File contents encoded as base64.
    pub data_base64: String,
}

impl ToJson for FsReadFileResponse {
    fn to_json(&self) -> Value {
        object([("dataBase64", self.data_base64.to_json())])
    }
}

impl FromJson for FsReadFileResponse {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("FsReadFileResponse")?;
        Ok(Self {
            data_base64: object.take_required("dataBase64")?,
        })
    }
}

/// Write a file on the host filesystem.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FsWriteFileParams {
    /// Absolute path to write.
    pub path: AbsolutePathBuf,
    /// File contents encoded as base64.
    pub data_base64: String,
}

impl ToJson for FsWriteFileParams {
    fn to_json(&self) -> Value {
        object([
            ("path", self.path.to_json()),
            ("dataBase64", self.data_base64.to_json()),
        ])
    }
}

impl FromJson for FsWriteFileParams {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("FsWriteFileParams")?;
        Ok(Self {
            path: object.take_required("path")?,
            data_base64: object.take_required("dataBase64")?,
        })
    }
}

/// Successful response for `fs/writeFile`.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FsWriteFileResponse {}

impl ToJson for FsWriteFileResponse {
    fn to_json(&self) -> Value {
        Value::Object(Map::new())
    }
}

impl FromJson for FsWriteFileResponse {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let _ = value.into_object("FsWriteFileResponse")?;
        Ok(Self {})
    }
}

/// Create a directory on the host filesystem.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FsCreateDirectoryParams {
    /// Absolute directory path to create.
    pub path: AbsolutePathBuf,
    /// Whether parent directories should also be created. Defaults to `true`.
    pub recursive: Option<bool>,
}

impl ToJson for FsCreateDirectoryParams {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("path", self.path.to_json());
        object.push_field("recursive", self.recursive.to_json());
        Value::Object(object)
    }
}

impl FromJson for FsCreateDirectoryParams {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("FsCreateDirectoryParams")?;
        Ok(Self {
            path: object.take_required("path")?,
            recursive: object.take_optional("recursive")?,
        })
    }
}

/// Successful response for `fs/createDirectory`.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FsCreateDirectoryResponse {}

impl ToJson for FsCreateDirectoryResponse {
    fn to_json(&self) -> Value {
        Value::Object(Map::new())
    }
}

impl FromJson for FsCreateDirectoryResponse {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let _ = value.into_object("FsCreateDirectoryResponse")?;
        Ok(Self {})
    }
}

/// Request metadata for an absolute path.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FsGetMetadataParams {
    /// Absolute path to inspect.
    pub path: AbsolutePathBuf,
}

impl ToJson for FsGetMetadataParams {
    fn to_json(&self) -> Value {
        object([("path", self.path.to_json())])
    }
}

impl FromJson for FsGetMetadataParams {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("FsGetMetadataParams")?;
        Ok(Self {
            path: object.take_required("path")?,
        })
    }
}

/// Metadata returned by `fs/getMetadata`.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FsGetMetadataResponse {
    /// Whether the path resolves to a directory.
    pub is_directory: bool,
    /// Whether the path resolves to a regular file.
    pub is_file: bool,
    /// Whether the path itself is a symbolic link.
    pub is_symlink: bool,
    /// File creation time in Unix milliseconds when available, otherwise `0`.
    pub created_at_ms: i64,
    /// File modification time in Unix milliseconds when available, otherwise `0`.
    pub modified_at_ms: i64,
}

impl ToJson for FsGetMetadataResponse {
    fn to_json(&self) -> Value {
        object([
            ("isDirectory", self.is_directory.to_json()),
            ("isFile", self.is_file.to_json()),
            ("isSymlink", self.is_symlink.to_json()),
            ("createdAtMs", self.created_at_ms.to_json()),
            ("modifiedAtMs", self.modified_at_ms.to_json()),
        ])
    }
}

impl FromJson for FsGetMetadataResponse {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("FsGetMetadataResponse")?;
        Ok(Self {
            is_directory: object.take_required("isDirectory")?,
            is_file: object.take_required("isFile")?,
            is_symlink: object.take_required("isSymlink")?,
            created_at_ms: object.take_required("createdAtMs")?,
            modified_at_ms: object.take_required("modifiedAtMs")?,
        })
    }
}

/// List direct child names for a directory.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct FsReadDirectoryParams {
    /// Absolute directory path to read.
    pub path: AbsolutePathBuf,
}

/// A directory entry returned by `fs/readDirectory`.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct FsReadDirectoryEntry {
    /// Direct child entry name only, not an absolute or relative path.
    pub file_name: String,
    /// Whether this entry resolves to a directory.
    pub is_directory: bool,
    /// Whether this entry resolves to a regular file.
    pub is_file: bool,
}

/// Directory entries returned by `fs/readDirectory`.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct FsReadDirectoryResponse {
    /// Direct child entries in the requested directory.
    pub entries: Vec<FsReadDirectoryEntry>,
}

/// Remove a file or directory tree from the host filesystem.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct FsRemoveParams {
    /// Absolute path to remove.
    pub path: AbsolutePathBuf,
    /// Whether directory removal should recurse. Defaults to `true`.
    pub recursive: Option<bool>,
    /// Whether missing paths should be ignored. Defaults to `true`.
    pub force: Option<bool>,
}

/// Successful response for `fs/remove`.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct FsRemoveResponse {}

/// Copy a file or directory tree on the host filesystem.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FsCopyParams {
    /// Absolute source path.
    pub source_path: AbsolutePathBuf,
    /// Absolute destination path.
    pub destination_path: AbsolutePathBuf,
    /// Required for directory copies; ignored for file copies.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub recursive: bool,
}

impl ToJson for FsCopyParams {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("sourcePath", self.source_path.to_json());
        object.push_field("destinationPath", self.destination_path.to_json());
        if self.recursive {
            object.push_field("recursive", true);
        }
        Value::Object(object)
    }
}

impl FromJson for FsCopyParams {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("FsCopyParams")?;
        Ok(Self {
            source_path: object.take_required("sourcePath")?,
            destination_path: object.take_required("destinationPath")?,
            recursive: object.take_optional("recursive")?.unwrap_or(false),
        })
    }
}

/// Successful response for `fs/copy`.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct FsCopyResponse {}

/// Start filesystem watch notifications for an absolute path.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct FsWatchParams {
    /// Connection-scoped watch identifier used for `fs/unwatch` and `fs/changed`.
    pub watch_id: String,
    /// Absolute file or directory path to watch.
    pub path: AbsolutePathBuf,
}

/// Successful response for `fs/watch`.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct FsWatchResponse {
    /// Canonicalized path associated with the watch.
    pub path: AbsolutePathBuf,
}

/// Stop filesystem watch notifications for a prior `fs/watch`.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct FsUnwatchParams {
    /// Watch identifier previously provided to `fs/watch`.
    pub watch_id: String,
}

/// Successful response for `fs/unwatch`.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[serde(rename_all = "camelCase")]
pub struct FsUnwatchResponse {}

/// Filesystem watch notification emitted for `fs/watch` subscribers.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FsChangedNotification {
    /// Watch identifier previously provided to `fs/watch`.
    pub watch_id: String,
    /// File or directory paths associated with this event.
    pub changed_paths: Vec<AbsolutePathBuf>,
}

impl ToJson for FsChangedNotification {
    fn to_json(&self) -> Value {
        object([
            ("watchId", self.watch_id.to_json()),
            ("changedPaths", self.changed_paths.to_json()),
        ])
    }
}

impl FromJson for FsChangedNotification {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("FsChangedNotification")?;
        Ok(Self {
            watch_id: object.take_required("watchId")?,
            changed_paths: object.take_required("changedPaths")?,
        })
    }
}

fn object<const N: usize>(fields: [(&str, Value); N]) -> Value {
    let mut object = Map::with_capacity(N);
    for (key, value) in fields {
        object.push_field(key, value);
    }
    Value::Object(object)
}
