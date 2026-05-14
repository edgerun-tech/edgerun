use std::collections::BTreeMap;
use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use std::time::Instant;

use codex_core::Prompt;
use codex_core::Provider;
use codex_core::ResponseEvent;
use codex_core::TokenUsage;
use codex_core::TurnOutput;
use codex_core::TurnRequest;
use codex_core::api::AuthProvider;
use codex_core::api::RetryConfig;
use codex_core::apply_patch;
use codex_core::protocol::models::ContentItem;
use codex_core::protocol::models::FunctionCallOutputPayload;
use codex_core::protocol::models::LocalShellAction;
use codex_core::protocol::models::LocalShellStatus;
use codex_core::protocol::models::ResponseItem;
use codex_core::protocol::protocol::RateLimitSnapshot;
use codex_core::tools::AdditionalProperties;
use codex_core::tools::JsonSchema;
use codex_core::tools::ResponsesApiTool;
use codex_core::tools::ToolSpec;
use edgerun_http::HeaderMap;
use edgerun_http::HeaderValue;
use edgerun_http::header::AUTHORIZATION;
use edgerun_json::Value;

mod ui;

const MAX_TOOL_ROUNDS: usize = 16;
const MAX_TOOL_OUTPUT_BYTES: usize = 24 * 1024;
const CODEX_BACKEND_VERSION: &str = "0.130.0";
const CANCEL_POLL_INTERVAL: Duration = Duration::from_millis(100);

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
    AssistantTextDelta(String),
    ReasoningDelta(String),
    ToolDiff(String),
    ToolInputDelta(String),
    ToolStarted(String),
    ToolCompleted {
        name: String,
        summary: String,
        success: bool,
    },
    Usage(TokenUsage),
    RateLimits(RateLimitSnapshot),
}

#[derive(Debug)]
enum ToolResponseTarget {
    Function { call_id: String },
    Custom { call_id: String, name: String },
}

#[derive(Debug)]
struct ToolInvocation {
    display_name: String,
    response: ToolResponseTarget,
    kind: ToolInvocationKind,
}

#[derive(Debug)]
enum ToolInvocationKind {
    ShellCommand(Result<ShellCommandArgs, String>),
    Shell(Result<ShellArgs, String>),
    ApplyPatch(Result<ApplyPatchArgs, String>),
    LocalShell(LocalShellArgs),
    Unsupported(String),
}

#[derive(Debug)]
struct ShellCommandArgs {
    command: String,
    workdir: Option<String>,
    timeout_ms: Option<u64>,
    login: bool,
}

#[derive(Debug)]
struct ShellArgs {
    command: Vec<String>,
    workdir: Option<String>,
    timeout_ms: Option<u64>,
}

#[derive(Debug)]
struct ApplyPatchArgs {
    input: String,
}

#[derive(Debug)]
struct LocalShellArgs {
    command: Vec<String>,
    workdir: Option<String>,
    timeout_ms: Option<u64>,
}

#[derive(Debug)]
struct ToolExecutionResult {
    success: bool,
    text: String,
    summary: String,
}

impl ToolExecutionResult {
    fn into_payload(self) -> FunctionCallOutputPayload {
        let mut payload = FunctionCallOutputPayload::from_text(truncate_tool_output(self.text));
        payload.success = Some(self.success);
        payload
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = AppArgs::parse()?;
    let model = std::env::var("CODEX_TUI_MODEL").unwrap_or_else(|_| "gpt-5.5".to_string());
    if args.dump_ui_scene {
        ui::run(ui::UiOptions {
            model,
            frames: args.frames,
            dump_scene: true,
        })
        .map_err(|error| -> Box<dyn Error> { error.into() })?;
        return Ok(());
    }

    if args.ui {
        ui::run(ui::UiOptions {
            model,
            frames: args.frames,
            dump_scene: false,
        })
        .map_err(|error| -> Box<dyn Error> { error.into() })?;
        return Ok(());
    }

    let auth = read_chatgpt_auth()?;
    let client = codex_core::ModelClient::new_native(model, provider(), auth);
    let runtime = edgerun_tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    if let Some(prompt) = args.prompt {
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
        AgentEvent::AssistantTextDelta(text) => {
            print!("{text}");
            let _ = io::stdout().flush();
        }
        AgentEvent::ReasoningDelta(_) => {}
        AgentEvent::ToolDiff(diff) => {
            println!("\n[diff]\n{diff}");
        }
        AgentEvent::ToolInputDelta(delta) => {
            print!("{delta}");
            let _ = io::stdout().flush();
        }
        AgentEvent::ToolStarted(name) => {
            println!("\n[tool] {name}");
        }
        AgentEvent::ToolCompleted { summary, .. } => {
            println!("[tool result] {summary}");
        }
        AgentEvent::Usage(_) | AgentEvent::RateLimits(_) => {}
    };
    let (_, history) = run_agent_loop(client, input, Some(&mut printer), None).await?;
    println!();
    Ok(history)
}

#[derive(Debug, Default)]
struct AppArgs {
    prompt: Option<String>,
    ui: bool,
    dump_ui_scene: bool,
    frames: Option<u32>,
}

impl AppArgs {
    fn parse() -> Result<Self, Box<dyn Error>> {
        let mut args = std::env::args().skip(1);
        let mut parsed = Self {
            ui: true,
            ..Self::default()
        };
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--prompt" | "-p" => {
                    let value = args
                        .next()
                        .ok_or("--prompt requires a prompt string argument")?;
                    parsed.prompt = Some(value);
                    parsed.ui = false;
                }
                "--cli" => parsed.ui = false,
                "--ui" => parsed.ui = true,
                "--dump-ui-scene" => {
                    parsed.dump_ui_scene = true;
                    parsed.ui = true;
                }
                "--frames" => {
                    parsed.frames = Some(
                        args.next()
                            .ok_or("--frames requires a frame count")?
                            .parse::<u32>()?,
                    );
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
                    println!(
                        "Usage: {program} [--ui|--cli] [--prompt TEXT] [--dump-ui-scene] [--frames N]"
                    );
                    std::process::exit(0);
                }
                other => return Err(format!("unknown argument: {other}").into()),
            }
        }
        Ok(parsed)
    }
}

#[allow(dead_code)]
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
            tools: cached_native_agent_tools(),
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
    cancel: Option<Arc<AtomicBool>>,
) -> Result<(Option<String>, Vec<ResponseItem>), Box<dyn Error>> {
    for _ in 0..MAX_TOOL_ROUNDS {
        if is_cancelled(cancel.as_deref()) {
            return Err("cancelled".into());
        }
        let streamed = stream_turn_with_events(
            client,
            turn_request(history.clone()),
            &mut emit,
            cancel.as_deref(),
        )
        .await?;
        let output = streamed.output;
        let last_response_id = output.response_id.clone();

        let tool_calls = collect_tool_invocations(&output.output_items);
        history.extend(output.output_items);
        if tool_calls.is_empty() {
            return Ok((last_response_id, history));
        }

        for invocation in tool_calls {
            if is_cancelled(cancel.as_deref()) {
                return Err("cancelled".into());
            }
            if let Some(emit) = emit.as_deref_mut() {
                if !streamed.streamed_tool_input
                    && let Some(diff) = patch_preview(&invocation)
                {
                    emit(AgentEvent::ToolDiff(diff));
                }
                emit(AgentEvent::ToolStarted(invocation.display_name.clone()));
            }
            let result = execute_tool_invocation(&invocation, cancel.as_deref()).await;
            if let Some(emit) = emit.as_deref_mut() {
                emit(AgentEvent::ToolCompleted {
                    name: invocation.display_name.clone(),
                    summary: result.summary.clone(),
                    success: result.success,
                });
            }
            history.push(tool_output_item(invocation, result.into_payload()));
        }
    }
    Err(format!("model exceeded {MAX_TOOL_ROUNDS} tool rounds").into())
}

struct StreamedTurnOutput {
    output: TurnOutput,
    streamed_tool_input: bool,
}

async fn stream_turn_with_events(
    client: &codex_core::ModelClient,
    request: TurnRequest,
    emit: &mut Option<&mut dyn FnMut(AgentEvent)>,
    cancel: Option<&AtomicBool>,
) -> Result<StreamedTurnOutput, Box<dyn Error>> {
    let mut stream = client.stream_turn(request).await?;
    let mut output = TurnOutput::default();
    let mut streamed_tool_input = false;

    loop {
        if is_cancelled(cancel) {
            return Err("cancelled".into());
        }
        let Some(event) =
            (match edgerun_tokio::time::timeout(CANCEL_POLL_INTERVAL, stream.rx_event.recv()).await
            {
                Ok(event) => event,
                Err(_) => continue,
            })
        else {
            break;
        };
        match event? {
            ResponseEvent::Created => {}
            ResponseEvent::OutputItemAdded(_) => {}
            ResponseEvent::OutputItemDone(item) => output.output_items.push(item),
            ResponseEvent::OutputTextDelta(delta) => {
                output.output_text.push_str(&delta);
                if let Some(emit) = emit.as_deref_mut() {
                    emit(AgentEvent::AssistantTextDelta(delta));
                }
            }
            ResponseEvent::ReasoningSummaryDelta { delta, .. } => {
                output.reasoning_summary_text.push_str(&delta);
                if let Some(emit) = emit.as_deref_mut() {
                    emit(AgentEvent::ReasoningDelta(delta));
                }
            }
            ResponseEvent::ReasoningContentDelta { delta, .. } => {
                output.reasoning_content_text.push_str(&delta);
                if let Some(emit) = emit.as_deref_mut() {
                    emit(AgentEvent::ReasoningDelta(delta));
                }
            }
            ResponseEvent::Completed {
                response_id,
                token_usage,
                end_turn,
            } => {
                output.response_id = Some(response_id);
                if let Some(usage) = token_usage {
                    if let Some(emit) = emit.as_deref_mut() {
                        emit(AgentEvent::Usage(usage.clone()));
                    }
                    output.token_usage = Some(usage);
                }
                output.end_turn = end_turn;
            }
            ResponseEvent::ServerModel(model) => output.server_model = Some(model),
            ResponseEvent::ServerReasoningIncluded(included) => {
                output.server_reasoning_included = included;
            }
            ResponseEvent::ToolCallInputDelta { delta, .. } => {
                streamed_tool_input = true;
                if let Some(emit) = emit.as_deref_mut() {
                    emit(AgentEvent::ToolInputDelta(delta));
                }
            }
            ResponseEvent::RateLimits(snapshot) => {
                if let Some(emit) = emit.as_deref_mut() {
                    emit(AgentEvent::RateLimits(snapshot));
                }
            }
            ResponseEvent::ReasoningSummaryPartAdded { .. }
            | ResponseEvent::ModelVerifications(_)
            | ResponseEvent::ModelsEtag(_) => {}
        }
    }

    Ok(StreamedTurnOutput {
        output,
        streamed_tool_input,
    })
}

fn is_cancelled(cancel: Option<&AtomicBool>) -> bool {
    cancel
        .map(|cancel| cancel.load(Ordering::Relaxed))
        .unwrap_or(false)
}

fn user_item(text: String) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText { text }],
        phase: None,
    }
}

fn collect_tool_invocations(items: &[ResponseItem]) -> Vec<ToolInvocation> {
    items
        .iter()
        .filter_map(|item| match item {
            ResponseItem::FunctionCall {
                name,
                arguments,
                call_id,
                ..
            } => Some(function_tool_invocation(name, arguments, call_id)),
            ResponseItem::CustomToolCall {
                name,
                input,
                call_id,
                ..
            } => Some(custom_tool_invocation(name, input, call_id)),
            ResponseItem::LocalShellCall {
                call_id: Some(call_id),
                status: LocalShellStatus::Completed | LocalShellStatus::InProgress,
                action: LocalShellAction::Exec(action),
                ..
            } => Some(ToolInvocation {
                display_name: format!("local_shell {}", action.command.join(" ")),
                response: ToolResponseTarget::Function {
                    call_id: call_id.clone(),
                },
                kind: ToolInvocationKind::LocalShell(LocalShellArgs {
                    command: action.command.clone(),
                    workdir: action.working_directory.clone(),
                    timeout_ms: action.timeout_ms,
                }),
            }),
            _ => None,
        })
        .collect()
}

fn function_tool_invocation(name: &str, arguments: &str, call_id: &str) -> ToolInvocation {
    let kind = match name {
        "shell_command" => ToolInvocationKind::ShellCommand(parse_shell_command_args(arguments)),
        "shell" | "container.exec" => ToolInvocationKind::Shell(parse_shell_args(arguments)),
        "apply_patch" => ToolInvocationKind::ApplyPatch(parse_apply_patch_args(arguments)),
        other => ToolInvocationKind::Unsupported(format!("unsupported tool call: {other}")),
    };
    ToolInvocation {
        display_name: name.to_string(),
        response: ToolResponseTarget::Function {
            call_id: call_id.to_string(),
        },
        kind,
    }
}

fn custom_tool_invocation(name: &str, input: &str, call_id: &str) -> ToolInvocation {
    let kind = match name {
        "apply_patch" => ToolInvocationKind::ApplyPatch(Ok(ApplyPatchArgs {
            input: input.to_string(),
        })),
        other => ToolInvocationKind::Unsupported(format!("unsupported tool call: {other}")),
    };
    ToolInvocation {
        display_name: name.to_string(),
        response: ToolResponseTarget::Custom {
            call_id: call_id.to_string(),
            name: name.to_string(),
        },
        kind,
    }
}

async fn execute_tool_invocation(
    invocation: &ToolInvocation,
    cancel: Option<&AtomicBool>,
) -> ToolExecutionResult {
    let result = match &invocation.kind {
        ToolInvocationKind::ShellCommand(Ok(args)) => execute_shell_command(args, cancel).await,
        ToolInvocationKind::ShellCommand(Err(error))
        | ToolInvocationKind::Shell(Err(error))
        | ToolInvocationKind::ApplyPatch(Err(error)) => Err(error.clone()),
        ToolInvocationKind::Shell(Ok(args)) => {
            execute_process(
                args.command.clone(),
                args.workdir.clone(),
                args.timeout_ms,
                cancel,
            )
            .await
        }
        ToolInvocationKind::ApplyPatch(Ok(args)) => execute_apply_patch(&args.input).await,
        ToolInvocationKind::LocalShell(args) => {
            execute_process(
                args.command.clone(),
                args.workdir.clone(),
                args.timeout_ms,
                cancel,
            )
            .await
        }
        ToolInvocationKind::Unsupported(error) => Err(error.clone()),
    };
    match result {
        Ok(text) => ToolExecutionResult {
            success: true,
            summary: tool_summary(&invocation.display_name, &text),
            text,
        },
        Err(error) => {
            let text = format!("tool error: {error}");
            ToolExecutionResult {
                success: false,
                summary: tool_summary(&invocation.display_name, &text),
                text,
            }
        }
    }
}

fn parse_shell_command_args(arguments: &str) -> Result<ShellCommandArgs, String> {
    let params = parse_tool_arguments(arguments)?;
    Ok(ShellCommandArgs {
        command: json_required_string(&params, "command")?.to_string(),
        workdir: json_optional_string(&params, "workdir").map(ToString::to_string),
        timeout_ms: json_optional_u64(&params, "timeout_ms")
            .or_else(|| json_optional_u64(&params, "timeout")),
        login: json_optional_bool(&params, "login").unwrap_or(false),
    })
}

async fn execute_shell_command(
    args: &ShellCommandArgs,
    cancel: Option<&AtomicBool>,
) -> Result<String, String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let shell_flag = if args.login { "-lc" } else { "-c" };
    execute_process(
        vec![shell, shell_flag.to_string(), args.command.clone()],
        args.workdir.clone(),
        args.timeout_ms,
        cancel,
    )
    .await
}

fn parse_shell_args(arguments: &str) -> Result<ShellArgs, String> {
    let params = parse_tool_arguments(arguments)?;
    Ok(ShellArgs {
        command: json_required_string_array(&params, "command")?,
        workdir: json_optional_string(&params, "workdir").map(ToString::to_string),
        timeout_ms: json_optional_u64(&params, "timeout_ms")
            .or_else(|| json_optional_u64(&params, "timeout")),
    })
}

async fn execute_process(
    command: Vec<String>,
    workdir: Option<String>,
    timeout_ms: Option<u64>,
    cancel: Option<&AtomicBool>,
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
        if is_cancelled(cancel) {
            let _ = child.kill();
            let output = child
                .wait_with_output()
                .map_err(|error| error.to_string())?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "command cancelled\nstdout:\n{}\nstderr:\n{}",
                stdout, stderr
            ));
        }
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

fn parse_apply_patch_args(arguments: &str) -> Result<ApplyPatchArgs, String> {
    let args = parse_tool_arguments(arguments)?;
    Ok(ApplyPatchArgs {
        input: json_required_string(&args, "input")?.to_string(),
    })
}

fn parse_tool_arguments(arguments: &str) -> Result<Value, String> {
    match edgerun_json::from_str(arguments) {
        Ok(Value::String(inner)) => {
            edgerun_json::from_str(&inner).map_err(|error| error.to_string())
        }
        Ok(value) => Ok(value),
        Err(error) => {
            let trimmed = arguments.trim();
            if (trimmed.starts_with('{') || trimmed.starts_with('[')) && trimmed.contains("\\\"") {
                let wrapped = format!("\"{trimmed}\"");
                if let Ok(Value::String(inner)) = edgerun_json::from_str(&wrapped) {
                    return edgerun_json::from_str(&inner).map_err(|inner_error| {
                        format!("{error}; also failed to parse escaped arguments after unwrapping: {inner_error}")
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
    let cwd = apply_patch::AbsolutePathBuf::from_absolute_path(cwd)?;
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

fn tool_output_item(invocation: ToolInvocation, output: FunctionCallOutputPayload) -> ResponseItem {
    match invocation.response {
        ToolResponseTarget::Custom { call_id, name } => ResponseItem::CustomToolCallOutput {
            call_id,
            name: Some(name),
            output,
        },
        ToolResponseTarget::Function { call_id } => {
            ResponseItem::FunctionCallOutput { call_id, output }
        }
    }
}

fn tool_summary(display_name: &str, text: &str) -> String {
    let first_line = text.lines().next().unwrap_or("");
    format!("{display_name} -> {first_line}")
}

fn patch_preview(invocation: &ToolInvocation) -> Option<String> {
    match &invocation.kind {
        ToolInvocationKind::ApplyPatch(Ok(args)) => Some(args.input.clone()),
        _ => None,
    }
    .map(truncate_diff_preview)
}

fn truncate_diff_preview(mut text: String) -> String {
    const MAX_DIFF_PREVIEW_BYTES: usize = 12 * 1024;
    if text.len() <= MAX_DIFF_PREVIEW_BYTES {
        return text;
    }
    text.truncate(MAX_DIFF_PREVIEW_BYTES);
    text.push_str("\n[diff preview truncated]");
    text
}

fn truncate_tool_output(mut text: String) -> String {
    if text.len() <= MAX_TOOL_OUTPUT_BYTES {
        return text;
    }
    text.truncate(MAX_TOOL_OUTPUT_BYTES);
    text.push_str("\n[output truncated]");
    text
}

fn cached_native_agent_tools() -> Vec<ToolSpec> {
    static TOOLS: OnceLock<Vec<ToolSpec>> = OnceLock::new();
    TOOLS.get_or_init(native_agent_tools).clone()
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
    headers.insert("version", HeaderValue::from_static(CODEX_BACKEND_VERSION));
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

fn codex_home() -> Result<PathBuf, String> {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .ok_or_else(|| "CODEX_HOME or HOME must be set".to_string())
}

fn read_chatgpt_auth() -> Result<Arc<ChatGptAuth>, Box<dyn Error>> {
    let auth_path = codex_home()?.join("auth.json");
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
    use super::ToolInvocationKind;
    use super::execute_process;
    use super::function_tool_invocation;
    use super::json_required_string;
    use super::parse_tool_arguments;
    use super::patch_preview;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

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

    #[test]
    fn patch_preview_extracts_apply_patch_input() {
        let invocation = function_tool_invocation(
            "apply_patch",
            r#"{"input":"*** Begin Patch\n*** End Patch\n"}"#,
            "call-1",
        );

        assert_eq!(
            patch_preview(&invocation).as_deref(),
            Some("*** Begin Patch\n*** End Patch\n")
        );
    }

    #[test]
    fn function_tool_invocation_parses_typed_shell_args_once() {
        let invocation = function_tool_invocation(
            "shell_command",
            r#"{"command":"echo ok","workdir":"/tmp","timeout_ms":1000,"login":true}"#,
            "call-1",
        );

        let ToolInvocationKind::ShellCommand(Ok(args)) = invocation.kind else {
            panic!("expected shell_command args");
        };
        assert_eq!(args.command, "echo ok");
        assert_eq!(args.workdir.as_deref(), Some("/tmp"));
        assert_eq!(args.timeout_ms, Some(1000));
        assert!(args.login);
    }

    #[test]
    fn execute_process_honors_cancel_flag() {
        let runtime = edgerun_tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let cancel = Arc::new(AtomicBool::new(true));

        let result = runtime.block_on(execute_process(
            vec!["sh".to_string(), "-c".to_string(), "sleep 5".to_string()],
            None,
            Some(30_000),
            Some(cancel.as_ref()),
        ));

        assert!(
            result
                .expect_err("cancelled command should fail")
                .contains("command cancelled")
        );
        assert!(cancel.load(Ordering::Relaxed));
    }
}
