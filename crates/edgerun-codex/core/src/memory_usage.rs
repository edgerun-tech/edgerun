use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::unified_exec::ExecCommandArgs;
use codex_memories_read::usage::MEMORIES_USAGE_METRIC;
use codex_memories_read::usage::memories_usage_kinds_from_command;
use std::path::PathBuf;

pub(crate) async fn emit_metric_for_tool_read(invocation: &ToolInvocation, success: bool) {
    let Some((command, _)) = shell_command_for_invocation(invocation) else {
        return;
    };
    let kinds = memories_usage_kinds_from_command(&command);
    if kinds.is_empty() {
        return;
    }

    let success = if success { "true" } else { "false" };
    let tool_name = invocation.tool_name.display();
    for kind in kinds {
        invocation.turn.session_telemetry.counter(
            MEMORIES_USAGE_METRIC,
            /*inc*/ 1,
            &[
                ("kind", kind.as_tag()),
                ("tool", &tool_name),
                ("success", success),
            ],
        );
    }
}

fn shell_command_for_invocation(invocation: &ToolInvocation) -> Option<(Vec<String>, PathBuf)> {
    let ToolPayload::Function { arguments } = &invocation.payload else {
        return None;
    };

    match (
        invocation.tool_name.namespace.as_deref(),
        invocation.tool_name.name.as_str(),
    ) {
        (None, "shell") => shell_command_argv_and_workdir(arguments).map(|(command, workdir)| {
            (command, invocation.turn.resolve_path(workdir).to_path_buf())
        }),
        (None, "shell_command") => {
            shell_command_text_login_and_workdir(arguments).map(|(command_text, login, workdir)| {
                if !invocation.turn.tools_config.allow_login_shell && login == Some(true) {
                    return (
                        Vec::new(),
                        invocation.turn.resolve_path(workdir).to_path_buf(),
                    );
                }
                let use_login_shell =
                    login.unwrap_or(invocation.turn.tools_config.allow_login_shell);
                let command = invocation
                    .session
                    .user_shell()
                    .derive_exec_args(&command_text, use_login_shell);
                (command, invocation.turn.resolve_path(workdir).to_path_buf())
            })
        }
        (None, "exec_command") => edgerun_json::from_serde_str::<ExecCommandArgs>(arguments)
            .ok()
            .and_then(|params| {
                let command = crate::tools::handlers::unified_exec::get_command(
                    &params,
                    invocation.session.user_shell(),
                    &invocation.turn.tools_config.unified_exec_shell_mode,
                    invocation.turn.tools_config.allow_login_shell,
                )
                .ok()?;
                Some((
                    command,
                    invocation.turn.resolve_path(params.workdir).to_path_buf(),
                ))
            }),
        (Some(_), _) | (None, _) => None,
    }
}

fn shell_command_argv_and_workdir(arguments: &str) -> Option<(Vec<String>, Option<String>)> {
    let tape = edgerun_json::parse_json_tape(arguments).ok()?;
    let root = tape.root(arguments)?;
    let command = root
        .get_array("command")?
        .into_iter()
        .map(|item| item.as_str().map(str::to_string))
        .collect::<Option<Vec<_>>>()?;
    let workdir = root
        .get("workdir")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    Some((command, workdir))
}

fn shell_command_text_login_and_workdir(
    arguments: &str,
) -> Option<(String, Option<bool>, Option<String>)> {
    let tape = edgerun_json::parse_json_tape(arguments).ok()?;
    let root = tape.root(arguments)?;
    let command = root.get("command")?.as_str()?.to_string();
    let login = root.get("login").and_then(|value| value.as_bool());
    let workdir = root
        .get("workdir")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    Some((command, login, workdir))
}
