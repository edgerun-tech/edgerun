use std::collections::HashMap;

use codex_protocol::ToolName;
use edgerun_json::Value as JsonValue;

use crate::description::ToolDefinition;
use crate::response::FunctionCallOutputContentItem;

pub const DEFAULT_EXEC_YIELD_TIME_MS: u64 = 10_000;
pub const DEFAULT_WAIT_YIELD_TIME_MS: u64 = 10_000;
pub const DEFAULT_MAX_OUTPUT_TOKENS_PER_EXEC_CALL: usize = 10_000;

#[derive(Clone, Debug)]
pub struct ExecuteRequest {
    /// Runtime cell id for this execution.
    ///
    /// Callers allocate this before execution so tracing, waits, and nested tool
    /// calls can refer to the cell as soon as execution starts.
    pub cell_id: String,
    pub tool_call_id: String,
    pub enabled_tools: Vec<ToolDefinition>,
    pub source: String,
    pub stored_values: HashMap<String, JsonValue>,
    pub yield_time_ms: Option<u64>,
    pub max_output_tokens: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct WaitRequest {
    pub cell_id: String,
    pub yield_time_ms: u64,
    pub terminate: bool,
}

#[derive(Debug, PartialEq)]
pub enum WaitOutcome {
    LiveCell(RuntimeResponse),
    MissingCell(RuntimeResponse),
}

impl From<WaitOutcome> for RuntimeResponse {
    fn from(outcome: WaitOutcome) -> Self {
        match outcome {
            WaitOutcome::LiveCell(response) | WaitOutcome::MissingCell(response) => response,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum RuntimeResponse {
    Yielded {
        cell_id: String,
        content_items: Vec<FunctionCallOutputContentItem>,
    },
    Terminated {
        cell_id: String,
        content_items: Vec<FunctionCallOutputContentItem>,
    },
    Result {
        cell_id: String,
        content_items: Vec<FunctionCallOutputContentItem>,
        stored_values: HashMap<String, JsonValue>,
        error_text: Option<String>,
    },
}

/// Nested tool request emitted by one external code-mode runtime.
///
/// Code mode owns the per-cell runtime id. Hosts should preserve it for
/// provenance/debugging, but should still assign their own runtime tool call id
/// if their tool-call graph requires globally unique ids.
#[derive(Debug)]
pub struct CodeModeNestedToolCall {
    pub cell_id: String,
    pub runtime_tool_call_id: String,
    pub tool_name: ToolName,
    pub input: Option<JsonValue>,
}

#[derive(Debug)]
pub(crate) enum TurnMessage {
    ToolCall(CodeModeNestedToolCall),
    Notify {
        cell_id: String,
        call_id: String,
        text: String,
    },
}

#[derive(Debug)]
pub(crate) enum RuntimeCommand {
    ToolResponse { id: String, result: JsonValue },
    ToolError { id: String, error_text: String },
    Terminate,
}

#[derive(Debug)]
pub(crate) enum RuntimeEvent {
    Started,
    ContentItem(FunctionCallOutputContentItem),
    YieldRequested,
    ToolCall {
        id: String,
        name: ToolName,
        input: Option<JsonValue>,
    },
    Notify {
        call_id: String,
        text: String,
    },
    Result {
        stored_values: HashMap<String, JsonValue>,
        error_text: Option<String>,
    },
}

pub(crate) fn missing_cell_response(cell_id: String) -> RuntimeResponse {
    RuntimeResponse::Result {
        error_text: Some(format!("exec cell {cell_id} not found")),
        cell_id,
        content_items: Vec::new(),
        stored_values: HashMap::new(),
    }
}
