use std::collections::BTreeMap;
use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use codex_core::Prompt;
use codex_core::Provider;
use codex_core::TurnRequest;
use codex_core::api::AuthProvider;
use codex_core::api::RetryConfig;
use codex_core::apply_patch;
use codex_core::protocol::models::ContentItem;
use codex_core::protocol::models::FunctionCallOutputPayload;
use codex_core::protocol::models::LocalShellAction;
use codex_core::protocol::models::LocalShellStatus;
use codex_core::protocol::models::ResponseItem;
use codex_core::tools::AdditionalProperties;
use codex_core::tools::JsonSchema;
use codex_core::tools::ResponsesApiTool;
use codex_core::tools::ToolSpec;
use edgerun_http::HeaderMap;
use edgerun_http::HeaderValue;
use edgerun_http::header::AUTHORIZATION;
use edgerun_json::Value;

const MAX_TOOL_ROUNDS: usize = 16;
const MAX_TOOL_OUTPUT_BYTES: usize = 24 * 1024;

#[derive(Debug)]
struct ChatGptAuth {
    access_token: String,
    account_id: Option<String>,
}

impl AuthProvider for ChatGptAuth {
    fn add_auth_headers(&self, headers: &mut HeaderMap) {
        let bearer = format!("Bearer {}", self.access_token);
        if let Ok(value) = HeaderValue::from_str(&bearer) {
            headers.insert(AUTHORIZATION, value);
        }
        if let Some(account_id) = self.account_id.as_deref()
            && let Ok(value) = HeaderValue::from_str(account_id)
        {
            headers.insert("ChatGPT-Account-ID", value);
        }
    }
}

#[derive(Debug)]
enum AgentEvent {
    AssistantText(String),
    ToolStarted(String),
    ToolCompleted(String),
}

#[derive(Debug, Clone)]
enum ToolCall {
    Function {
        name: String,
        arguments: String,
        call_id: String,
    },
    Custom {
        name: String,
        input: String,
        call_id: String,
    },
    LocalShell {
        command: Vec<String>,
        workdir: Option<String>,
        timeout_ms: Option<u64>,
        call_id: String,
    },
}

impl ToolCall {
    fn display_name(&self) -> String {
        match self {
            ToolCall::Function { name, .. } | ToolCall::Custom { name, .. } => name.clone(),
            ToolCall::LocalShell { command, .. } => format!("local_shell {}", command.join(" ")),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let auth = read_chatgpt_auth()?;
    let model = std::env::var("CODEX_TUI_MODEL").unwrap_or_else(|_| "gpt-5.5".to_string());
    let client = codex_core::ModelClient::new_native(model, provider(), auth);
    let runtime = edgerun_tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    if let Some(prompt) = prompt_arg()? {
        runtime.block_on(run_prompt(&client, vec![user_item(prompt)]))?;
        return Ok(());
    }

    println!("Native EdgeRun Codex. Tools enabled: shell_command, shell, apply_patch.");
    println!("Enter a prompt, or Ctrl-D to quit.");
    let mut history = Vec::new();
    let stdin = io::stdin();
    loop {
        print!("edgerun-codex> ");
        io::stdout().flush()?;

        let mut input = String::new();
        if stdin.read_line(&mut input)? == 0 {
            break;
        }
        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        history.push(user_item(input.to_string()));
        history = runtime.block_on(run_prompt(&client, history))?;
    }
    Ok(())
}

async fn run_prompt(
    client: &codex_core::ModelClient,
    input: Vec<ResponseItem>,
) -> Result<Vec<ResponseItem>, Box<dyn Error>> {
    let mut printer = |event: AgentEvent| match event {
        AgentEvent::AssistantText(text) => {
            print!("{text}");
            let _ = io::stdout().flush();
        }
        AgentEvent::ToolStarted(name) => {
            println!("\n[tool] {name}");
        }
        AgentEvent::ToolCompleted(summary) => {
            println!("[tool result] {summary}");
        }
    };
    let (_, history) = run_agent_loop(client, input, Some(&mut printer)).await?;
    println!();
    Ok(history)
}

fn prompt_arg() -> Result<Option<String>, Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mut prompt = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--prompt" | "-p" => {
                let value = args
                    .next()
                    .ok_or("--prompt requires a prompt string argument")?;
                prompt = Some(value);
            }
            "--help" | "-h" => {
                let program = std::env::args()
                    .next()
                    .and_then(|path| {
                        PathBuf::from(path)
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                    })
                    .unwrap_or_else(|| "edgerun-codex".to_string());
                println!("Usage: {program} [--prompt TEXT]");
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }
    Ok(prompt)
}

fn turn_request(input: Vec<ResponseItem>) -> TurnRequest {
    TurnRequest {
        prompt: Prompt {
            input,
            tools: native_agent_tools(),
            parallel_tool_calls: true,
            base_instructions: codex_core::protocol::models::BaseInstructions {
                text: include_str!("../../core/gpt_5_codex_prompt.md").to_string(),
            },
            ..Prompt::default()
        },
        store: false,
        ..TurnRequest::default()
    }
}

async fn run_agent_loop(
    client: &codex_core::ModelClient,
    mut history: Vec<ResponseItem>,
    mut emit: Option<&mut dyn FnMut(AgentEvent)>,
) -> Result<(Option<String>, Vec<ResponseItem>), Box<dyn Error>> {
    for _ in 0..MAX_TOOL_ROUNDS {
        let output = client.collect_turn(turn_request(history.clone())).await?;
        let last_response_id = output.response_id.clone();
        if !output.output_text.is_empty()
            && let Some(emit) = emit.as_deref_mut()
        {
            emit(AgentEvent::AssistantText(output.output_text.clone()));
        }

        let tool_calls = collect_tool_calls(&output.output_items);
        history.extend(output.output_items);
        if tool_calls.is_empty() {
            return Ok((last_response_id, history));
        }

        for call in tool_calls {
            if let Some(emit) = emit.as_deref_mut() {
                emit(AgentEvent::ToolStarted(call.display_name()));
            }
            let output = execute_tool_call(&call).await;
            if let Some(emit) = emit.as_deref_mut() {
                emit(AgentEvent::ToolCompleted(tool_summary(&call, &output)));
            }
            history.push(tool_output_item(call, output));
        }
    }
    Err(format!("model exceeded {MAX_TOOL_ROUNDS} tool rounds").into())
}

fn user_item(text: String) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText { text }],
        phase: None,
    }
}

fn collect_tool_calls(items: &[ResponseItem]) -> Vec<ToolCall> {
    items
        .iter()
        .filter_map(|item| match item {
            ResponseItem::FunctionCall {
                name,
                arguments,
                call_id,
                ..
            } => Some(ToolCall::Function {
                name: name.clone(),
                arguments: arguments.clone(),
                call_id: call_id.clone(),
            }),
            ResponseItem::CustomToolCall {
                name,
                input,
                call_id,
                ..
            } => Some(ToolCall::Custom {
                name: name.clone(),
                input: input.clone(),
                call_id: call_id.clone(),
            }),
            ResponseItem::LocalShellCall {
                call_id: Some(call_id),
                status: LocalShellStatus::Completed | LocalShellStatus::InProgress,
                action: LocalShellAction::Exec(action),
                ..
            } => Some(ToolCall::LocalShell {
                command: action.command.clone(),
                workdir: action.working_directory.clone(),
                timeout_ms: action.timeout_ms,
                call_id: call_id.clone(),
            }),
            _ => None,
        })
        .collect()
}

async fn execute_tool_call(call: &ToolCall) -> FunctionCallOutputPayload {
    let result = match call {
        ToolCall::Function {
            name, arguments, ..
        } if name == "shell_command" => execute_shell_command(arguments).await,
        ToolCall::Function {
            name, arguments, ..
        } if name == "shell" || name == "container.exec" => execute_shell(arguments).await,
        ToolCall::Function {
            name, arguments, ..
        } if name == "apply_patch" => execute_apply_patch_json(arguments).await,
        ToolCall::Custom { name, input, .. } if name == "apply_patch" => {
            execute_apply_patch(input).await
        }
        ToolCall::LocalShell {
            command,
            workdir,
            timeout_ms,
            ..
        } => execute_process(command.clone(), workdir.clone(), *timeout_ms).await,
        ToolCall::Function { name, .. } | ToolCall::Custom { name, .. } => {
            Err(format!("unsupported tool call: {name}"))
        }
    };

    let success = result.is_ok();
    let mut payload = FunctionCallOutputPayload::from_text(match result {
        Ok(text) => truncate_tool_output(text),
        Err(error) => truncate_tool_output(format!("tool error: {error}")),
    });
    payload.success = Some(success);
    payload
}

async fn execute_shell_command(arguments: &str) -> Result<String, String> {
    let params = parse_tool_arguments(arguments)?;
    let command = json_required_string(&params, "command")?.to_string();
    let workdir = json_optional_string(&params, "workdir").map(ToString::to_string);
    let timeout_ms =
        json_optional_u64(&params, "timeout_ms").or_else(|| json_optional_u64(&params, "timeout"));
    let login = json_optional_bool(&params, "login").unwrap_or(false);
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let shell_flag = if login { "-lc" } else { "-c" };
    execute_process(
        vec![shell, shell_flag.to_string(), command],
        workdir,
        timeout_ms,
    )
    .await
}

async fn execute_shell(arguments: &str) -> Result<String, String> {
    let params = parse_tool_arguments(arguments)?;
    let command = json_required_string_array(&params, "command")?;
    let workdir = json_optional_string(&params, "workdir").map(ToString::to_string);
    let timeout_ms =
        json_optional_u64(&params, "timeout_ms").or_else(|| json_optional_u64(&params, "timeout"));
    execute_process(command, workdir, timeout_ms).await
}

async fn execute_process(
    command: Vec<String>,
    workdir: Option<String>,
    timeout_ms: Option<u64>,
) -> Result<String, String> {
    if command.is_empty() {
        return Err("command must not be empty".to_string());
    }

    let mut cmd = Command::new(&command[0]);
    cmd.args(&command[1..]);
    if let Some(workdir) = workdir {
        cmd.current_dir(workdir);
    }
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(60_000));
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let started = Instant::now();

    loop {
        if child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_some()
        {
            break;
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let output = child
                .wait_with_output()
                .map_err(|error| error.to_string())?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "command timed out after {}ms\nstdout:\n{}\nstderr:\n{}",
                timeout.as_millis(),
                stdout,
                stderr
            ));
        }
        thread::sleep(Duration::from_millis(25));
    }

    let output = child
        .wait_with_output()
        .map_err(|error| error.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let text = format!(
        "exit_code: {}\nstdout:\n{}\nstderr:\n{}",
        output.status.code().unwrap_or(-1),
        stdout,
        stderr
    );
    if output.status.success() {
        Ok(text)
    } else {
        Err(text)
    }
}

async fn execute_apply_patch_json(arguments: &str) -> Result<String, String> {
    let args = parse_tool_arguments(arguments)?;
    let input = json_required_string(&args, "input")?;
    execute_apply_patch(input).await
}

fn parse_tool_arguments(arguments: &str) -> Result<Value, String> {
    match edgerun_json::from_str(arguments) {
        Ok(Value::String(inner)) => {
            return edgerun_json::from_str(&inner).map_err(|error| error.to_string());
        }
        Ok(value) => return Ok(value),
        Err(error) => {
            let trimmed = arguments.trim();
            if (trimmed.starts_with('{') || trimmed.starts_with('[')) && trimmed.contains("\\\"") {
                let wrapped = format!("\"{trimmed}\"");
                if let Ok(Value::String(inner)) = edgerun_json::from_str(&wrapped) {
                    return edgerun_json::from_str(&inner).map_err(|inner_error| {
                        format!(
                            "{}; also failed to parse escaped arguments after unwrapping: {}",
                            error, inner_error
                        )
                    });
                }
            }
            Err(error.to_string())
        }
    }
}

fn json_required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn json_optional_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn json_optional_bool(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn json_optional_u64(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn json_required_string_array(value: &Value, key: &str) -> Result<Vec<String>, String> {
    let array = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{key} must be an array of strings"))?;
    array
        .iter()
        .map(|item| {
            item.as_str()
                .map(ToString::to_string)
                .ok_or_else(|| format!("{key} must be an array of strings"))
        })
        .collect()
}

async fn execute_apply_patch(input: &str) -> Result<String, String> {
    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    let cwd =
        apply_patch::AbsolutePathBuf::from_absolute_path(cwd).map_err(|error| error.to_string())?;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    match apply_patch::apply_patch(
        input,
        &cwd,
        &mut stdout,
        &mut stderr,
        apply_patch::LOCAL_FS.as_ref(),
        None,
    )
    .await
    {
        Ok(_) => Ok(format!(
            "{}{}",
            String::from_utf8_lossy(&stdout),
            String::from_utf8_lossy(&stderr)
        )),
        Err(error) => {
            let (error, _) = error.into_parts();
            Err(format!("{}{}", String::from_utf8_lossy(&stderr), error))
        }
    }
}

fn tool_output_item(call: ToolCall, output: FunctionCallOutputPayload) -> ResponseItem {
    match call {
        ToolCall::Custom { call_id, name, .. } => ResponseItem::CustomToolCallOutput {
            call_id,
            name: Some(name),
            output,
        },
        ToolCall::Function { call_id, .. } | ToolCall::LocalShell { call_id, .. } => {
            ResponseItem::FunctionCallOutput { call_id, output }
        }
    }
}

fn tool_summary(call: &ToolCall, output: &FunctionCallOutputPayload) -> String {
    let text = output.to_string();
    let first_line = text.lines().next().unwrap_or("");
    format!("{} -> {}", call.display_name(), first_line)
}

fn truncate_tool_output(mut text: String) -> String {
    if text.len() <= MAX_TOOL_OUTPUT_BYTES {
        return text;
    }
    text.truncate(MAX_TOOL_OUTPUT_BYTES);
    text.push_str("\n[output truncated]");
    text
}

fn native_agent_tools() -> Vec<ToolSpec> {
    vec![shell_command_tool(), shell_tool(), apply_patch_json_tool()]
}

fn shell_command_tool() -> ToolSpec {
    let properties = BTreeMap::from([
        (
            "command".to_string(),
            JsonSchema::string(Some(
                "The shell script to execute in the user's default shell".to_string(),
            )),
        ),
        (
            "workdir".to_string(),
            JsonSchema::string(Some(
                "The working directory to execute the command in".to_string(),
            )),
        ),
        (
            "timeout_ms".to_string(),
            JsonSchema::number(Some(
                "The timeout for the command in milliseconds".to_string(),
            )),
        ),
        (
            "login".to_string(),
            JsonSchema::boolean(Some(
                "Whether to run the shell with login shell semantics".to_string(),
            )),
        ),
    ]);
    ToolSpec::Function(ResponsesApiTool {
        name: "shell_command".to_string(),
        description:
            "Runs a shell command and returns exit code, stdout, and stderr. Always set workdir."
                .to_string(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(
            properties,
            Some(vec!["command".to_string()]),
            Some(AdditionalProperties::Boolean(false)),
        ),
        output_schema: None,
    })
}

fn shell_tool() -> ToolSpec {
    let properties = BTreeMap::from([
        (
            "command".to_string(),
            JsonSchema::array(
                JsonSchema::string(Some("Command argument".to_string())),
                Some("Program and arguments to execute".to_string()),
            ),
        ),
        (
            "workdir".to_string(),
            JsonSchema::string(Some(
                "The working directory to execute the command in".to_string(),
            )),
        ),
        (
            "timeout_ms".to_string(),
            JsonSchema::number(Some(
                "The timeout for the command in milliseconds".to_string(),
            )),
        ),
    ]);
    ToolSpec::Function(ResponsesApiTool {
        name: "shell".to_string(),
        description: "Runs a process directly. Most terminal commands should be [\"bash\", \"-lc\", \"...\"] and include workdir.".to_string(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(
            properties,
            Some(vec!["command".to_string()]),
            Some(AdditionalProperties::Boolean(false)),
        ),
        output_schema: None,
    })
}

fn apply_patch_json_tool() -> ToolSpec {
    let properties = BTreeMap::from([(
        "input".to_string(),
        JsonSchema::string(Some(
            "The entire contents of the apply_patch command".to_string(),
        )),
    )]);
    ToolSpec::Function(ResponsesApiTool {
        name: "apply_patch".to_string(),
        description: "Use apply_patch to edit files. Pass the complete patch as input.".to_string(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(
            properties,
            Some(vec!["input".to_string()]),
            Some(AdditionalProperties::Boolean(false)),
        ),
        output_schema: None,
    })
}

fn provider() -> Provider {
    let mut headers = HeaderMap::new();
    headers.insert(
        "version",
        HeaderValue::from_static(env!("CARGO_PKG_VERSION")),
    );
    Provider {
        name: "OpenAI".to_string(),
        base_url: "https://chatgpt.com/backend-api/codex".to_string(),
        query_params: None::<HashMap<String, String>>,
        headers,
        retry: RetryConfig {
            max_attempts: 1,
            base_delay: Duration::from_millis(200),
            retry_429: false,
            retry_5xx: false,
            retry_transport: false,
        },
        stream_idle_timeout: Duration::from_secs(60),
    }
}

fn codex_home() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .expect("CODEX_HOME or HOME must be set")
}

fn read_chatgpt_auth() -> Result<Arc<ChatGptAuth>, Box<dyn Error>> {
    let auth_path = codex_home().join("auth.json");
    let auth = edgerun_json::from_slice(&std::fs::read(&auth_path)?)?;
    let access_token = auth
        .get("tokens")
        .and_then(|tokens| tokens.get("access_token"))
        .and_then(Value::as_str)
        .filter(|token| !token.is_empty())
        .ok_or_else(|| format!("missing tokens.access_token in {}", auth_path.display()))?
        .to_string();
    let account_id = auth
        .get("tokens")
        .and_then(|tokens| tokens.get("account_id"))
        .and_then(Value::as_str)
        .filter(|account_id| !account_id.is_empty())
        .map(ToString::to_string);
    Ok(Arc::new(ChatGptAuth {
        access_token,
        account_id,
    }))
}

#[cfg(test)]
mod tests {
    use super::json_required_string;
    use super::parse_tool_arguments;

    #[test]
    fn parse_tool_arguments_accepts_normal_json() {
        let args = parse_tool_arguments(r#"{"command":"echo ok"}"#).expect("parse args");

        assert_eq!(
            json_required_string(&args, "command").expect("command"),
            "echo ok"
        );
    }

    #[test]
    fn parse_tool_arguments_accepts_json_string_wrapped_object() {
        let args = parse_tool_arguments(r#""{\"command\":\"echo ok\"}""#).expect("parse args");

        assert_eq!(
            json_required_string(&args, "command").expect("command"),
            "echo ok"
        );
    }

    #[test]
    fn parse_tool_arguments_accepts_escaped_object_without_outer_quotes() {
        let args = parse_tool_arguments(r#"{\"command\":\"echo ok\"}"#).expect("parse args");

        assert_eq!(
            json_required_string(&args, "command").expect("command"),
            "echo ok"
        );
    }
}
