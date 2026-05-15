use edgerun_json::FromJson;
use edgerun_json::JsonValueError;
use edgerun_json::Map;
use edgerun_json::ToJson;
use edgerun_json::Value;
use schemars::JsonSchema;
use std::collections::HashMap;
use std::path::PathBuf;

/// PTY size in character cells for `command/exec` PTY sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
#[schemars(rename_all = "camelCase")]
pub struct CommandExecTerminalSize {
    /// Terminal height in character cells.
    pub rows: u16,
    /// Terminal width in character cells.
    pub cols: u16,
}

impl ToJson for CommandExecTerminalSize {
    fn to_json(&self) -> Value {
        object([("rows", self.rows.to_json()), ("cols", self.cols.to_json())])
    }
}

impl FromJson for CommandExecTerminalSize {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("CommandExecTerminalSize")?;
        Ok(Self {
            rows: object.take_required("rows")?,
            cols: object.take_required("cols")?,
        })
    }
}

/// Run a standalone command (argv vector) in the server sandbox without
/// creating a thread or turn.
///
/// The final `command/exec` response is deferred until the process exits and is
/// sent only after all `command/exec/outputDelta` notifications for that
/// connection have been emitted.
#[derive(Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename_all = "camelCase")]
pub struct CommandExecParams {
    /// Command argv vector. Empty arrays are rejected.
    pub command: Vec<String>,
    /// Optional client-supplied, connection-scoped process id.
    ///
    /// Required for `tty`, `streamStdin`, `streamStdoutStderr`, and follow-up
    /// `command/exec/write`, `command/exec/resize`, and
    /// `command/exec/terminate` calls. When omitted, buffered execution gets an
    /// internal id that is not exposed to the client.
    pub process_id: Option<String>,
    /// Enable PTY mode.
    ///
    /// This implies `streamStdin` and `streamStdoutStderr`.
    #[schemars(default, skip_serializing_if = "std::ops::Not::not")]
    pub tty: bool,
    /// Allow follow-up `command/exec/write` requests to write stdin bytes.
    ///
    /// Requires a client-supplied `processId`.
    #[schemars(default, skip_serializing_if = "std::ops::Not::not")]
    pub stream_stdin: bool,
    /// Stream stdout/stderr via `command/exec/outputDelta` notifications.
    ///
    /// Streamed bytes are not duplicated into the final response and require a
    /// client-supplied `processId`.
    #[schemars(default, skip_serializing_if = "std::ops::Not::not")]
    pub stream_stdout_stderr: bool,
    /// Optional per-stream stdout/stderr capture cap in bytes.
    ///
    /// When omitted, the server default applies. Cannot be combined with
    /// `disableOutputCap`.
    pub output_bytes_cap: Option<usize>,
    /// Disable stdout/stderr capture truncation for this request.
    ///
    /// Cannot be combined with `outputBytesCap`.
    #[schemars(default, skip_serializing_if = "std::ops::Not::not")]
    pub disable_output_cap: bool,
    /// Disable the timeout entirely for this request.
    ///
    /// Cannot be combined with `timeoutMs`.
    #[schemars(default, skip_serializing_if = "std::ops::Not::not")]
    pub disable_timeout: bool,
    /// Optional timeout in milliseconds.
    ///
    /// When omitted, the server default applies. Cannot be combined with
    /// `disableTimeout`.
    pub timeout_ms: Option<i64>,
    /// Optional working directory. Defaults to the server cwd.
    pub cwd: Option<PathBuf>,
    /// Optional environment overrides merged into the server-computed
    /// environment.
    ///
    /// Matching names override inherited values. Set a key to `null` to unset
    /// an inherited variable.
    pub env: Option<HashMap<String, Option<String>>>,
    /// Optional initial PTY size in character cells. Only valid when `tty` is
    /// true.
    pub size: Option<CommandExecTerminalSize>,
}

impl ToJson for CommandExecParams {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("command", self.command.to_json());
        if let Some(process_id) = &self.process_id {
            object.push_field("processId", process_id.clone());
        }
        if self.tty {
            object.push_field("tty", true);
        }
        if self.stream_stdin {
            object.push_field("streamStdin", true);
        }
        if self.stream_stdout_stderr {
            object.push_field("streamStdoutStderr", true);
        }
        object.push_field("outputBytesCap", self.output_bytes_cap.to_json());
        if self.disable_output_cap {
            object.push_field("disableOutputCap", true);
        }
        if self.disable_timeout {
            object.push_field("disableTimeout", true);
        }
        object.push_field("timeoutMs", self.timeout_ms.to_json());
        object.push_field(
            "cwd",
            self.cwd
                .as_ref()
                .map(|path| Value::from(path.display().to_string()))
                .unwrap_or(Value::Null),
        );
        object.push_field("env", self.env.to_json());
        object.push_field("size", self.size.to_json());
        Value::Object(object)
    }
}

impl FromJson for CommandExecParams {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("CommandExecParams")?;
        Ok(Self {
            command: object.take_required("command")?,
            process_id: object.take_optional("processId")?,
            tty: object.take_optional("tty")?.unwrap_or(false),
            stream_stdin: object.take_optional("streamStdin")?.unwrap_or(false),
            stream_stdout_stderr: object.take_optional("streamStdoutStderr")?.unwrap_or(false),
            output_bytes_cap: object.take_optional("outputBytesCap")?,
            disable_output_cap: object.take_optional("disableOutputCap")?.unwrap_or(false),
            disable_timeout: object.take_optional("disableTimeout")?.unwrap_or(false),
            timeout_ms: object.take_optional("timeoutMs")?,
            cwd: object.take_optional::<String>("cwd")?.map(PathBuf::from),
            env: object.take_optional("env")?,
            size: object.take_optional("size")?,
        })
    }
}

/// Final buffered result for `command/exec`.
#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct CommandExecResponse {
    /// Process exit code.
    pub exit_code: i32,
    /// Buffered stdout capture.
    ///
    /// Empty when stdout was streamed via `command/exec/outputDelta`.
    pub stdout: String,
    /// Buffered stderr capture.
    ///
    /// Empty when stderr was streamed via `command/exec/outputDelta`.
    pub stderr: String,
}

/// Write stdin bytes to a running `command/exec` session, close stdin, or
/// both.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[schemars(rename_all = "camelCase")]
pub struct CommandExecWriteParams {
    /// Client-supplied, connection-scoped `processId` from the original
    /// `command/exec` request.
    pub process_id: String,
    /// Optional base64-encoded stdin bytes to write.
    pub delta_base64: Option<String>,
    /// Close stdin after writing `deltaBase64`, if present.
    #[schemars(default, skip_serializing_if = "std::ops::Not::not")]
    pub close_stdin: bool,
}

impl ToJson for CommandExecWriteParams {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.push_field("processId", &self.process_id);
        object.push_field("deltaBase64", self.delta_base64.to_json());
        if self.close_stdin {
            object.push_field("closeStdin", true);
        }
        Value::Object(object)
    }
}

impl FromJson for CommandExecWriteParams {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("CommandExecWriteParams")?;
        Ok(Self {
            process_id: object.take_required("processId")?,
            delta_base64: object.take_optional("deltaBase64")?,
            close_stdin: object.take_optional("closeStdin")?.unwrap_or(false),
        })
    }
}

/// Empty success response for `command/exec/write`.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct CommandExecWriteResponse {}

/// Terminate a running `command/exec` session.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[schemars(rename_all = "camelCase")]
pub struct CommandExecTerminateParams {
    /// Client-supplied, connection-scoped `processId` from the original
    /// `command/exec` request.
    pub process_id: String,
}

impl ToJson for CommandExecTerminateParams {
    fn to_json(&self) -> Value {
        object([("processId", self.process_id.to_json())])
    }
}

impl FromJson for CommandExecTerminateParams {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("CommandExecTerminateParams")?;
        Ok(Self {
            process_id: object.take_required("processId")?,
        })
    }
}

/// Empty success response for `command/exec/terminate`.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct CommandExecTerminateResponse {}

/// Resize a running PTY-backed `command/exec` session.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[schemars(rename_all = "camelCase")]
pub struct CommandExecResizeParams {
    /// Client-supplied, connection-scoped `processId` from the original
    /// `command/exec` request.
    pub process_id: String,
    /// New PTY size in character cells.
    pub size: CommandExecTerminalSize,
}

impl ToJson for CommandExecResizeParams {
    fn to_json(&self) -> Value {
        object([
            ("processId", self.process_id.to_json()),
            ("size", self.size.to_json()),
        ])
    }
}

impl FromJson for CommandExecResizeParams {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("CommandExecResizeParams")?;
        Ok(Self {
            process_id: object.take_required("processId")?,
            size: object.take_required("size")?,
        })
    }
}

/// Empty success response for `command/exec/resize`.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct CommandExecResizeResponse {}

/// Stream label for `command/exec/outputDelta` notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
#[schemars(rename_all = "camelCase")]
pub enum CommandExecOutputStream {
    /// stdout stream. PTY mode multiplexes terminal output here.
    Stdout,
    /// stderr stream.
    Stderr,
}

impl ToJson for CommandExecOutputStream {
    fn to_json(&self) -> Value {
        match self {
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
        }
        .to_json()
    }
}

impl FromJson for CommandExecOutputStream {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        match String::from_json(value)?.as_str() {
            "stdout" => Ok(Self::Stdout),
            "stderr" => Ok(Self::Stderr),
            other => Err(JsonValueError::WrongType(format!(
                "unknown command exec output stream `{other}`"
            ))),
        }
    }
}
/// Base64-encoded output chunk emitted for a streaming `command/exec` request.
///
/// These notifications are connection-scoped. If the originating connection
/// closes, the server terminates the process.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[schemars(rename_all = "camelCase")]
pub struct CommandExecOutputDeltaNotification {
    /// Client-supplied, connection-scoped `processId` from the original
    /// `command/exec` request.
    pub process_id: String,
    /// Output stream for this chunk.
    pub stream: CommandExecOutputStream,
    /// Base64-encoded output bytes.
    pub delta_base64: String,
    /// `true` on the final streamed chunk for a stream when `outputBytesCap`
    /// truncated later output on that stream.
    pub cap_reached: bool,
}

impl ToJson for CommandExecOutputDeltaNotification {
    fn to_json(&self) -> Value {
        object([
            ("processId", self.process_id.to_json()),
            ("stream", self.stream.to_json()),
            ("deltaBase64", self.delta_base64.to_json()),
            ("capReached", self.cap_reached.to_json()),
        ])
    }
}

impl FromJson for CommandExecOutputDeltaNotification {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("CommandExecOutputDeltaNotification")?;
        Ok(Self {
            process_id: object.take_required("processId")?,
            stream: object.take_required("stream")?,
            delta_base64: object.take_required("deltaBase64")?,
            cap_reached: object.take_required("capReached")?,
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
