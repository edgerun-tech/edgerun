use codex_protocol::models::ShellCommandToolCallParams;
use codex_protocol::models::ShellToolCallParams;
use edgerun_json::Value as JsonValue;
use std::sync::Arc;

use crate::exec::ExecParams;
use crate::exec_policy::ExecApprovalRequest;
use crate::function_tool::FunctionCallError;
use crate::session::turn_context::TurnContext;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::events::ToolEmitter;
use crate::tools::events::ToolEventCtx;
use crate::tools::handlers::apply_patch::intercept_apply_patch;
use crate::tools::handlers::parse_arguments;
use crate::tools::hook_names::HookToolName;
use crate::tools::orchestrator::ToolOrchestrator;
use crate::tools::registry::PostToolUsePayload;
use crate::tools::registry::PreToolUsePayload;
use crate::tools::runtimes::shell::ShellRequest;
use crate::tools::runtimes::shell::ShellRuntime;
use crate::tools::runtimes::shell::ShellRuntimeBackend;
use crate::tools::sandboxing::ToolCtx;
use codex_protocol::protocol::ExecCommandSource;

mod container_exec;
mod local_shell;
mod shell_command;
mod shell_handler;

pub use container_exec::ContainerExecHandler;
pub use local_shell::LocalShellHandler;
pub use shell_command::ShellCommandHandler;
pub(crate) use shell_command::ShellCommandHandlerOptions;
pub use shell_handler::ShellHandler;

fn shell_function_payload_command(payload: &ToolPayload) -> Option<String> {
    let ToolPayload::Function { arguments } = payload else {
        return None;
    };

    shell_command_argv_from_arguments(arguments)
        .map(|command| codex_shell_command::parse_command::shlex_join(&command))
}

fn local_shell_payload_command(payload: &ToolPayload) -> Option<String> {
    let ToolPayload::LocalShell { params } = payload else {
        return None;
    };

    Some(codex_shell_command::parse_command::shlex_join(
        &params.command,
    ))
}

fn shell_command_payload_command(payload: &ToolPayload) -> Option<String> {
    let ToolPayload::Function { arguments } = payload else {
        return None;
    };

    shell_command_string_from_arguments(arguments)
}

fn shell_command_string_from_arguments(arguments: &str) -> Option<String> {
    let tape = edgerun_json::parse_json_tape(arguments).ok()?;
    tape.root(arguments)?
        .get("command")
        .and_then(|command| command.as_str())
        .map(str::to_string)
}

fn shell_command_argv_from_arguments(arguments: &str) -> Option<Vec<String>> {
    let tape = edgerun_json::parse_json_tape(arguments).ok()?;
    let command = tape.root(arguments)?.get_array("command")?;
    command
        .into_iter()
        .map(|item| item.as_str().map(str::to_string))
        .collect()
}

struct RunExecLikeArgs {
    tool_name: String,
    exec_params: ExecParams,
    hook_command: String,
    prefix_rule: Option<Vec<String>>,
    session: Arc<crate::session::session::Session>,
    turn: Arc<TurnContext>,
    tracker: crate::tools::context::SharedTurnDiffTracker,
    call_id: String,
    freeform: bool,
    shell_runtime_backend: ShellRuntimeBackend,
}

fn shell_function_pre_tool_use_payload(invocation: &ToolInvocation) -> Option<PreToolUsePayload> {
    shell_function_payload_command(&invocation.payload).map(|command| PreToolUsePayload {
        tool_name: HookToolName::bash(),
        tool_input: edgerun_json::json!({ "command": command }),
    })
}

fn shell_function_post_tool_use_payload(
    invocation: &ToolInvocation,
    result: &FunctionToolOutput,
) -> Option<PostToolUsePayload> {
    let tool_response = result.post_tool_use_response(&invocation.call_id, &invocation.payload)?;
    let command = shell_function_payload_command(&invocation.payload)?;
    Some(PostToolUsePayload {
        tool_name: HookToolName::bash(),
        tool_use_id: invocation.call_id.clone(),
        tool_input: edgerun_json::json!({ "command": command }),
        tool_response,
    })
}

async fn run_exec_like(args: RunExecLikeArgs) -> Result<FunctionToolOutput, FunctionCallError> {
    let RunExecLikeArgs {
        tool_name,
        exec_params,
        hook_command,
        prefix_rule,
        session,
        turn,
        tracker,
        call_id,
        freeform,
        shell_runtime_backend,
    } = args;

    let mut exec_params = exec_params;
    let Some(turn_environment) = turn.environments.primary() else {
        return Err(FunctionCallError::RespondToModel(
            "shell is unavailable in this session".to_string(),
        ));
    };
    let fs = turn_environment.environment.get_filesystem();

    let dependency_env = session.dependency_env().await;
    if !dependency_env.is_empty() {
        exec_params.env.extend(dependency_env.clone());
    }

    let mut explicit_env_overrides = turn.shell_environment_policy.r#set.clone();
    for key in dependency_env.keys() {
        if let Some(value) = exec_params.env.get(key) {
            explicit_env_overrides.insert(key.clone(), value.clone());
        }
    }

    // Intercept apply_patch if present.
    if let Some(output) = intercept_apply_patch(
        &exec_params.command,
        &exec_params.cwd,
        fs.as_ref(),
        session.clone(),
        turn.clone(),
        Some(&tracker),
        &call_id,
        tool_name.as_str(),
    )
    .await?
    {
        return Ok(output);
    }

    let source = ExecCommandSource::Agent;
    let emitter = ToolEmitter::shell(
        exec_params.command.clone(),
        exec_params.cwd.clone(),
        source,
        freeform,
    );
    let event_ctx = ToolEventCtx::new(
        session.as_ref(),
        turn.as_ref(),
        &call_id,
        /*turn_diff_tracker*/ None,
    );
    emitter.begin(event_ctx).await;

    let file_system_sandbox_policy = turn.file_system_sandbox_policy();
    let exec_approval_requirement = session
        .services
        .exec_policy
        .create_exec_approval_requirement_for_command(ExecApprovalRequest {
            command: &exec_params.command,
            approval_policy: turn.approval_policy.value(),
            permission_profile: turn.permission_profile(),
            file_system_sandbox_policy: &file_system_sandbox_policy,
            sandbox_cwd: turn.cwd.as_path(),
            prefix_rule,
        })
        .await;

    let req = ShellRequest {
        command: exec_params.command.clone(),
        hook_command,
        cwd: exec_params.cwd.clone(),
        timeout_ms: exec_params.expiration.timeout_ms(),
        env: exec_params.env.clone(),
        explicit_env_overrides,
        network: exec_params.network.clone(),
        justification: None,
        exec_approval_requirement,
    };
    let mut orchestrator = ToolOrchestrator::new();
    let mut runtime = {
        use ShellRuntimeBackend::*;
        match shell_runtime_backend {
            Generic => ShellRuntime::new(),
            backend @ (ShellCommandClassic | ShellCommandZshFork) => {
                ShellRuntime::for_shell_command(backend)
            }
        }
    };
    let tool_ctx = ToolCtx {
        session: session.clone(),
        turn: turn.clone(),
        call_id: call_id.clone(),
        tool_name,
    };
    let out = orchestrator
        .run(
            &mut runtime,
            &req,
            &tool_ctx,
            &turn,
            turn.approval_policy.value(),
        )
        .await
        .map(|result| result.output);
    let event_ctx = ToolEventCtx::new(
        session.as_ref(),
        turn.as_ref(),
        &call_id,
        /*turn_diff_tracker*/ None,
    );
    let post_tool_use_response = out
        .as_ref()
        .ok()
        .map(|output| crate::tools::format_exec_output_str(output, turn.truncation_policy))
        .map(JsonValue::String);
    let content = emitter
        .finish(event_ctx, out, /*applied_patch_delta*/ None)
        .await?;
    Ok(FunctionToolOutput {
        body: vec![
            codex_protocol::models::FunctionCallOutputContentItem::InputText { text: content },
        ],
        success: Some(true),
        post_tool_use_response,
    })
}
