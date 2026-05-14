use codex_protocol::compat::absolute_path::AbsolutePathBuf;
use edgerun_json::FromJson;
use edgerun_json::JsonValueError;
use edgerun_json::Map;
use edgerun_json::ToJson;
use edgerun_json::Value;
use edgerun_serde::Deserialize;
use edgerun_serde::Serialize;
use schemars::JsonSchema;
use std::collections::HashMap;
use ts_rs::TS;

/// PTY size in character cells for `process/spawn` PTY sessions.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProcessTerminalSize {
    /// Terminal height in character cells.
    pub rows: u16,
    /// Terminal width in character cells.
    pub cols: u16,
}

impl ToJson for ProcessTerminalSize {
    fn to_json(&self) -> Value {
        object([("rows", self.rows.to_json()), ("cols", self.cols.to_json())])
    }
}

impl FromJson for ProcessTerminalSize {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("ProcessTerminalSize")?;
        Ok(Self {
            rows: object.take_required("rows")?,
            cols: object.take_required("cols")?,
        })
    }
}

/// Spawn a standalone process (argv vector) without a Codex sandbox on the host
/// where the app server is running.
///
/// `process/spawn` returns after the process has started and the connection-scoped
/// `processHandle` has been registered. Process output and exit are reported via
/// `process/outputDelta` and `process/exited` notifications.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProcessSpawnParams {
    /// Command argv vector. Empty arrays are rejected.
    pub command: Vec<String>,
    /// Client-supplied, connection-scoped process handle.
    ///
    /// Duplicate active handles are rejected on the same connection. The same
    /// handle can be reused after the prior process exits.
    pub process_handle: String,
    /// Absolute working directory for the process.
    pub cwd: AbsolutePathBuf,
    /// Enable PTY mode.
    ///
    /// This implies `streamStdin` and `streamStdoutStderr`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub tty: bool,
    /// Allow follow-up `process/writeStdin` requests to write stdin bytes.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stream_stdin: bool,
    /// Stream stdout/stderr via `process/outputDelta` notifications.
    ///
    /// Streamed bytes are not duplicated into the `process/exited` notification.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stream_stdout_stderr: bool,
    /// Optional per-stream stdout/stderr capture cap in bytes.
    ///
    /// When omitted, the server default applies. Set to `null` to disable the
    /// cap.
    #[serde(
        default,
        deserialize_with = "crate::protocol::serde_helpers::deserialize_double_option",
        serialize_with = "crate::protocol::serde_helpers::serialize_double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[ts(type = "number | null")]
    #[ts(optional = nullable)]
    pub output_bytes_cap: Option<Option<usize>>,
    /// Optional timeout in milliseconds.
    ///
    /// When omitted, the server default applies. Set to `null` to disable the
    /// timeout.
    #[serde(
        default,
        deserialize_with = "crate::protocol::serde_helpers::deserialize_double_option",
        serialize_with = "crate::protocol::serde_helpers::serialize_double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[ts(type = "number | null")]
    #[ts(optional = nullable)]
    pub timeout_ms: Option<Option<i64>>,
    /// Optional environment overrides merged into the app-server process
    /// environment.
    ///
    /// Matching names override inherited values. Set a key to `null` to unset
    /// an inherited variable.
    #[ts(optional = nullable)]
    pub env: Option<HashMap<String, Option<String>>>,
    /// Optional initial PTY size in character cells. Only valid when `tty` is
    /// true.
    #[ts(optional = nullable)]
    pub size: Option<ProcessTerminalSize>,
}

impl ToJson for ProcessSpawnParams {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("command", self.command.to_json());
        object.push_field("processHandle", &self.process_handle);
        object.push_field("cwd", self.cwd.to_json());
        if self.tty {
            object.push_field("tty", true);
        }
        if self.stream_stdin {
            object.push_field("streamStdin", true);
        }
        if self.stream_stdout_stderr {
            object.push_field("streamStdoutStderr", true);
        }
        if let Some(output_bytes_cap) = self.output_bytes_cap {
            object.push_field("outputBytesCap", output_bytes_cap.to_json());
        }
        if let Some(timeout_ms) = self.timeout_ms {
            object.push_field("timeoutMs", timeout_ms.to_json());
        }
        object.push_field("env", self.env.to_json());
        object.push_field("size", self.size.to_json());
        Value::Object(object)
    }
}

impl FromJson for ProcessSpawnParams {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("ProcessSpawnParams")?;
        Ok(Self {
            command: object.take_required("command")?,
            process_handle: object.take_required("processHandle")?,
            cwd: object.take_required("cwd")?,
            tty: object.take_optional("tty")?.unwrap_or(false),
            stream_stdin: object.take_optional("streamStdin")?.unwrap_or(false),
            stream_stdout_stderr: object.take_optional("streamStdoutStderr")?.unwrap_or(false),
            output_bytes_cap: take_double_option(&mut object, "outputBytesCap")?,
            timeout_ms: take_double_option(&mut object, "timeoutMs")?,
            env: object.take_optional("env")?,
            size: object.take_optional("size")?,
        })
    }
}

/// Successful response for `process/spawn`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProcessSpawnResponse {}

/// Write stdin bytes to a running `process/spawn` session, close stdin, or
/// both.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProcessWriteStdinParams {
    /// Client-supplied, connection-scoped `processHandle` from `process/spawn`.
    pub process_handle: String,
    /// Optional base64-encoded stdin bytes to write.
    #[ts(optional = nullable)]
    pub delta_base64: Option<String>,
    /// Close stdin after writing `deltaBase64`, if present.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub close_stdin: bool,
}

impl ToJson for ProcessWriteStdinParams {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("processHandle", &self.process_handle);
        object.push_field("deltaBase64", self.delta_base64.to_json());
        if self.close_stdin {
            object.push_field("closeStdin", true);
        }
        Value::Object(object)
    }
}

impl FromJson for ProcessWriteStdinParams {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("ProcessWriteStdinParams")?;
        Ok(Self {
            process_handle: object.take_required("processHandle")?,
            delta_base64: object.take_optional("deltaBase64")?,
            close_stdin: object.take_optional("closeStdin")?.unwrap_or(false),
        })
    }
}

/// Empty success response for `process/writeStdin`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProcessWriteStdinResponse {}

/// Terminate a running `process/spawn` session.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProcessKillParams {
    /// Client-supplied, connection-scoped `processHandle` from `process/spawn`.
    pub process_handle: String,
}

impl ToJson for ProcessKillParams {
    fn to_json(&self) -> Value {
        object([("processHandle", self.process_handle.to_json())])
    }
}

impl FromJson for ProcessKillParams {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("ProcessKillParams")?;
        Ok(Self {
            process_handle: object.take_required("processHandle")?,
        })
    }
}

/// Empty success response for `process/kill`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProcessKillResponse {}

/// Resize a running PTY-backed `process/spawn` session.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProcessResizePtyParams {
    /// Client-supplied, connection-scoped `processHandle` from `process/spawn`.
    pub process_handle: String,
    /// New PTY size in character cells.
    pub size: ProcessTerminalSize,
}

impl ToJson for ProcessResizePtyParams {
    fn to_json(&self) -> Value {
        object([
            ("processHandle", self.process_handle.to_json()),
            ("size", self.size.to_json()),
        ])
    }
}

impl FromJson for ProcessResizePtyParams {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("ProcessResizePtyParams")?;
        Ok(Self {
            process_handle: object.take_required("processHandle")?,
            size: object.take_required("size")?,
        })
    }
}

/// Empty success response for `process/resizePty`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProcessResizePtyResponse {}

/// Stream label for `process/outputDelta` notifications.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum ProcessOutputStream {
    /// stdout stream. PTY mode multiplexes terminal output here.
    Stdout,
    /// stderr stream.
    Stderr,
}

impl ToJson for ProcessOutputStream {
    fn to_json(&self) -> Value {
        match self {
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
        }
        .to_json()
    }
}

impl FromJson for ProcessOutputStream {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "stdout" => Ok(Self::Stdout),
            "stderr" => Ok(Self::Stderr),
            other => Err(JsonValueError::WrongType(format!(
                "unknown process output stream `{other}`"
            ))),
        }
    }
}

/// Base64-encoded output chunk emitted for a streaming `process/spawn` request.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProcessOutputDeltaNotification {
    /// Client-supplied, connection-scoped `processHandle` from `process/spawn`.
    pub process_handle: String,
    /// Output stream this chunk belongs to.
    pub stream: ProcessOutputStream,
    /// Base64-encoded output bytes.
    pub delta_base64: String,
    /// True on the final streamed chunk for this stream when output was
    /// truncated by `outputBytesCap`.
    pub cap_reached: bool,
}

impl ToJson for ProcessOutputDeltaNotification {
    fn to_json(&self) -> Value {
        object([
            ("processHandle", self.process_handle.to_json()),
            ("stream", self.stream.to_json()),
            ("deltaBase64", self.delta_base64.to_json()),
            ("capReached", self.cap_reached.to_json()),
        ])
    }
}

impl FromJson for ProcessOutputDeltaNotification {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("ProcessOutputDeltaNotification")?;
        Ok(Self {
            process_handle: object.take_required("processHandle")?,
            stream: object.take_required("stream")?,
            delta_base64: object.take_required("deltaBase64")?,
            cap_reached: object.take_required("capReached")?,
        })
    }
}

/// Final process exit notification for `process/spawn`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ProcessExitedNotification {
    /// Client-supplied, connection-scoped `processHandle` from `process/spawn`.
    pub process_handle: String,
    /// Process exit code.
    pub exit_code: i32,
    /// Buffered stdout capture.
    ///
    /// Empty when stdout was streamed via `process/outputDelta`.
    pub stdout: String,
    /// Whether stdout reached `outputBytesCap`.
    ///
    /// In streaming mode, stdout is empty and cap state is also reported on the
    /// final stdout `process/outputDelta` notification.
    pub stdout_cap_reached: bool,
    /// Buffered stderr capture.
    ///
    /// Empty when stderr was streamed via `process/outputDelta`.
    pub stderr: String,
    /// Whether stderr reached `outputBytesCap`.
    ///
    /// In streaming mode, stderr is empty and cap state is also reported on the
    /// final stderr `process/outputDelta` notification.
    pub stderr_cap_reached: bool,
}

impl ToJson for ProcessExitedNotification {
    fn to_json(&self) -> Value {
        object([
            ("processHandle", self.process_handle.to_json()),
            ("exitCode", self.exit_code.to_json()),
            ("stdout", self.stdout.to_json()),
            ("stdoutCapReached", self.stdout_cap_reached.to_json()),
            ("stderr", self.stderr.to_json()),
            ("stderrCapReached", self.stderr_cap_reached.to_json()),
        ])
    }
}

impl FromJson for ProcessExitedNotification {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("ProcessExitedNotification")?;
        Ok(Self {
            process_handle: object.take_required("processHandle")?,
            exit_code: object.take_required("exitCode")?,
            stdout: object.take_required("stdout")?,
            stdout_cap_reached: object.take_required("stdoutCapReached")?,
            stderr: object.take_required("stderr")?,
            stderr_cap_reached: object.take_required("stderrCapReached")?,
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

fn take_double_option<T: FromJson>(
    object: &mut Map,
    field: &str,
) -> Result<Option<Option<T>>, JsonValueError> {
    match object.remove(field) {
        Some(Value::Null) => Ok(Some(None)),
        Some(value) => T::from_json(value).map(Some).map(Some),
        None => Ok(None),
    }
}
