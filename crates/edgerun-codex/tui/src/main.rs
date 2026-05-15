use std::collections::BTreeMap;
use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use codex_core::Prompt;
use codex_core::Provider;
use codex_core::ResponseEvent;
use codex_core::TokenUsage;
use codex_core::TurnOutput;
use codex_core::TurnRequest;
use codex_core::api::AuthProvider;
use codex_core::api::RetryConfig;
use codex_core::protocol::models::ContentItem;
use codex_core::protocol::models::FunctionCallOutputPayload;
use codex_core::protocol::models::ResponseItem;
use codex_core::protocol::protocol::RateLimitSnapshot;
use codex_core::tools::AdditionalProperties;
use codex_core::tools::JsonSchema;
use codex_core::tools::ResponsesApiTool;
use codex_core::tools::ToolSpec;
use edgerun_codelyzer::filesystem::load_vfs_from_dir;
use edgerun_http::HeaderMap;
use edgerun_http::HeaderValue;
use edgerun_http::header::AUTHORIZATION;
use edgerun_json::Value;
use edgerun_vfs::VirtualFileSystem;

mod ui;

const MAX_TOOL_ROUNDS: usize = 16;
const MAX_TOOL_OUTPUT_BYTES: usize = 24 * 1024;
const MAX_CONTEXT_BYTES: usize = 24 * 1024;
const MAX_SEARCH_MATCHES: usize = 80;
const MAX_PROPOSAL_BYTES: usize = 256 * 1024;
const CODEX_BACKEND_VERSION: &str = "0.130.0";
const CANCEL_POLL_INTERVAL: Duration = Duration::from_millis(100);
const WORKSPACE_EXCLUDE_DIRS: &[&str] = &[
    ".git",
    ".next",
    ".turbo",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "out",
    "target",
    "vendor",
];
const WORKSPACE_TOOL_INSTRUCTIONS: &str = r#"
EdgeRun workspace model:
- You do not have direct shell, process, or file-write access.
- Use search_code and read_code to inspect the in-memory workspace snapshot.
- Use propose_change to update the in-memory workspace and describe the intended edit.
- Do not claim tests were run unless the host reports test output.
- Prefer small, reviewable proposed changes. The host owns compile, test, persist, merge, and any external side effects.
"#;

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
    SearchCode(Result<SearchCodeArgs, String>),
    ReadCode(Result<ReadCodeArgs, String>),
    ProposeChange(Result<ProposeChangeArgs, String>),
    Unsupported(String),
}

#[derive(Debug)]
struct SearchCodeArgs {
    query: String,
    limit: Option<u64>,
}

#[derive(Debug)]
struct ReadCodeArgs {
    path: String,
    start_line: Option<u64>,
    max_lines: Option<u64>,
}

#[derive(Debug)]
struct ProposeChangeArgs {
    path: String,
    content: String,
    note: Option<String>,
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

#[derive(Debug, Clone)]
struct ProposedChange {
    path: String,
    bytes: usize,
    note: Option<String>,
}

#[derive(Debug)]
struct WorkspaceEngine {
    root: PathBuf,
    vfs: VirtualFileSystem,
    proposals: Vec<ProposedChange>,
}

impl WorkspaceEngine {
    fn load_current() -> Result<Self, String> {
        let root = std::env::current_dir().map_err(|error| error.to_string())?;
        Self::load(root)
    }

    fn load(root: PathBuf) -> Result<Self, String> {
        let root_string = root.to_string_lossy().into_owned();
        let vfs = load_vfs_from_dir(&root_string, WORKSPACE_EXCLUDE_DIRS)?;
        Ok(Self {
            root,
            vfs,
            proposals: Vec::new(),
        })
    }

    fn search_code(&self, query: &str, limit: usize) -> Result<String, String> {
        let query = query.trim();
        if query.is_empty() {
            return Err("query must not be empty".to_string());
        }
        let limit = limit.clamp(1, MAX_SEARCH_MATCHES);
        let needle = query.to_lowercase();
        let mut matches = Vec::new();

        for (path, bytes) in self.vfs.files() {
            let Ok(text) = std::str::from_utf8(bytes) else {
                continue;
            };
            for (line_index, line) in text.lines().enumerate() {
                if !line.to_lowercase().contains(&needle) {
                    continue;
                }
                matches.push(format!("{path}:{}: {}", line_index + 1, line.trim()));
                if matches.len() >= limit {
                    break;
                }
            }
            if matches.len() >= limit {
                break;
            }
        }

        if matches.is_empty() {
            Ok(format!("no matches for {query:?}"))
        } else {
            Ok(format!(
                "workspace: {}\n{} match(es):\n{}",
                self.root.display(),
                matches.len(),
                matches.join("\n")
            ))
        }
    }

    fn read_code(
        &self,
        path: &str,
        start_line: Option<usize>,
        max_lines: Option<usize>,
    ) -> Result<String, String> {
        let path = workspace_relative_path(path)?;
        let text = self
            .vfs
            .read_str(&path)
            .ok_or_else(|| format!("file not found or not utf-8: {path}"))?;
        let start_line = start_line.unwrap_or(1).max(1);
        let max_lines = max_lines.unwrap_or(240).clamp(1, 800);
        let mut out = String::new();
        out.push_str(&format!("file: {path}\n"));
        let mut emitted = 0usize;
        for (line_index, line) in text.lines().enumerate().skip(start_line - 1) {
            if emitted >= max_lines || out.len() >= MAX_CONTEXT_BYTES {
                break;
            }
            out.push_str(&format!("{:>5} | {}\n", line_index + 1, line));
            emitted += 1;
        }
        if emitted == 0 {
            out.push_str("[no lines in requested range]\n");
        }
        if out.len() >= MAX_CONTEXT_BYTES {
            out.push_str("[context truncated]\n");
        }
        Ok(out)
    }

    fn propose_change(
        &mut self,
        path: &str,
        content: &str,
        note: Option<String>,
    ) -> Result<String, String> {
        if content.len() > MAX_PROPOSAL_BYTES {
            return Err(format!(
                "proposal is {} bytes; limit is {} bytes",
                content.len(),
                MAX_PROPOSAL_BYTES
            ));
        }
        let path = workspace_relative_path(path)?;
        self.vfs.write(&path, content.to_string())?;
        self.proposals.push(ProposedChange {
            path: path.clone(),
            bytes: content.len(),
            note,
        });
        let count = self.proposals.len();
        let last = self.proposals.last().expect("proposal was just pushed");
        let note = last
            .note
            .as_deref()
            .filter(|note| !note.trim().is_empty())
            .unwrap_or("no note");
        Ok(format!(
            "accepted in-memory proposal #{count}: {} ({} bytes)\nnote: {note}\nNo disk files were changed.",
            last.path, last.bytes
        ))
    }
}

fn workspace_relative_path(path: &str) -> Result<String, String> {
    let path = path.trim().trim_matches('/');
    if path.is_empty() {
        return Err("path must not be empty".to_string());
    }
    if path.starts_with('\\')
        || path.contains('\\')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err("path must stay inside the workspace".to_string());
    }
    Ok(path.to_string())
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
    let workspace = Arc::new(Mutex::new(WorkspaceEngine::load_current()?));
    let runtime = edgerun_tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    if let Some(prompt) = args.prompt {
        runtime.block_on(run_prompt(
            &client,
            workspace.clone(),
            vec![user_item(prompt)],
        ))?;
        return Ok(());
    }

    println!("Native EdgeRun Codex. Tools enabled: search_code, read_code, propose_change.");
    println!("Direct shell execution and disk edits are disabled in the agent loop.");
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
        history = runtime.block_on(run_prompt(&client, workspace.clone(), history))?;
    }
    Ok(())
}

async fn run_prompt(
    client: &codex_core::ModelClient,
    workspace: Arc<Mutex<WorkspaceEngine>>,
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
    let (_, history) = run_agent_loop(client, workspace, input, Some(&mut printer), None).await?;
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
            tools: cached_workspace_agent_tools(),
            parallel_tool_calls: true,
            base_instructions: codex_core::protocol::models::BaseInstructions {
                text: format!(
                    "{}\n\n{}",
                    include_str!("../../core/gpt_5_codex_prompt.md"),
                    WORKSPACE_TOOL_INSTRUCTIONS
                ),
            },
            ..Prompt::default()
        },
        store: false,
        ..TurnRequest::default()
    }
}

async fn run_agent_loop(
    client: &codex_core::ModelClient,
    workspace: Arc<Mutex<WorkspaceEngine>>,
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
                emit(AgentEvent::ToolStarted(invocation.display_name.clone()));
            }
            let result = execute_tool_invocation(&invocation, &workspace).await;
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
}

async fn stream_turn_with_events(
    client: &codex_core::ModelClient,
    request: TurnRequest,
    emit: &mut Option<&mut dyn FnMut(AgentEvent)>,
    cancel: Option<&AtomicBool>,
) -> Result<StreamedTurnOutput, Box<dyn Error>> {
    let mut stream = client.stream_turn(request).await?;
    let mut output = TurnOutput::default();

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

    Ok(StreamedTurnOutput { output })
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
            _ => None,
        })
        .collect()
}

fn function_tool_invocation(name: &str, arguments: &str, call_id: &str) -> ToolInvocation {
    let kind = match name {
        "search_code" => ToolInvocationKind::SearchCode(parse_search_code_args(arguments)),
        "read_code" => ToolInvocationKind::ReadCode(parse_read_code_args(arguments)),
        "propose_change" => ToolInvocationKind::ProposeChange(parse_propose_change_args(arguments)),
        "shell_command" | "shell" | "container.exec" | "apply_patch" => {
            ToolInvocationKind::Unsupported(format!(
                "{name} is disabled. Use search_code, read_code, and propose_change; the host owns execution and persistence."
            ))
        }
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
        "apply_patch" => ToolInvocationKind::Unsupported(
            "apply_patch is disabled. Use propose_change for in-memory edits.".to_string(),
        ),
        "propose_change" => ToolInvocationKind::ProposeChange(Ok(ProposeChangeArgs {
            path: "proposal.txt".to_string(),
            content: input.to_string(),
            note: Some("custom propose_change input".to_string()),
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
    workspace: &Arc<Mutex<WorkspaceEngine>>,
) -> ToolExecutionResult {
    let result = match &invocation.kind {
        ToolInvocationKind::SearchCode(Ok(args)) => workspace
            .lock()
            .map_err(|_| "workspace lock poisoned".to_string())
            .and_then(|workspace| {
                workspace.search_code(
                    &args.query,
                    args.limit
                        .and_then(|limit| usize::try_from(limit).ok())
                        .unwrap_or(40),
                )
            }),
        ToolInvocationKind::ReadCode(Ok(args)) => workspace
            .lock()
            .map_err(|_| "workspace lock poisoned".to_string())
            .and_then(|workspace| {
                workspace.read_code(
                    &args.path,
                    args.start_line.and_then(|line| usize::try_from(line).ok()),
                    args.max_lines.and_then(|lines| usize::try_from(lines).ok()),
                )
            }),
        ToolInvocationKind::ProposeChange(Ok(args)) => workspace
            .lock()
            .map_err(|_| "workspace lock poisoned".to_string())
            .and_then(|mut workspace| {
                workspace.propose_change(&args.path, &args.content, args.note.clone())
            }),
        ToolInvocationKind::SearchCode(Err(error))
        | ToolInvocationKind::ReadCode(Err(error))
        | ToolInvocationKind::ProposeChange(Err(error)) => Err(error.clone()),
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

fn parse_search_code_args(arguments: &str) -> Result<SearchCodeArgs, String> {
    let params = parse_tool_arguments(arguments)?;
    Ok(SearchCodeArgs {
        query: json_required_string(&params, "query")?.to_string(),
        limit: json_optional_u64(&params, "limit"),
    })
}

fn parse_read_code_args(arguments: &str) -> Result<ReadCodeArgs, String> {
    let params = parse_tool_arguments(arguments)?;
    Ok(ReadCodeArgs {
        path: json_required_string(&params, "path")?.to_string(),
        start_line: json_optional_u64(&params, "start_line"),
        max_lines: json_optional_u64(&params, "max_lines"),
    })
}

fn parse_propose_change_args(arguments: &str) -> Result<ProposeChangeArgs, String> {
    let args = parse_tool_arguments(arguments)?;
    Ok(ProposeChangeArgs {
        path: json_required_string(&args, "path")?.to_string(),
        content: json_required_string(&args, "content")?.to_string(),
        note: json_optional_string(&args, "note").map(ToString::to_string),
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

fn json_optional_u64(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
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

fn truncate_tool_output(mut text: String) -> String {
    if text.len() <= MAX_TOOL_OUTPUT_BYTES {
        return text;
    }
    text.truncate(MAX_TOOL_OUTPUT_BYTES);
    text.push_str("\n[output truncated]");
    text
}

fn cached_workspace_agent_tools() -> Vec<ToolSpec> {
    static TOOLS: OnceLock<Vec<ToolSpec>> = OnceLock::new();
    TOOLS.get_or_init(workspace_agent_tools).clone()
}

fn workspace_agent_tools() -> Vec<ToolSpec> {
    vec![search_code_tool(), read_code_tool(), propose_change_tool()]
}

fn search_code_tool() -> ToolSpec {
    let properties = BTreeMap::from([
        (
            "query".to_string(),
            JsonSchema::string(Some(
                "Text to search for in UTF-8 workspace files".to_string(),
            )),
        ),
        (
            "limit".to_string(),
            JsonSchema::number(Some(
                "Maximum number of matching lines to return".to_string(),
            )),
        ),
    ]);
    ToolSpec::Function(ResponsesApiTool {
        name: "search_code".to_string(),
        description: "Searches the in-memory workspace snapshot. No disk or process access."
            .to_string(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(
            properties,
            Some(vec!["query".to_string()]),
            Some(AdditionalProperties::Boolean(false)),
        ),
        output_schema: None,
    })
}

fn read_code_tool() -> ToolSpec {
    let properties = BTreeMap::from([
        (
            "path".to_string(),
            JsonSchema::string(Some(
                "Workspace-relative UTF-8 file path to read".to_string(),
            )),
        ),
        (
            "start_line".to_string(),
            JsonSchema::number(Some("One-based first line to return".to_string())),
        ),
        (
            "max_lines".to_string(),
            JsonSchema::number(Some("Maximum number of lines to return".to_string())),
        ),
    ]);
    ToolSpec::Function(ResponsesApiTool {
        name: "read_code".to_string(),
        description: "Reads a UTF-8 file from the in-memory workspace snapshot.".to_string(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(
            properties,
            Some(vec!["path".to_string()]),
            Some(AdditionalProperties::Boolean(false)),
        ),
        output_schema: None,
    })
}

fn propose_change_tool() -> ToolSpec {
    let properties = BTreeMap::from([
        (
            "path".to_string(),
            JsonSchema::string(Some(
                "Workspace-relative file path to replace in memory".to_string(),
            )),
        ),
        (
            "content".to_string(),
            JsonSchema::string(Some(
                "Complete proposed file contents. This is stored only in memory.".to_string(),
            )),
        ),
        (
            "note".to_string(),
            JsonSchema::string(Some("Short explanation of the proposed change".to_string())),
        ),
    ]);
    ToolSpec::Function(ResponsesApiTool {
        name: "propose_change".to_string(),
        description:
            "Stages a complete-file replacement in the in-memory workspace. Does not write to disk."
                .to_string(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(
            properties,
            Some(vec!["path".to_string(), "content".to_string()]),
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
    use super::WorkspaceEngine;
    use super::execute_tool_invocation;
    use super::function_tool_invocation;
    use super::json_required_string;
    use super::parse_tool_arguments;
    use std::fs;
    use std::sync::Arc;
    use std::sync::Mutex;

    #[test]
    fn parse_tool_arguments_accepts_normal_json() {
        let args = parse_tool_arguments(r#"{"query":"Codex"}"#).expect("parse args");

        assert_eq!(
            json_required_string(&args, "query").expect("query"),
            "Codex"
        );
    }

    #[test]
    fn parse_tool_arguments_accepts_json_string_wrapped_object() {
        let args = parse_tool_arguments(r#""{\"query\":\"Codex\"}""#).expect("parse args");

        assert_eq!(
            json_required_string(&args, "query").expect("query"),
            "Codex"
        );
    }

    #[test]
    fn parse_tool_arguments_accepts_escaped_object_without_outer_quotes() {
        let args = parse_tool_arguments(r#"{\"query\":\"Codex\"}"#).expect("parse args");

        assert_eq!(
            json_required_string(&args, "query").expect("query"),
            "Codex"
        );
    }

    #[test]
    fn function_tool_invocation_parses_read_code_args() {
        let invocation = function_tool_invocation(
            "read_code",
            r#"{"path":"tui/src/main.rs","start_line":10,"max_lines":20}"#,
            "call-1",
        );

        let ToolInvocationKind::ReadCode(Ok(args)) = invocation.kind else {
            panic!("expected read_code args");
        };
        assert_eq!(args.path, "tui/src/main.rs");
        assert_eq!(args.start_line, Some(10));
        assert_eq!(args.max_lines, Some(20));
    }

    #[test]
    fn shell_tools_are_unsupported() {
        let invocation = function_tool_invocation(
            "shell_command",
            r#"{"command":"echo ok","workdir":"/tmp","timeout_ms":1000,"login":true}"#,
            "call-1",
        );

        let ToolInvocationKind::Unsupported(error) = invocation.kind else {
            panic!("expected unsupported shell command");
        };
        assert!(error.contains("disabled"));
    }

    #[test]
    fn workspace_proposals_stay_in_memory() {
        let root =
            std::env::temp_dir().join(format!("edgerun-codex-vfs-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).expect("create test dir");
        fs::write(root.join("src/lib.rs"), "pub fn before() {}\n").expect("write fixture");

        let workspace = Arc::new(Mutex::new(
            WorkspaceEngine::load(root.clone()).expect("load workspace"),
        ));
        let runtime = edgerun_tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");

        let search = function_tool_invocation("search_code", r#"{"query":"before"}"#, "call-1");
        let result = runtime.block_on(execute_tool_invocation(&search, &workspace));
        assert!(result.success);
        assert!(result.text.contains("src/lib.rs:1"));

        let proposal = function_tool_invocation(
            "propose_change",
            r#"{"path":"src/lib.rs","content":"pub fn after() {}\n","note":"rename fixture"}"#,
            "call-2",
        );
        let result = runtime.block_on(execute_tool_invocation(&proposal, &workspace));
        assert!(result.success);
        assert!(result.text.contains("No disk files were changed"));

        let read = function_tool_invocation("read_code", r#"{"path":"src/lib.rs"}"#, "call-3");
        let result = runtime.block_on(execute_tool_invocation(&read, &workspace));
        assert!(result.success);
        assert!(result.text.contains("pub fn after()"));

        assert!(
            fs::read_to_string(root.join("src/lib.rs"))
                .expect("read disk fixture")
                .contains("pub fn before()")
        );
        let _ = fs::remove_dir_all(&root);
    }
}
