use crate::function_tool::FunctionCallError;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use edgerun_json::FromJson;
use edgerun_json::JsonValueError;
use edgerun_json::Value;

use super::DEFAULT_WAIT_YIELD_TIME_MS;
use super::ExecContext;
use super::WAIT_TOOL_NAME;
use super::handle_runtime_response;
use super::wait_spec::create_wait_tool;

pub struct CodeModeWaitHandler;

#[derive(Debug)]
struct ExecWaitArgs {
    cell_id: String,
    yield_time_ms: u64,
    max_tokens: Option<usize>,
    terminate: bool,
}

impl FromJson for ExecWaitArgs {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("ExecWaitArgs")?;
        Ok(Self {
            cell_id: object.take_required("cell_id")?,
            yield_time_ms: object
                .take_optional("yield_time_ms")?
                .unwrap_or(DEFAULT_WAIT_YIELD_TIME_MS),
            max_tokens: object.take_optional("max_tokens")?,
            terminate: object.take_optional("terminate")?.unwrap_or(false),
        })
    }
}

impl ToolHandler for CodeModeWaitHandler {
    type Output = FunctionToolOutput;

    fn tool_name(&self) -> ToolName {
        ToolName::plain(WAIT_TOOL_NAME)
    }

    fn spec(&self) -> Option<ToolSpec> {
        Some(create_wait_tool())
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let ToolInvocation {
            session,
            turn,
            tool_name,
            payload,
            ..
        } = invocation;

        match payload {
            ToolPayload::Function { arguments }
                if tool_name.namespace.is_none() && tool_name.name.as_str() == WAIT_TOOL_NAME =>
            {
                let args: ExecWaitArgs =
                    edgerun_json::from_json_str(&arguments).map_err(|err| {
                        FunctionCallError::RespondToModel(format!(
                            "failed to parse function arguments: {err}"
                        ))
                    })?;
                let exec = ExecContext { session, turn };
                let started_at = std::time::Instant::now();
                let wait_response = exec
                    .session
                    .services
                    .code_mode_service
                    .wait(codex_code_mode::WaitRequest {
                        cell_id: args.cell_id,
                        yield_time_ms: args.yield_time_ms,
                        terminate: args.terminate,
                    })
                    .await
                    .map_err(FunctionCallError::RespondToModel)?;
                if let codex_code_mode::WaitOutcome::LiveCell(response) = &wait_response
                    && !matches!(response, codex_code_mode::RuntimeResponse::Yielded { .. })
                {
                    // Only a live-cell wait can close a CodeCell. A missing
                    // cell is still an ordinary `wait` tool result, but there
                    // is no runtime object for the reducer to complete.
                    let runtime_cell_id = match response {
                        codex_code_mode::RuntimeResponse::Yielded { cell_id, .. }
                        | codex_code_mode::RuntimeResponse::Terminated { cell_id, .. }
                        | codex_code_mode::RuntimeResponse::Result { cell_id, .. } => cell_id,
                    };
                    exec.session
                        .services
                        .rollout_thread_trace
                        .code_cell_trace_context(exec.turn.sub_id.as_str(), runtime_cell_id)
                        .record_ended(response);
                }
                handle_runtime_response(&exec, wait_response.into(), args.max_tokens, started_at)
                    .await
                    .map_err(FunctionCallError::RespondToModel)
            }
            _ => Err(FunctionCallError::RespondToModel(format!(
                "{WAIT_TOOL_NAME} expects JSON arguments"
            ))),
        }
    }
}
