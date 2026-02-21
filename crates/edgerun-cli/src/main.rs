// SPDX-License-Identifier: Apache-2.0
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::ffi::OsString;
use std::io::{self, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Result};
use axum::{
    extract::{Path as AxPath, State},
    http::HeaderMap,
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::{Mutex, Notify};
use tokio::time::sleep;

#[derive(Parser, Debug)]
#[command(name = "edgerun", about = "EdgeRun project orchestration CLI")]
struct Cli {
    #[arg(long, default_value = ".")]
    root: PathBuf,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Version,
    Doctor,
    Setup {
        #[arg(long)]
        install_missing: bool,
    },
    Build {
        #[arg(value_enum, default_value_t = BuildTarget::Workspace)]
        target: BuildTarget,
    },
    Test {
        #[arg(value_enum, default_value_t = TestTarget::Workspace)]
        target: TestTarget,
    },
    Run {
        #[arg(value_enum)]
        target: RunTarget,
    },
    Ci {
        #[arg(long)]
        job: Option<String>,
        #[arg(long, default_value = "pull_request")]
        event: String,
        #[arg(long)]
        dry_run: bool,
    },
    Dev,
    Install,
    All,
    Interactive,
    Serve {
        #[arg(long, default_value = "127.0.0.1:8787")]
        addr: String,
    },
    CompareReplay {
        left: PathBuf,
        right: PathBuf,
    },
    ValidateSecurity {
        #[arg(long)]
        path: Option<PathBuf>,
    },
    Storage {
        #[command(subcommand)]
        command: StorageCommand,
    },
}

#[derive(Clone, Debug, ValueEnum)]
enum BuildTarget {
    Workspace,
    Program,
    All,
}

#[derive(Clone, Debug, ValueEnum)]
enum TestTarget {
    Workspace,
    Runtime,
    Integration,
    E2e,
    Rotation,
    AbiRollover,
    Program,
    All,
}

#[derive(Clone, Debug, ValueEnum)]
enum RunTarget {
    FuzzWeekly,
    ReplayCorpus,
    SecurityReview,
}

#[derive(Subcommand, Debug, Clone)]
enum StorageCommand {
    Check,
    Test,
    PerfGate,
    Sweep {
        #[arg(long)]
        duration: Option<u64>,
        #[arg(long)]
        out_dir: Option<PathBuf>,
        #[arg(long)]
        max_cases: Option<usize>,
    },
    Crash {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    Bench {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    RepBench {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    EncDemo {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    CiSmoke,
}

#[derive(Clone)]
struct WebState {
    root: PathBuf,
    config: AppConfig,
    tasks: Arc<Mutex<HashMap<String, TaskStatus>>>,
    queue: Arc<Mutex<VecDeque<String>>>,
    running_cancel: Arc<Mutex<HashMap<String, Arc<Notify>>>>,
    dispatcher_running: Arc<AtomicBool>,
    state_file: PathBuf,
    runs_dir: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Default)]
struct AppConfig {
    #[serde(default)]
    web: WebConfig,
    #[serde(default)]
    runtime: RuntimeConfig,
    #[serde(default)]
    integration: IntegrationConfig,
}

#[derive(Clone, Debug, Deserialize, Default)]
struct WebConfig {
    default_addr: Option<String>,
    refresh_ms: Option<u64>,
    history_limit: Option<usize>,
    auth_token: Option<String>,
    allow_remote_bind: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Default)]
struct RuntimeConfig {
    replay_profile_debug: Option<String>,
    replay_profile_release: Option<String>,
    replay_runs: Option<u32>,
    fuzz_seconds_per_target: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Default)]
struct IntegrationConfig {
    require_worker_signatures: Option<bool>,
    require_result_attestation: Option<bool>,
    quorum_requires_attestation: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize)]
struct TaskStatus {
    task: String,
    state: String,
    started_at_unix_s: Option<u64>,
    finished_at_unix_s: Option<u64>,
    runs: u64,
    last_exit: Option<i32>,
    last_output: String,
    #[serde(default)]
    history: Vec<TaskRunRecord>,
    #[serde(default)]
    last_log_path: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
struct TaskRunRecord {
    started_at_unix_s: Option<u64>,
    finished_at_unix_s: Option<u64>,
    state: String,
    exit: Option<i32>,
    output: String,
    #[serde(default)]
    log_path: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct TaskStatusSnapshot {
    #[serde(default)]
    tasks: Vec<TaskStatus>,
}

#[derive(Serialize)]
struct ApiMessage {
    ok: bool,
    message: String,
}

#[derive(Serialize)]
struct StatusResponse {
    tasks: Vec<TaskStatus>,
}

const DEFAULT_HISTORY_LIMIT: usize = 30;
const CLI_BUILD_NUMBER: &str = match option_env!("EDGERUN_BUILD_NUMBER") {
    Some(v) => v,
    None => "dev",
};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = std::fs::canonicalize(&cli.root)
        .with_context(|| format!("failed to resolve root path: {}", cli.root.display()))?;
    let config = load_app_config(&root)?;

    match cli.command.unwrap_or(Commands::Interactive) {
        Commands::Version => {
            println!(
                "edgerun {} (build {})",
                env!("CARGO_PKG_VERSION"),
                CLI_BUILD_NUMBER
            );
        }
        Commands::Doctor => run_named_task_sync(&root, "doctor")?,
        Commands::Setup { install_missing } => {
            let name = if install_missing {
                "setup-install"
            } else {
                "setup"
            };
            run_named_task_sync(&root, name)?;
        }
        Commands::Build { target } => {
            run_build_target(&root, target)?;
        }
        Commands::Test { target } => {
            run_test_target(&root, target).await?;
        }
        Commands::Run { target } => run_run_target(&root, target, &config).await?,
        Commands::Ci {
            job,
            event,
            dry_run,
        } => run_ci(&root, job, event, dry_run).await?,
        Commands::Dev => run_named_task_sync(&root, "dev")?,
        Commands::Install => run_named_task_sync(&root, "install")?,
        Commands::All => run_named_task_sync(&root, "all")?,
        Commands::Interactive => run_interactive(&root).await?,
        Commands::Serve { addr } => run_server(root, addr, config).await?,
        Commands::CompareReplay { left, right } => compare_replay_profiles(&left, &right)?,
        Commands::ValidateSecurity { path } => {
            let p =
                path.unwrap_or_else(|| root.join("crates/edgerun-runtime/SECURITY_FINDINGS.json"));
            validate_external_security_review(&p)?
        }
        Commands::Storage { command } => run_storage_command(&root, command)?,
    }

    Ok(())
}

fn run_build_target(root: &Path, target: BuildTarget) -> Result<()> {
    match target {
        BuildTarget::Workspace => run_named_task_sync(root, "build-workspace"),
        BuildTarget::Program => run_named_task_sync(root, "build-program"),
        BuildTarget::All => {
            run_named_task_sync(root, "build-workspace")?;
            run_named_task_sync(root, "build-program")
        }
    }
}

async fn run_test_target(root: &Path, target: TestTarget) -> Result<()> {
    match target {
        TestTarget::Workspace => run_named_task_sync(root, "test-workspace"),
        TestTarget::Runtime => run_named_task_sync(root, "test-runtime"),
        TestTarget::Integration => run_integration_scheduler_api(root).await,
        TestTarget::E2e => run_integration_e2e_lifecycle(root).await,
        TestTarget::Rotation => run_integration_policy_rotation(root).await,
        TestTarget::AbiRollover => run_integration_abi_rollover(root).await,
        TestTarget::Program => run_named_task_sync(root, "test-program"),
        TestTarget::All => {
            run_named_task_sync(root, "test-runtime")?;
            run_integration_scheduler_api(root).await?;
            run_integration_e2e_lifecycle(root).await?;
            run_integration_policy_rotation(root).await?;
            run_integration_abi_rollover(root).await
        }
    }
}

async fn run_run_target(root: &Path, target: RunTarget, config: &AppConfig) -> Result<()> {
    match target {
        RunTarget::FuzzWeekly => run_weekly_fuzz(root, config).await,
        RunTarget::ReplayCorpus => run_replay_corpus(root, config).await,
        RunTarget::SecurityReview => validate_external_security_review(
            &root.join("crates/edgerun-runtime/SECURITY_FINDINGS.json"),
        ),
    }
}

async fn run_ci(root: &Path, job: Option<String>, event: String, dry_run: bool) -> Result<()> {
    let job_name = job.unwrap_or_else(|| "all".to_string());
    if dry_run {
        println!("event={event}");
        println!("job={job_name}");
        return Ok(());
    }

    if command_exists("act") {
        let mut args = vec![
            event,
            "-W".to_string(),
            ".github/workflows/ci.yml".to_string(),
            "-P".to_string(),
            "ubuntu-24.04=ghcr.io/catthehacker/ubuntu:act-24.04".to_string(),
        ];
        if job_name != "all" {
            args.push("-j".to_string());
            args.push(job_name);
        }
        return run_program_sync_owned("Run CI via act", "act", &args, root, false);
    }

    match job_name.as_str() {
        "all" => {
            run_rust_checks_sync(root)?;
            run_integration_scheduler_api(root).await?;
            run_integration_e2e_lifecycle(root).await?;
            run_integration_policy_rotation(root).await?;
            run_integration_abi_rollover(root).await?;
            run_replay_corpus(root, &load_app_config(root)?).await?;
            validate_external_security_review(
                &root.join("crates/edgerun-runtime/SECURITY_FINDINGS.json"),
            )?;
            Ok(())
        }
        "rust-checks" => run_rust_checks_sync(root),
        "integration" => {
            run_integration_scheduler_api(root).await?;
            run_integration_e2e_lifecycle(root).await?;
            run_integration_policy_rotation(root).await?;
            run_integration_abi_rollover(root).await
        }
        "runtime-determinism" => run_replay_corpus(root, &load_app_config(root)?).await,
        "runtime-security" => validate_external_security_review(
            &root.join("crates/edgerun-runtime/SECURITY_FINDINGS.json"),
        ),
        other => Err(anyhow!("unsupported ci job in native mode: {other}")),
    }
}

async fn run_interactive(root: &Path) -> Result<()> {
    let menu = [
        ("doctor", "Check local toolchain"),
        ("setup", "Setup dev dependencies"),
        ("setup-install", "Setup + install missing optional tools"),
        ("build-workspace", "Build Rust workspace"),
        ("test-runtime", "Run runtime tests"),
        ("test-integration", "Run scheduler API integration"),
        ("test-e2e", "Run e2e lifecycle integration"),
        ("test-rotation", "Run policy rotation integration"),
        ("test-abi-rollover", "Run ABI rollover integration"),
        ("run-replay-corpus", "Run replay corpus checks"),
        ("run-fuzz-weekly", "Run weekly fuzz targets"),
        ("storage-check", "Storage checks"),
        ("storage-test", "Storage tests"),
        ("storage-perf-gate", "Storage perf gate"),
        ("storage-sweep", "Storage mixed RW sweep"),
        ("storage-ci-smoke", "Storage CI smoke"),
        ("dev", "Run local dev check (make check)"),
        ("all", "Run broad default workflow"),
    ];

    loop {
        println!("\nEdgeRun interactive mode");
        for (i, (_, label)) in menu.iter().enumerate() {
            println!("{:>2}. {}", i + 1, label);
        }
        println!(" q. Quit");
        print!("Select action: ");
        io::stdout().flush().context("failed to flush stdout")?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .context("failed to read interactive input")?;
        let trimmed = input.trim();

        if trimmed.eq_ignore_ascii_case("q") {
            break;
        }

        let chosen = if let Ok(n) = trimmed.parse::<usize>() {
            menu.get(n.saturating_sub(1)).map(|(task, _)| *task)
        } else {
            menu.iter()
                .find(|(task, _)| task.eq_ignore_ascii_case(trimmed))
                .map(|(task, _)| *task)
        };

        match chosen {
            Some(task) => {
                if let Err(e) = run_named_task_async(root.to_path_buf(), task.to_string()).await {
                    eprintln!("task failed: {e:#}");
                }
            }
            None => eprintln!("unknown selection: {trimmed}"),
        }
    }

    Ok(())
}

async fn run_server(root: PathBuf, addr: String, config: AppConfig) -> Result<()> {
    let bind_addr = if addr == "127.0.0.1:8787" {
        config
            .web
            .default_addr
            .clone()
            .unwrap_or_else(|| addr.clone())
    } else {
        addr
    };
    let allow_remote_bind = config.web.allow_remote_bind.unwrap_or(false);
    if !allow_remote_bind && !is_loopback_bind_addr(&bind_addr) {
        bail!(
            "refusing to bind web server to non-loopback address {bind_addr}; set [web].allow_remote_bind = true to override"
        );
    }
    let state_dir = root.join(".edgerun-cli");
    std::fs::create_dir_all(&state_dir)
        .with_context(|| format!("failed to create {}", state_dir.display()))?;
    let state_file = state_dir.join("task-status.json");
    let runs_dir = state_dir.join("runs");
    std::fs::create_dir_all(&runs_dir)
        .with_context(|| format!("failed to create {}", runs_dir.display()))?;
    let mut tasks = load_task_statuses(&state_file)?;
    let history_limit = web_history_limit(&config);
    let trimmed = enforce_history_limit(&mut tasks, history_limit);
    if trimmed {
        persist_task_statuses(&state_file, &tasks)?;
    }

    let state = WebState {
        root,
        config: config.clone(),
        tasks: Arc::new(Mutex::new(tasks)),
        queue: Arc::new(Mutex::new(VecDeque::new())),
        running_cancel: Arc::new(Mutex::new(HashMap::new())),
        dispatcher_running: Arc::new(AtomicBool::new(false)),
        state_file,
        runs_dir,
    };

    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("failed to bind web server at {bind_addr}"))?;

    println!("edgerun web control panel: http://{bind_addr}");
    axum::serve(listener, app)
        .await
        .context("web server failed")?;
    Ok(())
}

fn build_router(state: WebState) -> Router {
    Router::new()
        .route("/", get(ui_page))
        .route("/api/status", get(api_status))
        .route("/api/run/{task}", post(api_run_task))
        .route("/api/cancel/{task}", post(api_cancel_task))
        .route("/api/log/{task}", get(api_task_log))
        .with_state(state)
}

async fn ui_page(State(state): State<WebState>) -> Html<String> {
    let refresh_ms = state.config.web.refresh_ms.unwrap_or(2000).max(500);
    let history_limit = web_history_limit(&state.config);
    let token_required = state.config.web.auth_token.is_some();
    Html(format!(
        r#"<!doctype html>
<html>
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>EdgeRun Control Panel</title>
  <style>
    :root {{
      --bg: #f8f4eb;
      --panel: #fffdf7;
      --ink: #1f2933;
      --muted: #64748b;
      --ok: #1f7a4a;
      --fail: #9f1239;
      --run: #9a6700;
      --accent: #0f5f9c;
      --border: #d7c8a2;
    }}
    body {{ font-family: "IBM Plex Sans", "Trebuchet MS", sans-serif; background: radial-gradient(circle at top right, #f2e5c3, var(--bg)); color: var(--ink); margin: 0; }}
    .wrap {{ max-width: 1200px; margin: 0 auto; padding: 24px; }}
    h1 {{ margin-top: 0; letter-spacing: 0.02em; }}
    .toolbar {{ display: flex; flex-wrap: wrap; gap: 10px; margin-bottom: 12px; }}
    .toolbar input, .toolbar select {{ padding: 8px; border: 1px solid var(--border); border-radius: 8px; background: var(--panel); }}
    .summary {{ margin-bottom: 12px; color: var(--muted); }}
    .grid {{ display: grid; gap: 12px; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); }}
    .card {{ background: var(--panel); border: 1px solid var(--border); border-radius: 12px; padding: 12px; box-shadow: 0 6px 20px rgba(15,95,156,0.08); }}
    .task {{ font-family: "IBM Plex Mono", "Consolas", monospace; font-size: 0.9rem; }}
    button {{ border: 0; background: var(--accent); color: white; padding: 8px 10px; border-radius: 8px; cursor: pointer; }}
    .row {{ display: flex; gap: 8px; margin-top: 6px; }}
    .linkbtn {{ display: inline-block; text-decoration: none; color: var(--accent); background: #e8f2fb; padding: 8px 10px; border-radius: 8px; }}
    button:disabled {{ opacity: 0.55; cursor: not-allowed; }}
    .state-idle {{ color: var(--muted); }}
    .state-running {{ color: var(--run); font-weight: 700; }}
    .state-success {{ color: var(--ok); font-weight: 700; }}
    .state-failed {{ color: var(--fail); font-weight: 700; }}
    .muted {{ color: var(--muted); font-size: 0.85rem; }}
    pre {{ white-space: pre-wrap; background: #f4efe2; border-radius: 8px; padding: 8px; max-height: 180px; overflow: auto; }}
  </style>
</head>
<body>
  <div class="wrap">
    <h1>EdgeRun Control Panel</h1>
    <p>Trigger core project workflows and monitor latest status.</p>
    <div class="toolbar">
      <input id="search" placeholder="Search task..." oninput="refresh()" />
      <select id="group" onchange="refresh()">
        <option value="all">All Groups</option>
        <option value="Core">Core</option>
        <option value="Build">Build</option>
        <option value="Test">Test</option>
        <option value="Run">Run</option>
        <option value="Storage">Storage</option>
        <option value="CI">CI</option>
      </select>
      <select id="stateFilter" onchange="refresh()">
        <option value="all">All States</option>
        <option value="running">Running</option>
        <option value="failed">Failed</option>
        <option value="success">Success</option>
        <option value="idle">Idle</option>
      </select>
    </div>
    <div id="summary" class="summary"></div>
    <div class="summary">history retention: last {history_limit} runs per task</div>
    <div id="tasks" class="grid"></div>
  </div>
<script>
const TOKEN_REQUIRED = {token_required};
const TASKS = [
  {{ id: "doctor", label: "Doctor", group: "Core" }},
  {{ id: "setup", label: "Setup", group: "Core" }},
  {{ id: "setup-install", label: "Setup + Install Missing", group: "Core" }},
  {{ id: "dev", label: "Dev Check", group: "Core" }},
  {{ id: "install", label: "Install CLI", group: "Core" }},
  {{ id: "all", label: "Default All", group: "Core" }},

  {{ id: "build-workspace", label: "Build Workspace", group: "Build" }},
  {{ id: "build-program", label: "Build Program", group: "Build" }},
  {{ id: "build-all", label: "Build All", group: "Build" }},

  {{ id: "test-workspace", label: "Test Workspace", group: "Test" }},
  {{ id: "test-runtime", label: "Test Runtime", group: "Test" }},
  {{ id: "test-integration", label: "Test Integration API", group: "Test" }},
  {{ id: "test-e2e", label: "Test E2E", group: "Test" }},
  {{ id: "test-rotation", label: "Test Policy Rotation", group: "Test" }},
  {{ id: "test-abi-rollover", label: "Test ABI Rollover", group: "Test" }},
  {{ id: "test-program", label: "Test Program", group: "Test" }},
  {{ id: "test-all", label: "Test All", group: "Test" }},

  {{ id: "run-fuzz-weekly", label: "Run Weekly Fuzz", group: "Run" }},
  {{ id: "run-replay-corpus", label: "Run Replay Corpus", group: "Run" }},
  {{ id: "run-security-review", label: "Run Security Review", group: "Run" }},

  {{ id: "storage-check", label: "Storage Check", group: "Storage" }},
  {{ id: "storage-test", label: "Storage Test", group: "Storage" }},
  {{ id: "storage-perf-gate", label: "Storage Perf Gate", group: "Storage" }},
  {{ id: "storage-sweep", label: "Storage Sweep", group: "Storage" }},
  {{ id: "storage-ci-smoke", label: "Storage CI Smoke", group: "Storage" }},

  {{ id: "ci-all", label: "CI All", group: "CI" }},
  {{ id: "ci-rust-checks", label: "CI Rust Checks", group: "CI" }},
  {{ id: "ci-integration", label: "CI Integration", group: "CI" }},
  {{ id: "ci-runtime-determinism", label: "CI Runtime Determinism", group: "CI" }},
  {{ id: "ci-runtime-security", label: "CI Runtime Security", group: "CI" }}
];

async function runTask(task) {{
  await fetch(`/api/run/${{task}}`, {{ method: 'POST' }});
  await refresh();
}}

function renderCard(meta, data) {{
  const stateCls = `state-${{data.state}}`;
  const disabled = data.state === 'running' ? 'disabled' : '';
  return `<div class="card">
    <div class="task">${{meta.label}}</div>
    <div class="muted">${{meta.group}} · ${{data.task}}</div>
    <div class="${{stateCls}}">state: ${{data.state}}</div>
    <div>runs: ${{data.runs}}</div>
    <div>history: ${{(data.history || []).length}}</div>
    <div>exit: ${{data.last_exit ?? '-'}}</div>
    <div class="muted">${{data.last_log_path ? data.last_log_path : 'no log yet'}}</div>
    <div class="row">
      <button ${{disabled}} onclick="runTask('${{data.task}}')">Run</button>
      <a class="linkbtn" href="/api/log/${{data.task}}" target="_blank">Open Log</a>
    </div>
    <pre>${{(data.last_output || '').slice(-2000)}}</pre>
  </div>`;
}}

function apiHeaders() {{
  const token = (document.getElementById('token')?.value || '');
  return token ? {{ 'x-edgerun-token': token }} : {{}};
}}

async function refresh() {{
  const res = await fetch('/api/status', {{ headers: apiHeaders() }});
  if (res.status === 401) {{
    document.getElementById('summary').innerText = TOKEN_REQUIRED
      ? 'unauthorized: provide API token'
      : 'unauthorized';
    document.getElementById('tasks').innerHTML = '';
    return;
  }}
  const body = await res.json();
  const map = new Map(body.tasks.map(t => [t.task, t]));
  let merged = TASKS.map(meta => {{
    const task = map.get(meta.id) || ({{
      task: meta.id,
      state: 'idle',
      runs: 0,
      last_output: '',
      history: []
    }});
    return {{ meta, task }};
  }});
  const search = (document.getElementById('search').value || '').toLowerCase();
  const group = document.getElementById('group').value;
  const stateFilter = document.getElementById('stateFilter').value;
  merged = merged.filter(x => {{
    if (group !== 'all' && x.meta.group !== group) return false;
    if (stateFilter !== 'all' && x.task.state !== stateFilter) return false;
    const hay = `${{x.meta.label}} ${{x.meta.group}} ${{x.task.task}}`.toLowerCase();
    return !search || hay.includes(search);
  }});
  const counts = {{
    total: merged.length,
    running: merged.filter(x => x.task.state === 'running').length,
    failed: merged.filter(x => x.task.state === 'failed').length,
    success: merged.filter(x => x.task.state === 'success').length
  }};
  document.getElementById('summary').innerText =
    `visible=${{counts.total}} running=${{counts.running}} failed=${{counts.failed}} success=${{counts.success}}`;
  document.getElementById('tasks').innerHTML = merged.map(x => renderCard(x.meta, x.task)).join('');
}}

setInterval(refresh, {refresh_ms});
refresh();
</script>
</body>
</html>"#
    ))
}

async fn api_status(State(state): State<WebState>, headers: HeaderMap) -> impl IntoResponse {
    if !is_request_authorized(&state.config, &headers) {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(ApiMessage {
                ok: false,
                message: "unauthorized".to_string(),
            }),
        )
            .into_response();
    }
    let tasks = state.tasks.lock().await;
    let mut entries: Vec<TaskStatus> = tasks.values().cloned().collect();
    entries.sort_by(|a, b| a.task.cmp(&b.task));
    Json(StatusResponse { tasks: entries }).into_response()
}

async fn api_task_log(
    State(state): State<WebState>,
    AxPath(task): AxPath<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !is_request_authorized(&state.config, &headers) {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            "unauthorized".to_string(),
        );
    }
    let tasks = state.tasks.lock().await;
    let Some(status) = tasks.get(&task) else {
        return (
            axum::http::StatusCode::NOT_FOUND,
            "unknown task".to_string(),
        );
    };
    let Some(path) = status.last_log_path.as_deref() else {
        return (
            axum::http::StatusCode::NOT_FOUND,
            "no log for task".to_string(),
        );
    };
    match std::fs::read_to_string(path) {
        Ok(content) => (axum::http::StatusCode::OK, content),
        Err(err) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to read log: {err}"),
        ),
    }
}

async fn api_run_task(
    State(state): State<WebState>,
    AxPath(task): AxPath<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !is_request_authorized(&state.config, &headers) {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(ApiMessage {
                ok: false,
                message: "unauthorized".to_string(),
            }),
        );
    }
    if !is_supported_task(&task) {
        return (
            axum::http::StatusCode::NOT_FOUND,
            Json(ApiMessage {
                ok: false,
                message: format!("unknown task: {task}"),
            }),
        );
    }

    {
        let mut tasks = state.tasks.lock().await;
        let entry = tasks.entry(task.clone()).or_insert_with(|| TaskStatus {
            task: task.clone(),
            state: "idle".to_string(),
            started_at_unix_s: None,
            finished_at_unix_s: None,
            runs: 0,
            last_exit: None,
            last_output: String::new(),
            history: Vec::new(),
            last_log_path: None,
        });
        if matches!(entry.state.as_str(), "running" | "queued" | "canceling") {
            return (
                axum::http::StatusCode::CONFLICT,
                Json(ApiMessage {
                    ok: false,
                    message: format!("task already active: {task}"),
                }),
            );
        }
        entry.state = "queued".to_string();
        entry.started_at_unix_s = None;
        entry.finished_at_unix_s = None;
        entry.last_output = "queued".to_string();
        if let Err(err) = persist_task_statuses(&state.state_file, &tasks) {
            eprintln!("failed to persist task state: {err:#}");
        }
    }
    {
        let mut queue = state.queue.lock().await;
        queue.push_back(task.clone());
    }
    try_start_next_queued_task(state.clone()).await;

    (
        axum::http::StatusCode::ACCEPTED,
        Json(ApiMessage {
            ok: true,
            message: format!("task queued: {task}"),
        }),
    )
}

async fn api_cancel_task(
    State(state): State<WebState>,
    AxPath(task): AxPath<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !is_request_authorized(&state.config, &headers) {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(ApiMessage {
                ok: false,
                message: "unauthorized".to_string(),
            }),
        );
    }
    let mut queue = state.queue.lock().await;
    if let Some(pos) = queue.iter().position(|t| t == &task) {
        queue.remove(pos);
        drop(queue);
        let mut tasks = state.tasks.lock().await;
        if let Some(entry) = tasks.get_mut(&task) {
            entry.state = "canceled".to_string();
            entry.last_exit = Some(130);
            entry.finished_at_unix_s = Some(now_unix_s());
            entry.last_output = "canceled while queued".to_string();
            entry.history.push(TaskRunRecord {
                started_at_unix_s: entry.started_at_unix_s,
                finished_at_unix_s: entry.finished_at_unix_s,
                state: entry.state.clone(),
                exit: entry.last_exit,
                output: entry.last_output.clone(),
                log_path: entry.last_log_path.clone(),
            });
            trim_history(&mut entry.history, web_history_limit(&state.config));
        }
        if let Err(err) = persist_task_statuses(&state.state_file, &tasks) {
            eprintln!("failed to persist task state: {err:#}");
        }
        drop(tasks);
        try_start_next_queued_task(state.clone()).await;
        return (
            axum::http::StatusCode::ACCEPTED,
            Json(ApiMessage {
                ok: true,
                message: format!("task canceled from queue: {task}"),
            }),
        );
    }
    drop(queue);

    {
        let mut tasks = state.tasks.lock().await;
        if let Some(entry) = tasks.get_mut(&task) {
            if entry.state == "queued" {
                entry.state = "canceled".to_string();
                entry.last_exit = Some(130);
                entry.finished_at_unix_s = Some(now_unix_s());
                entry.last_output = "canceled before dispatch".to_string();
                entry.history.push(TaskRunRecord {
                    started_at_unix_s: entry.started_at_unix_s,
                    finished_at_unix_s: entry.finished_at_unix_s,
                    state: entry.state.clone(),
                    exit: entry.last_exit,
                    output: entry.last_output.clone(),
                    log_path: entry.last_log_path.clone(),
                });
                trim_history(&mut entry.history, web_history_limit(&state.config));
                if let Err(err) = persist_task_statuses(&state.state_file, &tasks) {
                    eprintln!("failed to persist task state: {err:#}");
                }
                return (
                    axum::http::StatusCode::ACCEPTED,
                    Json(ApiMessage {
                        ok: true,
                        message: format!("task canceled before dispatch: {task}"),
                    }),
                );
            }
        }
    }

    let cancel_notify = {
        let running_cancel = state.running_cancel.lock().await;
        running_cancel.get(&task).cloned()
    };
    if let Some(cancel_notify) = cancel_notify {
        cancel_notify.notify_waiters();
        let mut tasks = state.tasks.lock().await;
        if let Some(entry) = tasks.get_mut(&task) {
            entry.state = "canceling".to_string();
            entry.last_output = "cancel requested".to_string();
        }
        if let Err(err) = persist_task_statuses(&state.state_file, &tasks) {
            eprintln!("failed to persist task state: {err:#}");
        }
        return (
            axum::http::StatusCode::ACCEPTED,
            Json(ApiMessage {
                ok: true,
                message: format!("task cancel requested: {task}"),
            }),
        );
    }
    (
        axum::http::StatusCode::NOT_FOUND,
        Json(ApiMessage {
            ok: false,
            message: format!("task not active: {task}"),
        }),
    )
}

fn is_supported_task(task: &str) -> bool {
    matches!(
        task,
        "doctor"
            | "setup"
            | "setup-install"
            | "build-workspace"
            | "build-program"
            | "build-all"
            | "test-workspace"
            | "test-runtime"
            | "test-integration"
            | "test-e2e"
            | "test-rotation"
            | "test-abi-rollover"
            | "test-program"
            | "test-all"
            | "run-fuzz-weekly"
            | "run-replay-corpus"
            | "run-security-review"
            | "storage-check"
            | "storage-test"
            | "storage-perf-gate"
            | "storage-sweep"
            | "storage-ci-smoke"
            | "ci-all"
            | "ci-rust-checks"
            | "ci-integration"
            | "ci-runtime-determinism"
            | "ci-runtime-security"
            | "dev"
            | "install"
            | "all"
    )
}

fn has_active_task(tasks: &HashMap<String, TaskStatus>) -> bool {
    tasks
        .values()
        .any(|t| matches!(t.state.as_str(), "running" | "canceling"))
}

async fn try_start_next_queued_task(state: WebState) {
    if state.dispatcher_running.swap(true, Ordering::AcqRel) {
        return;
    }

    let has_active = {
        let tasks = state.tasks.lock().await;
        has_active_task(&tasks)
    };
    if has_active {
        state.dispatcher_running.store(false, Ordering::Release);
        return;
    }

    let next_task = {
        let mut queue = state.queue.lock().await;
        queue.pop_front()
    };
    if let Some(task) = next_task {
        if let Err(err) = spawn_task_execution(state.clone(), task).await {
            eprintln!("failed to spawn queued task: {err:#}");
        }
    }
    state.dispatcher_running.store(false, Ordering::Release);
}

fn schedule_dispatch(state: WebState) {
    tokio::spawn(async move {
        try_start_next_queued_task(state).await;
    });
}

async fn spawn_task_execution(state: WebState, task: String) -> Result<()> {
    let cancel_notify = Arc::new(Notify::new());
    {
        let mut tasks = state.tasks.lock().await;
        let Some(entry) = tasks.get_mut(&task) else {
            return Ok(());
        };
        if entry.state != "queued" {
            return Ok(());
        }
        entry.state = "running".to_string();
        entry.started_at_unix_s = Some(now_unix_s());
        entry.finished_at_unix_s = None;
        entry.runs += 1;
        if let Err(err) = persist_task_statuses(&state.state_file, &tasks) {
            eprintln!("failed to persist task state: {err:#}");
        }
    }
    {
        let mut running_cancel = state.running_cancel.lock().await;
        running_cancel.insert(task.clone(), cancel_notify.clone());
    }

    let state_clone = state.clone();
    let history_limit = web_history_limit(&state.config);
    tokio::spawn(async move {
        let (task_state, last_exit, summary) =
            match run_task_subprocess_capture(&state_clone.root, &task, cancel_notify).await {
                Ok((0, output)) => ("success".to_string(), Some(0), output),
                Ok((130, output)) => ("canceled".to_string(), Some(130), output),
                Ok((exit, output)) => ("failed".to_string(), Some(exit), output),
                Err(err) => ("failed".to_string(), Some(1), format!("ERROR: {err:#}")),
            };

        let mut tasks = state_clone.tasks.lock().await;
        if let Some(entry) = tasks.get_mut(&task) {
            entry.state = task_state;
            entry.last_exit = last_exit;
            entry.finished_at_unix_s = Some(now_unix_s());
            let full_output = summary;
            entry.last_output = truncate_output(full_output.clone(), 12_000);
            let log_path = match persist_run_log(
                &state_clone.runs_dir,
                &entry.task,
                entry.started_at_unix_s,
                entry.finished_at_unix_s,
                &entry.state,
                &full_output,
            ) {
                Ok(path) => Some(path),
                Err(err) => {
                    eprintln!("failed to persist run log: {err:#}");
                    None
                }
            };
            entry.history.push(TaskRunRecord {
                started_at_unix_s: entry.started_at_unix_s,
                finished_at_unix_s: entry.finished_at_unix_s,
                state: entry.state.clone(),
                exit: entry.last_exit,
                output: truncate_output(entry.last_output.clone(), 4_000),
                log_path: log_path.clone(),
            });
            entry.last_log_path = log_path;
            trim_history(&mut entry.history, history_limit);
        }
        if let Err(err) = persist_task_statuses(&state_clone.state_file, &tasks) {
            eprintln!("failed to persist task state: {err:#}");
        }
        drop(tasks);
        let mut running_cancel = state_clone.running_cancel.lock().await;
        running_cancel.remove(&task);
        drop(running_cancel);
        schedule_dispatch(state_clone);
    });
    Ok(())
}

fn task_to_cli_args(task: &str) -> Option<Vec<&'static str>> {
    match task {
        "doctor" => Some(vec!["doctor"]),
        "setup" => Some(vec!["setup"]),
        "setup-install" => Some(vec!["setup", "--install-missing"]),
        "build-workspace" => Some(vec!["build", "workspace"]),
        "build-program" => Some(vec!["build", "program"]),
        "build-all" => Some(vec!["build", "all"]),
        "test-workspace" => Some(vec!["test", "workspace"]),
        "test-runtime" => Some(vec!["test", "runtime"]),
        "test-integration" => Some(vec!["test", "integration"]),
        "test-e2e" => Some(vec!["test", "e2e"]),
        "test-rotation" => Some(vec!["test", "rotation"]),
        "test-abi-rollover" => Some(vec!["test", "abi-rollover"]),
        "test-program" => Some(vec!["test", "program"]),
        "test-all" => Some(vec!["test", "all"]),
        "run-fuzz-weekly" => Some(vec!["run", "fuzz-weekly"]),
        "run-replay-corpus" => Some(vec!["run", "replay-corpus"]),
        "run-security-review" => Some(vec!["run", "security-review"]),
        "storage-check" => Some(vec!["storage", "check"]),
        "storage-test" => Some(vec!["storage", "test"]),
        "storage-perf-gate" => Some(vec!["storage", "perf-gate"]),
        "storage-sweep" => Some(vec!["storage", "sweep"]),
        "storage-ci-smoke" => Some(vec!["storage", "ci-smoke"]),
        "ci-all" => Some(vec!["ci"]),
        "ci-rust-checks" => Some(vec!["ci", "--job", "rust-checks"]),
        "ci-integration" => Some(vec!["ci", "--job", "integration"]),
        "ci-runtime-determinism" => Some(vec!["ci", "--job", "runtime-determinism"]),
        "ci-runtime-security" => Some(vec!["ci", "--job", "runtime-security"]),
        "dev" => Some(vec!["dev"]),
        "install" => Some(vec!["install"]),
        "all" => Some(vec!["all"]),
        _ => None,
    }
}

async fn run_task_subprocess_capture(
    root: &Path,
    task: &str,
    cancel_notify: Arc<Notify>,
) -> Result<(i32, String)> {
    let args = task_to_cli_args(task).ok_or_else(|| anyhow!("unknown task: {task}"))?;
    let exe = std::env::current_exe().context("failed to resolve current executable")?;
    let mut cmd = Command::new(exe);
    cmd.arg("--root")
        .arg(root)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().context("failed to run task subprocess")?;

    let stdout_task = child.stdout.take().map(|mut stdout| {
        tokio::spawn(async move {
            let mut buf = Vec::new();
            let _ = stdout.read_to_end(&mut buf).await;
            buf
        })
    });
    let stderr_task = child.stderr.take().map(|mut stderr| {
        tokio::spawn(async move {
            let mut buf = Vec::new();
            let _ = stderr.read_to_end(&mut buf).await;
            buf
        })
    });

    let (status, canceled) = tokio::select! {
        waited = child.wait() => (
            waited.context("failed while waiting for task subprocess")?,
            false,
        ),
        _ = cancel_notify.notified() => {
            let _ = child.start_kill();
            (
                child
                    .wait()
                    .await
                    .context("failed while canceling task subprocess")?,
                true,
            )
        }
    };

    let stdout = match stdout_task {
        Some(task) => task.await.unwrap_or_default(),
        None => Vec::new(),
    };
    let stderr = match stderr_task {
        Some(task) => task.await.unwrap_or_default(),
        None => Vec::new(),
    };

    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&stdout));
    if !stdout.is_empty() && !stderr.is_empty() {
        text.push('\n');
    }
    text.push_str(&String::from_utf8_lossy(&stderr));
    let exit = if canceled {
        130
    } else {
        status.code().unwrap_or(1)
    };
    Ok((exit, text))
}

fn load_task_statuses(path: &Path) -> Result<HashMap<String, TaskStatus>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let bytes =
        std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let snapshot: TaskStatusSnapshot = serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    let mut out = HashMap::new();
    for mut task in snapshot.tasks {
        if matches!(task.state.as_str(), "running" | "queued" | "canceling") {
            task.state = "idle".to_string();
            task.finished_at_unix_s = Some(now_unix_s());
            task.last_exit = Some(1);
            task.last_output = truncate_output(
                format!(
                    "{}\n(recovered after process restart while active)",
                    task.last_output
                ),
                12_000,
            );
        }
        out.insert(task.task.clone(), task);
    }
    Ok(out)
}

fn persist_task_statuses(path: &Path, tasks: &HashMap<String, TaskStatus>) -> Result<()> {
    let mut entries: Vec<TaskStatus> = tasks.values().cloned().collect();
    entries.sort_by(|a, b| a.task.cmp(&b.task));
    let snapshot = TaskStatusSnapshot { tasks: entries };
    let data = serde_json::to_vec_pretty(&snapshot)?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, data).with_context(|| format!("failed to write {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| format!("failed to replace {}", path.display()))?;
    Ok(())
}

fn persist_run_log(
    runs_dir: &Path,
    task: &str,
    started_at_unix_s: Option<u64>,
    finished_at_unix_s: Option<u64>,
    state: &str,
    output: &str,
) -> Result<String> {
    let task_dir = runs_dir.join(sanitize_filename_component(task));
    std::fs::create_dir_all(&task_dir)
        .with_context(|| format!("failed to create {}", task_dir.display()))?;

    let started = started_at_unix_s.unwrap_or(now_unix_s());
    let finished = finished_at_unix_s.unwrap_or(now_unix_s());
    let filename = format!("{started}-{finished}-{state}.log");
    let log_path = task_dir.join(filename);
    let body = format!(
        "task={task}\nstate={state}\nstarted_at_unix_s={started}\nfinished_at_unix_s={finished}\n\n{output}\n"
    );
    std::fs::write(&log_path, body)
        .with_context(|| format!("failed to write {}", log_path.display()))?;

    Ok(log_path.display().to_string())
}

fn sanitize_filename_component(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn run_named_task_sync(root: &Path, task: &str) -> Result<()> {
    if run_native_task_sync(root, task)? {
        Ok(())
    } else {
        Err(anyhow!("unknown task: {task}"))
    }
}

async fn run_named_task_async(root: PathBuf, task: String) -> Result<String> {
    match task.as_str() {
        "test-integration" => {
            run_integration_scheduler_api(&root).await?;
            Ok("integration scheduler api completed".to_string())
        }
        "test-e2e" => {
            run_integration_e2e_lifecycle(&root).await?;
            Ok("integration e2e lifecycle completed".to_string())
        }
        "test-rotation" => {
            run_integration_policy_rotation(&root).await?;
            Ok("integration policy rotation completed".to_string())
        }
        "test-abi-rollover" => {
            run_integration_abi_rollover(&root).await?;
            Ok("integration abi rollover completed".to_string())
        }
        "run-fuzz-weekly" => {
            run_weekly_fuzz(&root, &load_app_config(&root)?).await?;
            Ok("weekly fuzz completed".to_string())
        }
        "run-replay-corpus" => {
            run_replay_corpus(&root, &load_app_config(&root)?).await?;
            Ok("replay corpus completed".to_string())
        }
        "run-security-review" => {
            validate_external_security_review(
                &root.join("crates/edgerun-runtime/SECURITY_FINDINGS.json"),
            )?;
            Ok("security review validation completed".to_string())
        }
        "all" => {
            run_named_task_sync_blocking(root.clone(), "doctor".to_string()).await?;
            run_named_task_sync_blocking(root.clone(), "setup".to_string()).await?;
            run_named_task_sync_blocking(root.clone(), "build-workspace".to_string()).await?;
            run_named_task_sync_blocking(root.clone(), "test-runtime".to_string()).await?;
            run_integration_scheduler_api(&root).await?;
            Ok("all workflow completed".to_string())
        }
        _ => {
            if !is_supported_task(&task) {
                return Err(anyhow!("unknown task: {task}"));
            }
            run_named_task_sync_blocking(root, task).await?;
            Ok("task completed".to_string())
        }
    }
}

async fn run_named_task_sync_blocking(root: PathBuf, task: String) -> Result<()> {
    tokio::task::spawn_blocking(move || run_named_task_sync(&root, &task))
        .await
        .context("task join failure")?
}

fn run_native_task_sync(root: &Path, task: &str) -> Result<bool> {
    match task {
        "doctor" => {
            run_doctor_sync(root)?;
            Ok(true)
        }
        "setup" => {
            run_setup_sync(root, false)?;
            Ok(true)
        }
        "setup-install" => {
            run_setup_sync(root, true)?;
            Ok(true)
        }
        "build-workspace" => {
            run_program_sync(
                "Build Workspace",
                "cargo",
                &["build", "--workspace"],
                root,
                false,
            )?;
            Ok(true)
        }
        "build-program" => {
            run_program_anchor_build_sync(root)?;
            Ok(true)
        }
        "test-workspace" => {
            run_program_sync(
                "Test Workspace",
                "cargo",
                &["test", "--workspace"],
                root,
                false,
            )?;
            Ok(true)
        }
        "test-runtime" => {
            run_program_sync(
                "Test Runtime",
                "cargo",
                &["test", "-p", "edgerun-runtime"],
                root,
                false,
            )?;
            Ok(true)
        }
        "test-program" => {
            run_program_local_tests_sync(root)?;
            Ok(true)
        }
        "dev" => {
            run_program_sync("Dev Check", "make", &["check"], root, false)?;
            Ok(true)
        }
        "install" => {
            run_program_sync(
                "Install edgerun binary",
                "cargo",
                &[
                    "install",
                    "--path",
                    "crates/edgerun-cli",
                    "--bin",
                    "edgerun",
                    "--force",
                ],
                root,
                false,
            )?;
            Ok(true)
        }
        "storage-check" => {
            run_storage_command(root, StorageCommand::Check)?;
            Ok(true)
        }
        "storage-test" => {
            run_storage_command(root, StorageCommand::Test)?;
            Ok(true)
        }
        "storage-perf-gate" => {
            run_storage_command(root, StorageCommand::PerfGate)?;
            Ok(true)
        }
        "storage-sweep" => {
            run_storage_command(
                root,
                StorageCommand::Sweep {
                    duration: None,
                    out_dir: None,
                    max_cases: None,
                },
            )?;
            Ok(true)
        }
        "storage-ci-smoke" => {
            run_storage_command(root, StorageCommand::CiSmoke)?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn run_doctor_sync(root: &Path) -> Result<()> {
    run_program_sync("Rust cargo", "cargo", &["--version"], root, false)?;
    run_program_sync("Rust compiler", "rustc", &["--version"], root, false)?;
    run_program_sync("Rustup", "rustup", &["--version"], root, false)?;
    run_program_sync("Bun", "bun", &["--version"], root, true)?;
    run_program_sync("Anchor CLI", "anchor", &["--version"], root, true)?;
    run_program_sync("Solana CLI", "solana", &["--version"], root, true)?;
    run_program_sync(
        "Solana test validator",
        "solana-test-validator",
        &["--version"],
        root,
        true,
    )?;
    run_program_sync("cargo-fuzz", "cargo-fuzz", &["--version"], root, true)?;
    run_program_sync("Python3", "python3", &["--version"], root, false)?;
    run_program_sync("curl", "curl", &["--version"], root, false)?;
    Ok(())
}

fn run_setup_sync(root: &Path, install_missing: bool) -> Result<()> {
    run_program_sync("Cargo fetch", "cargo", &["fetch", "--locked"], root, false)?;

    if command_exists("bun") {
        run_program_sync(
            "Program bun install",
            "bun",
            &["install", "--frozen-lockfile"],
            &root.join("program"),
            false,
        )?;
    } else {
        eprintln!("[warn] bun missing; skipping program dependency install");
    }

    if install_missing && !command_exists("cargo-fuzz") {
        run_program_sync(
            "Install cargo-fuzz",
            "cargo",
            &["install", "cargo-fuzz"],
            root,
            false,
        )?;
    }
    Ok(())
}

fn run_rust_checks_sync(root: &Path) -> Result<()> {
    run_program_sync(
        "cargo fmt --all --check",
        "cargo",
        &["fmt", "--all", "--check"],
        root,
        false,
    )?;
    run_program_sync(
        "cargo check --workspace",
        "cargo",
        &["check", "--workspace"],
        root,
        false,
    )?;
    run_program_sync(
        "cargo clippy --workspace --all-targets -- -D warnings",
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
        root,
        false,
    )?;
    run_program_sync(
        "cargo test --workspace",
        "cargo",
        &["test", "--workspace"],
        root,
        false,
    )
}

fn run_program_anchor_build_sync(root: &Path) -> Result<()> {
    let program_root = root.join("program");
    let env = program_tool_env(&program_root)?;
    run_program_sync_with_env(
        "Build Program",
        "anchor",
        &["build"],
        &program_root,
        false,
        &env,
    )
}

fn run_program_local_tests_sync(root: &Path) -> Result<()> {
    let program_root = root.join("program");
    let env = program_tool_env(&program_root)?;
    let program_id = "AgjxA2CoMmmWXrcsJtvvpmqdRHLVHrhYf6DAuBCL4s5T";
    let ledger_dir = program_root.join(".anchor/manual-test-ledger");
    let validator_log = ledger_dir.join("validator.log");
    std::fs::create_dir_all(&ledger_dir)?;

    run_program_anchor_build_sync(root)?;

    let log_file = std::fs::File::create(&validator_log)
        .with_context(|| format!("failed creating {}", validator_log.display()))?;
    let log_file_err = log_file.try_clone()?;
    let mut validator = std::process::Command::new("solana-test-validator")
        .arg("--reset")
        .arg("--ledger")
        .arg(&ledger_dir)
        .arg("--rpc-port")
        .arg("8899")
        .arg("--faucet-port")
        .arg("9900")
        .arg("--bpf-program")
        .arg(program_id)
        .arg(program_root.join("target/deploy/edgerun.so"))
        .current_dir(&program_root)
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(log_file_err))
        .spawn()
        .context("failed to start solana-test-validator")?;

    let test_result = (|| -> Result<()> {
        let mut ready = false;
        for _ in 0..60 {
            let status = std::process::Command::new("solana")
                .arg("cluster-version")
                .arg("--url")
                .arg("http://127.0.0.1:8899")
                .current_dir(&program_root)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            if matches!(status, Ok(s) if s.success()) {
                ready = true;
                break;
            }
            thread::sleep(Duration::from_millis(500));
        }
        ensure(
            ready,
            &format!(
                "validator did not become ready; check {}",
                validator_log.display()
            ),
        )?;

        let mut cmd = std::process::Command::new("bunx");
        cmd.arg("ts-mocha")
            .arg("-p")
            .arg("./tsconfig.json")
            .arg("-t")
            .arg("1000000")
            .arg("tests/**/*.ts")
            .current_dir(&program_root)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .env("ANCHOR_PROVIDER_URL", "http://127.0.0.1:8899")
            .env("ANCHOR_WALLET", program_root.join(".solana/id.json"))
            .env("ANCHOR_WS_URL", "ws://127.0.0.1:8900");
        for (k, v) in &env {
            cmd.env(k, v);
        }
        let status = cmd.status().context("failed to run bunx ts-mocha")?;
        ensure(status.success(), "program local tests failed")
    })();

    let _ = validator.kill();
    let _ = validator.wait();
    test_result
}

fn program_tool_env(program_root: &Path) -> Result<Vec<(OsString, OsString)>> {
    let cargo_home = program_root.join(".cargo-home");
    let cargo_install_root = program_root.join(".cargo");
    let cargo_target_dir = program_root.join("target");
    let cargo_bin_dir = cargo_install_root.join("bin");

    let mut paths = vec![cargo_bin_dir];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    let joined_path = std::env::join_paths(paths).context("failed to join PATH")?;

    Ok(vec![
        (OsString::from("CARGO_HOME"), cargo_home.into_os_string()),
        (
            OsString::from("CARGO_INSTALL_ROOT"),
            cargo_install_root.into_os_string(),
        ),
        (
            OsString::from("CARGO_TARGET_DIR"),
            cargo_target_dir.into_os_string(),
        ),
        (OsString::from("PATH"), joined_path),
    ])
}

fn run_program_sync(
    label: &str,
    program: &str,
    args: &[&str],
    cwd: &Path,
    allow_missing: bool,
) -> Result<()> {
    run_program_sync_with_env(label, program, args, cwd, allow_missing, &[])
}

fn run_program_sync_with_env(
    label: &str,
    program: &str,
    args: &[&str],
    cwd: &Path,
    allow_missing: bool,
    envs: &[(OsString, OsString)],
) -> Result<()> {
    let display = if args.is_empty() {
        program.to_string()
    } else {
        format!("{program} {}", args.join(" "))
    };
    println!("==> {label}");
    println!("$ {display}");

    let mut command = std::process::Command::new(program);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    for (k, v) in envs {
        command.env(k, v);
    }
    let status = command.status();

    match status {
        Ok(status) => {
            if status.success() {
                Ok(())
            } else {
                Err(anyhow!(
                    "step '{}' failed with exit status {:?}",
                    label,
                    status.code()
                ))
            }
        }
        Err(err) if allow_missing && err.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("[warn] optional command missing: {program}");
            Ok(())
        }
        Err(err) => Err(err).with_context(|| format!("failed to launch: {display}")),
    }
}

fn run_program_sync_owned(
    label: &str,
    program: &str,
    args: &[String],
    cwd: &Path,
    allow_missing: bool,
) -> Result<()> {
    let borrowed: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run_program_sync_with_env(label, program, &borrowed, cwd, allow_missing, &[])
}

#[derive(Debug, Deserialize)]
struct SimpleOkResponse {
    ok: bool,
    #[serde(default)]
    duplicate: bool,
}

#[derive(Debug, Deserialize)]
struct JobCreateResponse {
    job_id: String,
    bundle_hash: String,
}

#[derive(Debug, Deserialize)]
struct PolicyInfoResponse {
    signer_pubkey: String,
}

async fn post_json_ok(
    client: &reqwest::Client,
    base: &str,
    path: &str,
    body: &Value,
) -> Result<SimpleOkResponse> {
    client
        .post(format!("{base}{path}"))
        .json(body)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
        .map_err(Into::into)
}

fn run_program_capture_sync_owned(
    label: &str,
    program: &str,
    args: &[String],
    cwd: &Path,
    envs: &[(OsString, OsString)],
) -> Result<String> {
    let display = if args.is_empty() {
        program.to_string()
    } else {
        format!("{program} {}", args.join(" "))
    };
    println!("==> {label}");
    println!("$ {display}");

    let mut command = std::process::Command::new(program);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in envs {
        command.env(k, v);
    }
    let output = command
        .output()
        .with_context(|| format!("failed to launch: {display}"))?;

    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    if !output.stdout.is_empty() && !output.stderr.is_empty() {
        text.push('\n');
    }
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    print!("{text}");

    if output.status.success() {
        Ok(text)
    } else {
        Err(anyhow!(
            "step '{}' failed with exit status {:?}",
            label,
            output.status.code()
        ))
    }
}

#[derive(Clone, Copy)]
struct StorageSweepGate {
    min_top_score: f64,
    min_top_writes_ops: u64,
    min_top_reads_ops: u64,
    max_top_comp_failed: u64,
}

struct StorageSweepOptions {
    duration: u64,
    out_dir: PathBuf,
    max_cases: usize,
    gate: StorageSweepGate,
}

struct StorageSweepRecord {
    case_id: usize,
    writers: u64,
    readers: u64,
    write_batch: u64,
    read_batch: u64,
    key_space: u64,
    hot_key_space: u64,
    writes_ops: u64,
    reads_ops: u64,
    hit_rate_pct: f64,
    comp_sched: u64,
    comp_done: u64,
    comp_failed: u64,
    comp_skipped: u64,
    comp_total_ms: u64,
    score: f64,
    log_path: PathBuf,
}

fn run_storage_command(root: &Path, command: StorageCommand) -> Result<()> {
    let storage_root = root.join("crates/edgerun-storage");
    ensure(
        storage_root.exists(),
        &format!("missing storage crate: {}", storage_root.display()),
    )?;
    match command {
        StorageCommand::Check => run_program_sync_with_env(
            "Storage check",
            "cargo",
            &["check", "--all-targets"],
            &storage_root,
            false,
            &[(OsString::from("RUSTFLAGS"), OsString::from("-D warnings"))],
        ),
        StorageCommand::Test => run_program_sync(
            "Storage test",
            "cargo",
            &["test", "-q"],
            &storage_root,
            false,
        ),
        StorageCommand::PerfGate => run_storage_perf_gate(&storage_root),
        StorageCommand::Sweep {
            duration,
            out_dir,
            max_cases,
        } => {
            let opts = StorageSweepOptions {
                duration: duration.unwrap_or(8),
                out_dir: out_dir.unwrap_or_else(default_storage_sweep_out_dir),
                max_cases: max_cases.unwrap_or(0),
                gate: load_storage_sweep_gate_thresholds(),
            };
            run_storage_mixed_rw_tuning_sweep(&storage_root, opts).map(|_| ())
        }
        StorageCommand::Crash { args } => {
            let mut full = vec![
                "run".to_string(),
                "-q".to_string(),
                "--bin".to_string(),
                "crash_campaign".to_string(),
                "--".to_string(),
            ];
            full.extend(args);
            run_program_sync_owned(
                "Storage crash campaign",
                "cargo",
                &full,
                &storage_root,
                false,
            )
        }
        StorageCommand::Bench { args } => {
            let mut full = vec!["bench".to_string()];
            full.extend(args);
            run_program_sync_owned("Storage bench", "cargo", &full, &storage_root, false)
        }
        StorageCommand::RepBench { args } => {
            let mut full = vec![
                "run".to_string(),
                "-q".to_string(),
                "--bin".to_string(),
                "replication_group_commit_benchmark".to_string(),
                "--".to_string(),
            ];
            full.extend(args);
            run_program_sync_owned(
                "Storage replication benchmark",
                "cargo",
                &full,
                &storage_root,
                false,
            )
        }
        StorageCommand::EncDemo { args } => {
            let mut full = vec![
                "run".to_string(),
                "-q".to_string(),
                "--bin".to_string(),
                "encrypted_append_demo".to_string(),
                "--".to_string(),
            ];
            full.extend(args);
            run_program_sync_owned(
                "Storage encrypted append demo",
                "cargo",
                &full,
                &storage_root,
                false,
            )
        }
        StorageCommand::CiSmoke => run_storage_ci_smoke(&storage_root),
    }
}

fn run_storage_ci_smoke(storage_root: &Path) -> Result<()> {
    run_program_sync(
        "Storage fmt check",
        "cargo",
        &["fmt", "--check"],
        storage_root,
        false,
    )?;
    run_program_sync_with_env(
        "Storage check",
        "cargo",
        &["check", "--all-targets"],
        storage_root,
        false,
        &[(OsString::from("RUSTFLAGS"), OsString::from("-D warnings"))],
    )?;
    run_program_sync(
        "Storage test",
        "cargo",
        &["test", "-q"],
        storage_root,
        false,
    )?;
    let opts = StorageSweepOptions {
        duration: 1,
        out_dir: default_storage_sweep_out_dir(),
        max_cases: 1,
        gate: StorageSweepGate {
            min_top_score: 1.0,
            min_top_writes_ops: 1,
            min_top_reads_ops: 1,
            max_top_comp_failed: 999,
        },
    };
    run_storage_perf_gate_with_options(storage_root, opts)
}

fn run_storage_perf_gate(storage_root: &Path) -> Result<()> {
    let min_end_to_end_p1 = env_f64("MIN_END_TO_END_P1_MBPS", 120.0);
    let min_io_only_p1 = env_f64("MIN_IO_ONLY_P1_MBPS", 1800.0);
    let min_end_to_end_p8 = env_f64("MIN_END_TO_END_P8_MBPS", 450.0);
    let sweep_duration = env_u64("MIXED_RW_SWEEP_DURATION", 4);
    let sweep_max_cases = env_usize("MIXED_RW_SWEEP_MAX_CASES", 4);
    let opts = StorageSweepOptions {
        duration: sweep_duration,
        out_dir: default_storage_sweep_out_dir(),
        max_cases: sweep_max_cases,
        gate: load_storage_sweep_gate_thresholds(),
    };
    run_storage_perf_gate_with_options_and_thresholds(
        storage_root,
        opts,
        min_end_to_end_p1,
        min_io_only_p1,
        min_end_to_end_p8,
    )
}

fn run_storage_perf_gate_with_options(
    storage_root: &Path,
    opts: StorageSweepOptions,
) -> Result<()> {
    run_storage_perf_gate_with_options_and_thresholds(storage_root, opts, 120.0, 1800.0, 450.0)
}

fn run_storage_perf_gate_with_options_and_thresholds(
    storage_root: &Path,
    opts: StorageSweepOptions,
    min_end_to_end_p1: f64,
    min_io_only_p1: f64,
    min_end_to_end_p8: f64,
) -> Result<()> {
    println!("Running Phase A perf gate...");
    let out_p1 = run_program_capture_sync_owned(
        "Storage async writer benchmark (both, producers=1)",
        "cargo",
        &[
            "run".to_string(),
            "-q".to_string(),
            "--bin".to_string(),
            "async_writer_benchmark".to_string(),
            "--".to_string(),
            "--mode".to_string(),
            "both".to_string(),
            "--producers".to_string(),
            "1".to_string(),
        ],
        storage_root,
        &[],
    )?;
    let e2e_p1 = extract_mode_throughput_mbps(&out_p1, "end_to_end")
        .ok_or_else(|| anyhow!("unable to parse end_to_end throughput for producers=1"))?;
    let io_p1 = extract_mode_throughput_mbps(&out_p1, "io_only")
        .ok_or_else(|| anyhow!("unable to parse io_only throughput for producers=1"))?;

    let out_p8 = run_program_capture_sync_owned(
        "Storage async writer benchmark (end_to_end, producers=8)",
        "cargo",
        &[
            "run".to_string(),
            "-q".to_string(),
            "--bin".to_string(),
            "async_writer_benchmark".to_string(),
            "--".to_string(),
            "--mode".to_string(),
            "end_to_end".to_string(),
            "--producers".to_string(),
            "8".to_string(),
        ],
        storage_root,
        &[],
    )?;
    let e2e_p8 = extract_mode_throughput_mbps(&out_p8, "end_to_end")
        .ok_or_else(|| anyhow!("unable to parse end_to_end throughput for producers=8"))?;

    assert_mbps_ge(e2e_p1, min_end_to_end_p1, "end_to_end producers=1")?;
    assert_mbps_ge(io_p1, min_io_only_p1, "io_only producers=1")?;
    assert_mbps_ge(e2e_p8, min_end_to_end_p8, "end_to_end producers=8")?;
    println!("Phase A perf gate passed.\n");
    println!("Running mixed RW tuning sweep gate...");
    let result = run_storage_mixed_rw_tuning_sweep(storage_root, opts)?;
    println!("Mixed RW tuning sweep gate passed.");
    println!("CSV: {}", result.csv.display());
    println!("Summary: {}", result.summary.display());
    Ok(())
}

fn extract_mode_throughput_mbps(text: &str, mode: &str) -> Option<f64> {
    let mut in_mode = false;
    for line in text.lines() {
        if line.starts_with("--- Mode: ") {
            in_mode = line.contains(mode);
            continue;
        }
        if !in_mode {
            continue;
        }
        if line.contains("Throughput:") && line.contains("MB/s") {
            for token in line.split_whitespace() {
                if let Ok(v) = token.replace("MB/s", "").parse::<f64>() {
                    return Some(v);
                }
            }
        }
    }
    None
}

fn assert_mbps_ge(value: f64, min: f64, label: &str) -> Result<()> {
    if value < min {
        return Err(anyhow!("FAIL: {label} {value:.2} MB/s < {min:.2} MB/s"));
    }
    println!("PASS: {label} {value:.2} MB/s >= {min:.2} MB/s");
    Ok(())
}

struct StorageSweepResult {
    csv: PathBuf,
    summary: PathBuf,
}

fn run_storage_mixed_rw_tuning_sweep(
    storage_root: &Path,
    opts: StorageSweepOptions,
) -> Result<StorageSweepResult> {
    let logs_dir = opts.out_dir.join("logs");
    std::fs::create_dir_all(&logs_dir)
        .with_context(|| format!("failed to create {}", logs_dir.display()))?;
    let csv = opts.out_dir.join("results.csv");
    let summary = opts.out_dir.join("summary.md");
    std::fs::write(
        &csv,
        "case_id,writers,readers,write_batch,read_batch,key_space,hot_key_space,writes_ops,reads_ops,hit_rate_pct,comp_sched,comp_done,comp_failed,comp_skipped,comp_total_ms,score,log\n",
    )?;

    let cases: &[(u64, u64, u64, u64, u64, u64)] = &[
        (2, 4, 512, 2048, 2_000_000, 200_000),
        (2, 4, 1024, 2048, 2_000_000, 200_000),
        (2, 6, 512, 4096, 2_500_000, 250_000),
        (3, 6, 512, 4096, 2_500_000, 250_000),
        (3, 8, 512, 4096, 3_000_000, 300_000),
        (4, 8, 512, 4096, 3_000_000, 300_000),
        (4, 8, 1024, 4096, 3_000_000, 300_000),
        (4, 10, 1024, 4096, 3_500_000, 350_000),
    ];

    let mut records = Vec::new();
    for (idx, (writers, readers, write_batch, read_batch, key_space, hot_key_space)) in
        cases.iter().copied().enumerate()
    {
        if opts.max_cases > 0 && records.len() >= opts.max_cases {
            break;
        }
        let case_id = idx + 1;
        let log_path = logs_dir.join(format!("case_{case_id}.log"));
        println!(
            "[case {case_id}] writers={writers} readers={readers} write_batch={write_batch} read_batch={read_batch}"
        );
        let output = run_program_capture_sync_owned(
            &format!("Storage mixed RW benchmark case {case_id}"),
            "cargo",
            &[
                "run".to_string(),
                "-q".to_string(),
                "--bin".to_string(),
                "mixed_rw_compaction_benchmark".to_string(),
                "--".to_string(),
                "--duration".to_string(),
                opts.duration.to_string(),
                "--writers".to_string(),
                writers.to_string(),
                "--readers".to_string(),
                readers.to_string(),
                "--write-batch".to_string(),
                write_batch.to_string(),
                "--read-batch".to_string(),
                read_batch.to_string(),
                "--key-space".to_string(),
                key_space.to_string(),
                "--hot-key-space".to_string(),
                hot_key_space.to_string(),
            ],
            storage_root,
            &[],
        )?;
        std::fs::write(&log_path, &output)
            .with_context(|| format!("failed to write {}", log_path.display()))?;

        let writes_line = output
            .lines()
            .rev()
            .find(|l| l.starts_with("writes:"))
            .ok_or_else(|| anyhow!("missing writes line in case {case_id}"))?;
        let reads_line = output
            .lines()
            .rev()
            .find(|l| l.starts_with("reads:"))
            .ok_or_else(|| anyhow!("missing reads line in case {case_id}"))?;
        let comp_line = output
            .lines()
            .rev()
            .find(|l| l.starts_with("compaction:"))
            .ok_or_else(|| anyhow!("missing compaction line in case {case_id}"))?;

        let writes_ops = parse_ops_per_second(writes_line)
            .ok_or_else(|| anyhow!("missing writes ops/s in case {case_id}"))?;
        let reads_ops = parse_ops_per_second(reads_line)
            .ok_or_else(|| anyhow!("missing reads ops/s in case {case_id}"))?;
        let hit_rate_pct = parse_hit_rate_pct(reads_line)
            .ok_or_else(|| anyhow!("missing hit_rate in case {case_id}"))?;
        let comp_sched = parse_line_u64(comp_line, "scheduled")
            .ok_or_else(|| anyhow!("missing compaction scheduled in case {case_id}"))?;
        let comp_done = parse_line_u64(comp_line, "completed")
            .ok_or_else(|| anyhow!("missing compaction completed in case {case_id}"))?;
        let comp_failed = parse_line_u64(comp_line, "failed")
            .ok_or_else(|| anyhow!("missing compaction failed in case {case_id}"))?;
        let comp_skipped = parse_line_u64(comp_line, "skipped")
            .ok_or_else(|| anyhow!("missing compaction skipped in case {case_id}"))?;
        let comp_total_ms = parse_line_u64(comp_line, "total_ms")
            .ok_or_else(|| anyhow!("missing compaction total_ms in case {case_id}"))?;
        let score =
            writes_ops as f64 + (reads_ops as f64 * 4.0) - (comp_failed as f64 * 1_000_000.0);

        let record = StorageSweepRecord {
            case_id,
            writers,
            readers,
            write_batch,
            read_batch,
            key_space,
            hot_key_space,
            writes_ops,
            reads_ops,
            hit_rate_pct,
            comp_sched,
            comp_done,
            comp_failed,
            comp_skipped,
            comp_total_ms,
            score,
            log_path: log_path.clone(),
        };
        append_storage_csv(&csv, &record)?;
        records.push(record);
    }

    ensure(!records.is_empty(), "no sweep results found")?;
    let mut ranked = records;
    ranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let top = ranked.first().expect("ranked is non-empty");

    let mut gate_failed = false;
    if top.score < opts.gate.min_top_score {
        eprintln!(
            "FAIL: top score {:.2} < min {:.2}",
            top.score, opts.gate.min_top_score
        );
        gate_failed = true;
    }
    if top.writes_ops < opts.gate.min_top_writes_ops {
        eprintln!(
            "FAIL: top writes/s {} < min {}",
            top.writes_ops, opts.gate.min_top_writes_ops
        );
        gate_failed = true;
    }
    if top.reads_ops < opts.gate.min_top_reads_ops {
        eprintln!(
            "FAIL: top reads/s {} < min {}",
            top.reads_ops, opts.gate.min_top_reads_ops
        );
        gate_failed = true;
    }
    if top.comp_failed > opts.gate.max_top_comp_failed {
        eprintln!(
            "FAIL: top comp_failed {} > max {}",
            top.comp_failed, opts.gate.max_top_comp_failed
        );
        gate_failed = true;
    }

    write_storage_sweep_summary(&summary, opts.duration, &csv, &ranked, opts.gate)?;
    println!("Sweep complete.");
    println!("CSV: {}", csv.display());
    println!("Summary: {}", summary.display());
    if gate_failed {
        return Err(anyhow!("mixed RW tuning sweep gate failed"));
    }
    println!("Mixed RW tuning sweep gate passed.");
    Ok(StorageSweepResult { csv, summary })
}

fn append_storage_csv(csv_path: &Path, record: &StorageSweepRecord) -> Result<()> {
    let row = format!(
        "{},{},{},{},{},{},{},{},{},{:.2},{},{},{},{},{},{:.2},{}\n",
        record.case_id,
        record.writers,
        record.readers,
        record.write_batch,
        record.read_batch,
        record.key_space,
        record.hot_key_space,
        record.writes_ops,
        record.reads_ops,
        record.hit_rate_pct,
        record.comp_sched,
        record.comp_done,
        record.comp_failed,
        record.comp_skipped,
        record.comp_total_ms,
        record.score,
        record.log_path.display()
    );
    let mut existing = std::fs::read_to_string(csv_path)
        .with_context(|| format!("failed to read {}", csv_path.display()))?;
    existing.push_str(&row);
    std::fs::write(csv_path, existing)
        .with_context(|| format!("failed to write {}", csv_path.display()))?;
    Ok(())
}

fn write_storage_sweep_summary(
    summary_path: &Path,
    duration: u64,
    csv: &Path,
    ranked: &[StorageSweepRecord],
    gate: StorageSweepGate,
) -> Result<()> {
    let top = ranked.first().expect("ranked non-empty");
    let mut text = String::new();
    text.push_str("# Mixed RW Tuning Sweep\n\n");
    text.push_str(&format!("- Duration per case: {duration}s\n"));
    text.push_str(&format!("- Cases run: {}\n", ranked.len()));
    text.push_str(&format!("- CSV: `{}`\n", csv.display()));
    text.push_str("- Gate thresholds:\n");
    text.push_str(&format!("  - min_top_score: {:.2}\n", gate.min_top_score));
    text.push_str(&format!(
        "  - min_top_writes_ops: {}\n",
        gate.min_top_writes_ops
    ));
    text.push_str(&format!(
        "  - min_top_reads_ops: {}\n",
        gate.min_top_reads_ops
    ));
    text.push_str(&format!(
        "  - max_top_comp_failed: {}\n",
        gate.max_top_comp_failed
    ));
    text.push_str(&format!(
        "- Top case: {} (score={:.2}, writes/s={}, reads/s={}, comp_failed={})\n\n",
        top.case_id, top.score, top.writes_ops, top.reads_ops, top.comp_failed
    ));
    text.push_str("## Ranked Results\n\n");
    text.push_str(
        "| Rank | Case | W | R | WB | RB | writes/s | reads/s | hit% | comp_failed | score |\n",
    );
    text.push_str("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for (rank, rec) in ranked.iter().enumerate() {
        text.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {:.2} | {} | {:.2} |\n",
            rank + 1,
            rec.case_id,
            rec.writers,
            rec.readers,
            rec.write_batch,
            rec.read_batch,
            rec.writes_ops,
            rec.reads_ops,
            rec.hit_rate_pct,
            rec.comp_failed,
            rec.score
        ));
    }
    std::fs::write(summary_path, text)
        .with_context(|| format!("failed to write {}", summary_path.display()))
}

fn parse_ops_per_second(line: &str) -> Option<u64> {
    let start = line.find('(')?;
    let end = line.find(" ops/s")?;
    line[start + 1..end].trim().parse::<u64>().ok()
}

fn parse_hit_rate_pct(line: &str) -> Option<f64> {
    let marker = "hit_rate=";
    let idx = line.find(marker)?;
    let rest = &line[idx + marker.len()..];
    let end = rest.find('%')?;
    rest[..end].trim().parse::<f64>().ok()
}

fn parse_line_u64(line: &str, key: &str) -> Option<u64> {
    let marker = format!("{key}=");
    let idx = line.find(&marker)?;
    let rest = &line[idx + marker.len()..];
    let token = rest.split_whitespace().next()?;
    token.parse::<u64>().ok()
}

fn default_storage_sweep_out_dir() -> PathBuf {
    std::env::temp_dir().join(format!("mixed_rw_tuning_sweep_{}", now_unix_s()))
}

fn load_storage_sweep_gate_thresholds() -> StorageSweepGate {
    StorageSweepGate {
        min_top_score: env_f64("MIN_TOP_SCORE", 700_000.0),
        min_top_writes_ops: env_u64("MIN_TOP_WRITES_OPS", 250_000),
        min_top_reads_ops: env_u64("MIN_TOP_READS_OPS", 80_000),
        max_top_comp_failed: env_u64("MAX_TOP_COMP_FAILED", 0),
    }
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(default)
}

fn env_f64(name: &str, default: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(default)
}

async fn run_integration_scheduler_api(root: &Path) -> Result<()> {
    let cfg = load_app_config(root)?;
    let (require_sig, require_attest, quorum_attest) = integration_flag_env(&cfg);
    let tmp_dir = create_temp_dir("edgerun-int-scheduler-api")?;
    let sched_log = tmp_dir.join("scheduler.log");
    let sched_data = tmp_dir.join("scheduler-data");
    std::fs::create_dir_all(&sched_data)?;

    let sched_port = pick_free_port()?;
    let sched_addr = format!("127.0.0.1:{sched_port}");
    let sched_url = format!("http://{sched_addr}");

    let mut scheduler = spawn_scheduler(
        root,
        &sched_log,
        &[
            (
                "EDGERUN_SCHEDULER_DATA_DIR",
                sched_data.display().to_string(),
            ),
            ("EDGERUN_SCHEDULER_ADDR", sched_addr.clone()),
            ("EDGERUN_SCHEDULER_MAX_REPORTS_PER_JOB", "2".to_string()),
            ("EDGERUN_SCHEDULER_MAX_FAILURES_PER_JOB", "2".to_string()),
            ("EDGERUN_SCHEDULER_MAX_REPLAYS_PER_JOB", "2".to_string()),
            (
                "EDGERUN_SCHEDULER_REQUIRE_WORKER_SIGNATURES",
                require_sig.clone(),
            ),
            (
                "EDGERUN_SCHEDULER_REQUIRE_RESULT_ATTESTATION",
                require_attest.clone(),
            ),
            (
                "EDGERUN_SCHEDULER_QUORUM_REQUIRES_ATTESTATION",
                quorum_attest.clone(),
            ),
        ],
    )
    .await?;

    let client = reqwest::Client::new();
    wait_for_health(&client, &sched_url, &mut scheduler).await?;

    let runtime_id = "1111111111111111111111111111111111111111111111111111111111111111";
    let create_body = json!({
        "runtime_id": runtime_id,
        "wasm_base64":"AA==",
        "input_base64":"",
        "limits":{"max_memory_bytes":1048576,"max_instructions":10000},
        "escrow_lamports":1,
        "assignment_worker_pubkey":"worker-a"
    });
    let create: JobCreateResponse = client
        .post(format!("{sched_url}/v1/job/create"))
        .json(&create_body)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let job_id = create.job_id;
    let bundle_hash = create.bundle_hash;

    let output_hash_1 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let output_hash_2 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let output_hash_3 = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    let r1 = json!({
        "idempotency_key":"k-result-1","worker_pubkey":"worker-a","job_id":job_id,"bundle_hash":bundle_hash,"output_hash":output_hash_1,"output_len":10
    });
    let resp = post_json_ok(&client, &sched_url, "/v1/worker/result", &r1).await?;
    ensure(
        resp.ok && !resp.duplicate,
        "result first submit should not duplicate",
    )?;
    let resp = post_json_ok(&client, &sched_url, "/v1/worker/result", &r1).await?;
    ensure(
        resp.ok && resp.duplicate,
        "result second submit should duplicate",
    )?;

    let f1 = json!({
        "idempotency_key":"k-failure-1","worker_pubkey":"worker-a","job_id":job_id,"bundle_hash":bundle_hash,"phase":"runtime_execute","error_code":"InstructionLimitExceeded","error_message":"out of fuel"
    });
    let resp = post_json_ok(&client, &sched_url, "/v1/worker/failure", &f1).await?;
    ensure(
        resp.ok && !resp.duplicate,
        "failure first submit should not duplicate",
    )?;
    let resp = post_json_ok(&client, &sched_url, "/v1/worker/failure", &f1).await?;
    ensure(
        resp.ok && resp.duplicate,
        "failure second submit should duplicate",
    )?;

    let p1 = json!({
        "idempotency_key":"k-replay-1","worker_pubkey":"worker-a","job_id":job_id,"artifact":{"bundle_hash":bundle_hash,"ok":false,"abi_version":1,"runtime_id":runtime_id,"output_hash":null,"output_len":null,"input_len":3,"max_memory_bytes":1024,"max_instructions":1000,"fuel_limit":1000,"fuel_remaining":0,"error_code":"InstructionLimitExceeded","error_message":"out of fuel","trap_code":"OutOfFuel"}
    });
    let resp = post_json_ok(&client, &sched_url, "/v1/worker/replay", &p1).await?;
    ensure(
        resp.ok && !resp.duplicate,
        "replay first submit should not duplicate",
    )?;
    let resp = post_json_ok(&client, &sched_url, "/v1/worker/replay", &p1).await?;
    ensure(
        resp.ok && resp.duplicate,
        "replay second submit should duplicate",
    )?;

    post_json_ok(&client, &sched_url, "/v1/worker/result", &json!({"idempotency_key":"k-result-2","worker_pubkey":"worker-a","job_id":job_id,"bundle_hash":bundle_hash,"output_hash":output_hash_2,"output_len":20})).await?;
    post_json_ok(&client, &sched_url, "/v1/worker/result", &json!({"idempotency_key":"k-result-3","worker_pubkey":"worker-a","job_id":job_id,"bundle_hash":bundle_hash,"output_hash":output_hash_3,"output_len":30})).await?;
    post_json_ok(&client, &sched_url, "/v1/worker/failure", &json!({"idempotency_key":"k-failure-2","worker_pubkey":"worker-a","job_id":job_id,"bundle_hash":bundle_hash,"phase":"post_execution_verify","error_code":"BundleHashMismatch","error_message":"mismatch"})).await?;
    post_json_ok(&client, &sched_url, "/v1/worker/failure", &json!({"idempotency_key":"k-failure-3","worker_pubkey":"worker-a","job_id":job_id,"bundle_hash":bundle_hash,"phase":"runtime_execute","error_code":"Trap","error_message":"trap"})).await?;
    post_json_ok(&client, &sched_url, "/v1/worker/replay", &json!({"idempotency_key":"k-replay-2","worker_pubkey":"worker-a","job_id":job_id,"artifact":{"bundle_hash":bundle_hash,"ok":true,"abi_version":1,"runtime_id":runtime_id,"output_hash":output_hash_2,"output_len":20,"input_len":3,"max_memory_bytes":1024,"max_instructions":1000,"fuel_limit":1000,"fuel_remaining":900,"error_code":null,"error_message":null,"trap_code":null}})).await?;
    post_json_ok(&client, &sched_url, "/v1/worker/replay", &json!({"idempotency_key":"k-replay-3","worker_pubkey":"worker-a","job_id":job_id,"artifact":{"bundle_hash":bundle_hash,"ok":true,"abi_version":1,"runtime_id":runtime_id,"output_hash":output_hash_3,"output_len":30,"input_len":3,"max_memory_bytes":1024,"max_instructions":1000,"fuel_limit":1000,"fuel_remaining":800,"error_code":null,"error_message":null,"trap_code":null}})).await?;

    let status: Value = client
        .get(format!("{sched_url}/v1/job/{job_id}"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let reports = status["reports"].as_array().cloned().unwrap_or_default();
    let failures = status["failures"].as_array().cloned().unwrap_or_default();
    let replays = status["replay_artifacts"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    ensure(reports.len() == 2, "expected 2 reports")?;
    ensure(failures.len() == 2, "expected 2 failures")?;
    ensure(replays.len() == 2, "expected 2 replay artifacts")?;
    ensure(
        reports
            .last()
            .and_then(|x| x["output_hash"].as_str())
            .unwrap_or_default()
            == output_hash_3,
        "expected newest result output_hash=o3",
    )?;
    ensure(
        failures
            .last()
            .and_then(|x| x["idempotency_key"].as_str())
            .unwrap_or_default()
            == "k-failure-3",
        "expected newest failure idempotency key",
    )?;
    ensure(
        replays
            .last()
            .and_then(|x| x["idempotency_key"].as_str())
            .unwrap_or_default()
            == "k-replay-3",
        "expected newest replay idempotency key",
    )?;

    kill_child(&mut scheduler).await;
    let _ = std::fs::remove_dir_all(tmp_dir);
    Ok(())
}

async fn run_integration_e2e_lifecycle(root: &Path) -> Result<()> {
    let cfg = load_app_config(root)?;
    let (require_sig, require_attest, quorum_attest) = integration_flag_env(&cfg);
    let tmp_dir = create_temp_dir("edgerun-int-e2e")?;
    let sched_log = tmp_dir.join("scheduler.log");
    let worker_log = tmp_dir.join("worker.log");
    let sched_data = tmp_dir.join("scheduler-data");
    std::fs::create_dir_all(&sched_data)?;

    let sched_port = pick_free_port()?;
    let sched_addr = format!("127.0.0.1:{sched_port}");
    let sched_url = format!("http://{sched_addr}");
    let worker_pubkey = "worker-e2e-1";

    let mut scheduler = spawn_scheduler(
        root,
        &sched_log,
        &[
            (
                "EDGERUN_SCHEDULER_DATA_DIR",
                sched_data.display().to_string(),
            ),
            ("EDGERUN_SCHEDULER_ADDR", sched_addr.clone()),
            (
                "EDGERUN_SCHEDULER_REQUIRE_WORKER_SIGNATURES",
                require_sig.clone(),
            ),
            (
                "EDGERUN_SCHEDULER_REQUIRE_RESULT_ATTESTATION",
                require_attest.clone(),
            ),
            (
                "EDGERUN_SCHEDULER_QUORUM_REQUIRES_ATTESTATION",
                quorum_attest.clone(),
            ),
        ],
    )
    .await?;
    let client = reqwest::Client::new();
    wait_for_health(&client, &sched_url, &mut scheduler).await?;

    let mut worker = spawn_worker(
        root,
        &worker_log,
        &[
            ("EDGERUN_WORKER_PUBKEY", worker_pubkey.to_string()),
            ("EDGERUN_SCHEDULER_URL", sched_url.clone()),
        ],
    )
    .await?;

    let create_body = json!({
        "runtime_id":"0000000000000000000000000000000000000000000000000000000000000000",
        "wasm_base64":"AA==",
        "input_base64":"",
        "limits":{"max_memory_bytes":1048576,"max_instructions":10000},
        "escrow_lamports":1,
        "assignment_worker_pubkey":worker_pubkey
    });
    let create: JobCreateResponse = client
        .post(format!("{sched_url}/v1/job/create"))
        .json(&create_body)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let job_id = create.job_id;

    let mut success = false;
    for _ in 0..240 {
        if worker.try_wait()?.is_some() {
            break;
        }
        if scheduler.try_wait()?.is_some() {
            break;
        }
        let status: Value = client
            .get(format!("{sched_url}/v1/job/{job_id}"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let has_fail = status["failures"]
            .as_array()
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        let has_replay = status["replay_artifacts"]
            .as_array()
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        if has_fail && has_replay {
            let artifact_ok = status["replay_artifacts"]
                .as_array()
                .and_then(|arr| arr.last())
                .and_then(|last| last["artifact"]["ok"].as_bool())
                .unwrap_or(true);
            ensure(!artifact_ok, "expected replay artifact ok=false")?;
            success = true;
            break;
        }
        sleep(Duration::from_millis(500)).await;
    }

    kill_child(&mut worker).await;
    kill_child(&mut scheduler).await;
    let _ = std::fs::remove_dir_all(tmp_dir);
    ensure(success, "timed out waiting for e2e failure+replay")
}

async fn run_integration_policy_rotation(root: &Path) -> Result<()> {
    let cfg = load_app_config(root)?;
    let (require_sig, require_attest, quorum_attest) = integration_flag_env(&cfg);
    let tmp_dir = create_temp_dir("edgerun-int-policy-rotation")?;
    let sched_log = tmp_dir.join("scheduler.log");
    let worker_a_log = tmp_dir.join("worker-a.log");
    let worker_b_log = tmp_dir.join("worker-b.log");
    let sched_data = tmp_dir.join("scheduler-data");
    std::fs::create_dir_all(&sched_data)?;

    let key2_hex = "0202020202020202020202020202020202020202020202020202020202020202";
    let key2_id = "rot-key-2";
    let key2_ver = "2";
    let worker_a = "worker-rot-a";
    let worker_b = "worker-rot-b";

    let sched_port = pick_free_port()?;
    let sched_addr = format!("127.0.0.1:{sched_port}");
    let sched_url = format!("http://{sched_addr}");
    let client = reqwest::Client::new();

    let mut scheduler = spawn_scheduler(
        root,
        &sched_log,
        &[
            (
                "EDGERUN_SCHEDULER_DATA_DIR",
                sched_data.display().to_string(),
            ),
            ("EDGERUN_SCHEDULER_ADDR", sched_addr.clone()),
            (
                "EDGERUN_SCHEDULER_POLICY_SIGNING_KEY_HEX",
                key2_hex.to_string(),
            ),
            ("EDGERUN_SCHEDULER_POLICY_KEY_ID", key2_id.to_string()),
            ("EDGERUN_SCHEDULER_POLICY_VERSION", key2_ver.to_string()),
            (
                "EDGERUN_SCHEDULER_REQUIRE_WORKER_SIGNATURES",
                require_sig.clone(),
            ),
            (
                "EDGERUN_SCHEDULER_REQUIRE_RESULT_ATTESTATION",
                require_attest.clone(),
            ),
            (
                "EDGERUN_SCHEDULER_QUORUM_REQUIRES_ATTESTATION",
                quorum_attest.clone(),
            ),
        ],
    )
    .await?;
    wait_for_health(&client, &sched_url, &mut scheduler).await?;

    let policy: PolicyInfoResponse = client
        .get(format!("{sched_url}/v1/policy/info"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let key2_pub = policy.signer_pubkey;

    let mut worker_phase_a = spawn_worker(
        root,
        &worker_a_log,
        &[
            ("EDGERUN_WORKER_PUBKEY", worker_a.to_string()),
            ("EDGERUN_SCHEDULER_URL", sched_url.clone()),
            ("EDGERUN_WORKER_POLICY_KEY_ID_NEXT", key2_id.to_string()),
            ("EDGERUN_WORKER_POLICY_VERSION_NEXT", key2_ver.to_string()),
            ("EDGERUN_WORKER_POLICY_VERIFY_KEY_HEX_NEXT", key2_pub),
        ],
    )
    .await?;

    let job1 = create_assigned_job(&client, &sched_url, worker_a).await?;
    wait_for_failure_phase(&client, &sched_url, &job1, "assignment_policy_verify", true).await?;
    kill_child(&mut worker_phase_a).await;

    let mut worker_phase_b = spawn_worker(
        root,
        &worker_b_log,
        &[
            ("EDGERUN_WORKER_PUBKEY", worker_b.to_string()),
            ("EDGERUN_SCHEDULER_URL", sched_url.clone()),
        ],
    )
    .await?;
    let job2 = create_assigned_job(&client, &sched_url, worker_b).await?;
    wait_for_failure_phase(
        &client,
        &sched_url,
        &job2,
        "assignment_policy_verify",
        false,
    )
    .await?;

    kill_child(&mut worker_phase_b).await;
    kill_child(&mut scheduler).await;
    let _ = std::fs::remove_dir_all(tmp_dir);
    Ok(())
}

async fn run_integration_abi_rollover(root: &Path) -> Result<()> {
    let cfg = load_app_config(root)?;
    let (require_sig, require_attest, quorum_attest) = integration_flag_env(&cfg);
    let tmp_dir = create_temp_dir("edgerun-int-abi-rollover")?;
    let sched_log = tmp_dir.join("scheduler.log");
    let worker_log = tmp_dir.join("worker.log");
    let sched_data = tmp_dir.join("scheduler-data");
    std::fs::create_dir_all(&sched_data)?;

    let sched_port = pick_free_port()?;
    let sched_addr = format!("127.0.0.1:{sched_port}");
    let sched_url = format!("http://{sched_addr}");
    let worker_pubkey = "worker-abi-rollover";
    let client = reqwest::Client::new();

    let mut scheduler = spawn_scheduler(
        root,
        &sched_log,
        &[
            (
                "EDGERUN_SCHEDULER_DATA_DIR",
                sched_data.display().to_string(),
            ),
            ("EDGERUN_SCHEDULER_ADDR", sched_addr.clone()),
            (
                "EDGERUN_SCHEDULER_REQUIRE_WORKER_SIGNATURES",
                require_sig.clone(),
            ),
            (
                "EDGERUN_SCHEDULER_REQUIRE_RESULT_ATTESTATION",
                require_attest.clone(),
            ),
            (
                "EDGERUN_SCHEDULER_QUORUM_REQUIRES_ATTESTATION",
                quorum_attest.clone(),
            ),
        ],
    )
    .await?;
    wait_for_health(&client, &sched_url, &mut scheduler).await?;

    let mut worker = spawn_worker(
        root,
        &worker_log,
        &[
            ("EDGERUN_WORKER_PUBKEY", worker_pubkey.to_string()),
            ("EDGERUN_SCHEDULER_URL", sched_url.clone()),
        ],
    )
    .await?;

    let job_v1 = create_assigned_job_with_abi(&client, &sched_url, worker_pubkey, 1).await?;
    wait_for_runtime_execute_failure(&client, &sched_url, &job_v1).await?;
    let job_v2 = create_assigned_job_with_abi(&client, &sched_url, worker_pubkey, 2).await?;
    wait_for_runtime_execute_failure(&client, &sched_url, &job_v2).await?;

    let unsupported_body = json!({
        "runtime_id":"0000000000000000000000000000000000000000000000000000000000000000",
        "abi_version":3,
        "wasm_base64":"AA==",
        "input_base64":"",
        "limits":{"max_memory_bytes":1048576,"max_instructions":10000},
        "escrow_lamports":1,
        "assignment_worker_pubkey":worker_pubkey
    });
    let status = client
        .post(format!("{sched_url}/v1/job/create"))
        .json(&unsupported_body)
        .send()
        .await?
        .status();
    ensure(
        status.as_u16() == 400,
        "expected HTTP 400 for unsupported ABI",
    )?;

    kill_child(&mut worker).await;
    kill_child(&mut scheduler).await;
    let _ = std::fs::remove_dir_all(tmp_dir);
    Ok(())
}

async fn run_replay_corpus(root: &Path, config: &AppConfig) -> Result<()> {
    let out_dir = std::env::var("REPLAY_OUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root.join(".edgerun-replay-corpus"));
    std::fs::create_dir_all(&out_dir)?;

    let profile_debug = std::env::var("REPLAY_PROFILE_DEBUG")
        .ok()
        .or_else(|| config.runtime.replay_profile_debug.clone())
        .unwrap_or_else(|| "local-debug".to_string());
    let profile_release = std::env::var("REPLAY_PROFILE_RELEASE")
        .ok()
        .or_else(|| config.runtime.replay_profile_release.clone())
        .unwrap_or_else(|| "local-release".to_string());
    let runs = std::env::var("REPLAY_CORPUS_RUNS")
        .ok()
        .or_else(|| config.runtime.replay_runs.map(|v| v.to_string()))
        .unwrap_or_else(|| "3".to_string());

    let debug_artifact = out_dir.join(format!("{profile_debug}.json"));
    let release_artifact = out_dir.join(format!("{profile_release}.json"));
    run_program_sync_owned(
        "Replay debug",
        "cargo",
        &[
            "run".to_string(),
            "-p".to_string(),
            "edgerun-runtime".to_string(),
            "--".to_string(),
            "replay-corpus".to_string(),
            "--profile".to_string(),
            profile_debug.clone(),
            "--artifact".to_string(),
            debug_artifact.display().to_string(),
            "--runs".to_string(),
            runs.clone(),
        ],
        root,
        false,
    )?;
    run_program_sync_owned(
        "Replay release",
        "cargo",
        &[
            "run".to_string(),
            "--release".to_string(),
            "-p".to_string(),
            "edgerun-runtime".to_string(),
            "--".to_string(),
            "replay-corpus".to_string(),
            "--profile".to_string(),
            profile_release.clone(),
            "--artifact".to_string(),
            release_artifact.display().to_string(),
            "--runs".to_string(),
            runs.clone(),
        ],
        root,
        false,
    )?;

    compare_replay_profiles(&debug_artifact, &release_artifact)
}

async fn run_weekly_fuzz(root: &Path, config: &AppConfig) -> Result<()> {
    ensure(command_exists("cargo-fuzz"), "cargo-fuzz not installed")?;

    let artifact_dir = std::env::var("FUZZ_ARTIFACT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root.join(".edgerun-fuzz-weekly"));
    std::fs::create_dir_all(&artifact_dir)?;
    let secs = std::env::var("FUZZ_SECONDS_PER_TARGET")
        .ok()
        .or_else(|| {
            config
                .runtime
                .fuzz_seconds_per_target
                .map(|v| v.to_string())
        })
        .unwrap_or_else(|| "300".to_string());
    let fuzz_dir = root.join("crates/edgerun-runtime/fuzz");

    for target in [
        "fuzz_bundle_decode",
        "fuzz_validate_wasm",
        "fuzz_hostcall_boundary",
    ] {
        run_program_sync_owned(
            "Run fuzz target",
            "cargo",
            &[
                "fuzz".to_string(),
                "run".to_string(),
                target.to_string(),
                "--".to_string(),
                format!("-max_total_time={secs}"),
            ],
            &fuzz_dir,
            false,
        )?;
    }

    let crash_count = count_files_recursive(&fuzz_dir.join("artifacts"))?;
    ensure(crash_count == 0, "fuzz crashes detected in artifacts/")?;
    Ok(())
}

fn compare_replay_profiles(a_path: &Path, b_path: &Path) -> Result<()> {
    let a: Value = serde_json::from_slice(&std::fs::read(a_path)?)
        .with_context(|| format!("failed parsing {}", a_path.display()))?;
    let b: Value = serde_json::from_slice(&std::fs::read(b_path)?)
        .with_context(|| format!("failed parsing {}", b_path.display()))?;

    fn normalize(doc: &Value) -> BTreeMap<String, (Value, bool, bool)> {
        let mut out = BTreeMap::new();
        if let Some(cases) = doc["cases"].as_array() {
            for case in cases {
                if let Some(name) = case["case"].as_str() {
                    out.insert(
                        name.to_string(),
                        (
                            case.get("actual").cloned().unwrap_or(Value::Null),
                            case["passed"].as_bool().unwrap_or(false),
                            case["stable"].as_bool().unwrap_or(false),
                        ),
                    );
                }
            }
        }
        out
    }

    let a_cases = normalize(&a);
    let b_cases = normalize(&b);
    ensure(a_cases == b_cases, "replay profile mismatch detected")?;
    ensure(
        a_cases
            .values()
            .all(|(_, passed, stable)| *passed && *stable),
        "replay cases are not fully passed/stable in first profile",
    )?;
    Ok(())
}

fn validate_external_security_review(path: &Path) -> Result<()> {
    let doc: Value = serde_json::from_slice(&std::fs::read(path)?)
        .with_context(|| format!("failed parsing {}", path.display()))?;

    for key in [
        "review_cycle_id",
        "status",
        "provider",
        "scope_version",
        "sign_off",
        "findings",
    ] {
        ensure(
            doc.get(key).is_some(),
            &format!("missing top-level key: {key}"),
        )?;
    }

    let status = doc["status"].as_str().unwrap_or_default();
    ensure(
        matches!(status, "planned" | "in_progress" | "completed"),
        "invalid status",
    )?;

    let provider = &doc["provider"];
    ensure(provider.is_object(), "provider must be an object")?;
    ensure(
        provider.get("organization").is_some(),
        "missing provider.organization",
    )?;
    ensure(
        provider.get("reviewer").is_some(),
        "missing provider.reviewer",
    )?;

    let sign_off = &doc["sign_off"];
    ensure(sign_off.is_object(), "sign_off must be an object")?;
    for key in ["date", "approved", "notes"] {
        ensure(
            sign_off.get(key).is_some(),
            &format!("missing sign_off.{key}"),
        )?;
    }

    let findings = doc["findings"]
        .as_array()
        .ok_or_else(|| anyhow!("findings must be a list"))?;
    let mut unresolved_high_or_critical = Vec::new();
    for (i, finding) in findings.iter().enumerate() {
        ensure(
            finding.is_object(),
            &format!("finding[{i}] must be an object"),
        )?;
        for key in ["id", "title", "severity", "status", "owner", "notes"] {
            ensure(
                finding.get(key).is_some(),
                &format!("finding[{i}] missing key: {key}"),
            )?;
        }
        let severity = finding["severity"].as_str().unwrap_or_default();
        let finding_status = finding["status"].as_str().unwrap_or_default();
        ensure(
            matches!(severity, "low" | "medium" | "high" | "critical"),
            &format!("finding[{i}] invalid severity"),
        )?;
        ensure(
            matches!(finding_status, "open" | "closed" | "accepted_risk"),
            &format!("finding[{i}] invalid status"),
        )?;
        if (severity == "high" || severity == "critical") && finding_status != "closed" {
            unresolved_high_or_critical.push(
                finding["id"]
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "<unknown>".to_string()),
            );
        }
    }

    if status == "completed" {
        ensure(
            sign_off["approved"].as_bool().unwrap_or(false),
            "completed review requires sign_off.approved=true",
        )?;
        ensure(
            !sign_off["date"]
                .as_str()
                .unwrap_or_default()
                .trim()
                .is_empty(),
            "completed review requires non-empty sign_off.date",
        )?;
        let org = provider["organization"].as_str().unwrap_or_default().trim();
        let reviewer = provider["reviewer"].as_str().unwrap_or_default().trim();
        ensure(
            !org.is_empty() && org != "TBD",
            "completed review requires provider.organization",
        )?;
        ensure(
            !reviewer.is_empty() && reviewer != "TBD",
            "completed review requires provider.reviewer",
        )?;
        ensure(
            unresolved_high_or_critical.is_empty(),
            "completed review cannot have unresolved high/critical findings",
        )?;
    }

    Ok(())
}

async fn spawn_scheduler(
    root: &Path,
    log_path: &Path,
    envs: &[(&str, String)],
) -> Result<tokio::process::Child> {
    spawn_cargo_bin(root, log_path, "edgerun-scheduler", envs).await
}

async fn spawn_worker(
    root: &Path,
    log_path: &Path,
    envs: &[(&str, String)],
) -> Result<tokio::process::Child> {
    spawn_cargo_bin(root, log_path, "edgerun-worker", envs).await
}

async fn spawn_cargo_bin(
    root: &Path,
    log_path: &Path,
    package: &str,
    envs: &[(&str, String)],
) -> Result<tokio::process::Child> {
    let log_file = std::fs::File::create(log_path)
        .with_context(|| format!("failed to create {}", log_path.display()))?;
    let log_file_err = log_file.try_clone()?;
    let mut cmd = Command::new("cargo");
    cmd.arg("run")
        .arg("-p")
        .arg(package)
        .current_dir(root)
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(log_file_err));
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.spawn()
        .with_context(|| format!("failed to spawn cargo run -p {package}"))
}

async fn wait_for_health(
    client: &reqwest::Client,
    sched_url: &str,
    scheduler: &mut tokio::process::Child,
) -> Result<()> {
    for _ in 0..240 {
        if scheduler.try_wait()?.is_some() {
            break;
        }
        if client
            .get(format!("{sched_url}/health"))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
        {
            return Ok(());
        }
        sleep(Duration::from_millis(500)).await;
    }
    Err(anyhow!("scheduler failed to become healthy"))
}

async fn create_assigned_job(
    client: &reqwest::Client,
    sched_url: &str,
    worker: &str,
) -> Result<String> {
    create_assigned_job_with_abi(client, sched_url, worker, 2).await
}

async fn create_assigned_job_with_abi(
    client: &reqwest::Client,
    sched_url: &str,
    worker: &str,
    abi_version: u8,
) -> Result<String> {
    let body = json!({
        "runtime_id":"0000000000000000000000000000000000000000000000000000000000000000",
        "abi_version": abi_version,
        "wasm_base64":"AA==",
        "input_base64":"",
        "limits":{"max_memory_bytes":1048576,"max_instructions":10000},
        "escrow_lamports":1,
        "assignment_worker_pubkey":worker
    });
    let response: JobCreateResponse = client
        .post(format!("{sched_url}/v1/job/create"))
        .json(&body)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(response.job_id)
}

async fn wait_for_failure_phase(
    client: &reqwest::Client,
    sched_url: &str,
    job_id: &str,
    expected_phase: &str,
    invert: bool,
) -> Result<()> {
    for _ in 0..240 {
        let status: Value = client
            .get(format!("{sched_url}/v1/job/{job_id}"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let phase = status["failures"]
            .as_array()
            .and_then(|arr| arr.last())
            .and_then(|x| x["phase"].as_str())
            .unwrap_or("");
        if (!invert && phase == expected_phase)
            || (invert && !phase.is_empty() && phase != expected_phase)
        {
            return Ok(());
        }
        sleep(Duration::from_millis(500)).await;
    }
    Err(anyhow!("timed out waiting for expected failure phase"))
}

async fn wait_for_runtime_execute_failure(
    client: &reqwest::Client,
    sched_url: &str,
    job_id: &str,
) -> Result<()> {
    wait_for_failure_phase(client, sched_url, job_id, "runtime_execute", false).await
}

async fn kill_child(child: &mut tokio::process::Child) {
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
}

fn pick_free_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").context("failed to bind ephemeral port")?;
    let port = listener
        .local_addr()
        .context("failed to resolve local addr")?
        .port();
    drop(listener);
    Ok(port)
}

fn create_temp_dir(prefix: &str) -> Result<PathBuf> {
    let path = std::env::temp_dir().join(format!(
        "{}-{}-{}",
        prefix,
        now_unix_s(),
        std::process::id()
    ));
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

fn count_files_recursive(path: &Path) -> Result<usize> {
    if !path.exists() {
        return Ok(0);
    }
    let mut count = 0usize;
    let mut stack = vec![path.to_path_buf()];
    while let Some(next) = stack.pop() {
        for entry in std::fs::read_dir(&next)? {
            let entry = entry?;
            let entry_path = entry.path();
            if entry_path.is_dir() {
                stack.push(entry_path);
            } else if entry_path.is_file() {
                count += 1;
            }
        }
    }
    Ok(count)
}

fn load_app_config(root: &Path) -> Result<AppConfig> {
    let path = root.join("edgerun.toml");
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let cfg: AppConfig =
        toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(cfg)
}

fn integration_flag_env(config: &AppConfig) -> (String, String, String) {
    let sig = config
        .integration
        .require_worker_signatures
        .unwrap_or(false)
        .to_string();
    let att = config
        .integration
        .require_result_attestation
        .unwrap_or(false)
        .to_string();
    let quorum = config
        .integration
        .quorum_requires_attestation
        .unwrap_or(false)
        .to_string();
    (sig, att, quorum)
}

fn is_loopback_bind_addr(addr: &str) -> bool {
    if let Some(host) = addr
        .strip_prefix('[')
        .and_then(|rest| rest.split(']').next())
    {
        return host == "::1";
    }
    let host = addr.split(':').next().unwrap_or(addr);
    host == "127.0.0.1" || host == "localhost"
}

fn is_request_authorized(config: &AppConfig, headers: &HeaderMap) -> bool {
    let Some(expected) = config.web.auth_token.as_deref() else {
        return true;
    };
    headers
        .get("x-edgerun-token")
        .and_then(|v| v.to_str().ok())
        .map(|provided| provided == expected)
        .unwrap_or(false)
}

fn web_history_limit(config: &AppConfig) -> usize {
    config
        .web
        .history_limit
        .unwrap_or(DEFAULT_HISTORY_LIMIT)
        .max(1)
}

fn trim_history(history: &mut Vec<TaskRunRecord>, limit: usize) -> bool {
    if history.len() > limit {
        let drop_count = history.len() - limit;
        history.drain(0..drop_count);
        return true;
    }
    false
}

fn enforce_history_limit(tasks: &mut HashMap<String, TaskStatus>, limit: usize) -> bool {
    let mut changed = false;
    for task in tasks.values_mut() {
        if trim_history(&mut task.history, limit) {
            changed = true;
        }
    }
    changed
}

fn command_exists(cmd: &str) -> bool {
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(cmd);
        if candidate.is_file() {
            return true;
        }
    }
    false
}

fn ensure(ok: bool, msg: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(anyhow!(msg.to_string()))
    }
}

fn now_unix_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn truncate_output(mut s: String, max: usize) -> String {
    if s.len() > max {
        let start = s.len() - max;
        s = s[start..].to_string();
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn temp_dir(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "{}-{}-{}",
            prefix,
            now_unix_s(),
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn make_state(
        root: PathBuf,
        config: AppConfig,
        tasks: HashMap<String, TaskStatus>,
    ) -> WebState {
        let state_file = root.join(".edgerun-cli/task-status.json");
        let runs_dir = root.join(".edgerun-cli/runs");
        std::fs::create_dir_all(runs_dir.clone()).expect("runs dir");
        WebState {
            root,
            config,
            tasks: Arc::new(Mutex::new(tasks)),
            queue: Arc::new(Mutex::new(VecDeque::new())),
            running_cancel: Arc::new(Mutex::new(HashMap::new())),
            dispatcher_running: Arc::new(AtomicBool::new(false)),
            state_file,
            runs_dir,
        }
    }

    #[tokio::test]
    async fn web_status_and_unknown_task() {
        let root = temp_dir("edgerun-web-test");
        let state = make_state(root, AppConfig::default(), HashMap::new());
        let app = build_router(state);

        let status_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("status response");
        assert_eq!(status_resp.status(), StatusCode::OK);

        let missing_resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/run/not-a-task")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("missing response");
        assert_eq!(missing_resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn web_log_endpoint_not_found_without_log() {
        let root = temp_dir("edgerun-web-log-test");
        let mut tasks = HashMap::new();
        tasks.insert(
            "doctor".to_string(),
            TaskStatus {
                task: "doctor".to_string(),
                state: "idle".to_string(),
                started_at_unix_s: None,
                finished_at_unix_s: None,
                runs: 0,
                last_exit: None,
                last_output: String::new(),
                history: Vec::new(),
                last_log_path: None,
            },
        );
        let state = make_state(root, AppConfig::default(), tasks);
        let app = build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/log/doctor")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("log response");
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn config_loads_from_toml() {
        let root = temp_dir("edgerun-config-test");
        std::fs::write(
            root.join("edgerun.toml"),
            r#"
[web]
default_addr = "127.0.0.1:9000"
refresh_ms = 1500
history_limit = 12
auth_token = "secret"
allow_remote_bind = true

[runtime]
replay_runs = 7
fuzz_seconds_per_target = 120

[integration]
require_worker_signatures = true
"#,
        )
        .expect("write config");

        let cfg = load_app_config(&root).expect("load config");
        assert_eq!(cfg.web.default_addr.as_deref(), Some("127.0.0.1:9000"));
        assert_eq!(cfg.web.refresh_ms, Some(1500));
        assert_eq!(cfg.web.history_limit, Some(12));
        assert_eq!(cfg.web.auth_token.as_deref(), Some("secret"));
        assert_eq!(cfg.web.allow_remote_bind, Some(true));
        assert_eq!(cfg.runtime.replay_runs, Some(7));
        assert_eq!(cfg.runtime.fuzz_seconds_per_target, Some(120));
        assert_eq!(cfg.integration.require_worker_signatures, Some(true));
    }

    #[test]
    fn trim_history_keeps_latest_records() {
        let mut history = (0..6)
            .map(|i| TaskRunRecord {
                started_at_unix_s: Some(i),
                finished_at_unix_s: Some(i),
                state: "success".to_string(),
                exit: Some(0),
                output: format!("run-{i}"),
                log_path: None,
            })
            .collect::<Vec<_>>();
        let changed = trim_history(&mut history, 3);
        assert!(changed);
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].output, "run-3");
        assert_eq!(history[2].output, "run-5");
    }

    #[tokio::test]
    async fn ui_page_applies_refresh_floor_and_renders_history_limit() {
        let root = temp_dir("edgerun-ui-test");
        let state = make_state(
            root,
            AppConfig {
                web: WebConfig {
                    default_addr: None,
                    refresh_ms: Some(120),
                    history_limit: Some(9),
                    auth_token: Some("abc".to_string()),
                    allow_remote_bind: None,
                },
                runtime: RuntimeConfig::default(),
                integration: IntegrationConfig::default(),
            },
            HashMap::new(),
        );
        let body = ui_page(State(state)).await.0;
        assert!(body.contains("setInterval(refresh, 500);"));
        assert!(body.contains("history retention: last 9 runs per task"));
        assert!(body.contains("const TOKEN_REQUIRED = true;"));
    }

    #[tokio::test]
    async fn web_api_requires_auth_when_configured() {
        let root = temp_dir("edgerun-web-auth-test");
        let state = make_state(
            root,
            AppConfig {
                web: WebConfig {
                    auth_token: Some("top-secret".to_string()),
                    ..WebConfig::default()
                },
                ..AppConfig::default()
            },
            HashMap::new(),
        );
        let app = build_router(state);
        let unauthorized = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("unauthorized response");
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

        let authorized = app
            .oneshot(
                Request::builder()
                    .uri("/api/status")
                    .header("x-edgerun-token", "top-secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("authorized response");
        assert_eq!(authorized.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn cancel_queued_task_updates_state() {
        let root = temp_dir("edgerun-web-cancel-queue-test");
        let state = make_state(root, AppConfig::default(), HashMap::new());
        {
            let mut queue = state.queue.lock().await;
            queue.push_back("doctor".to_string());
        }
        {
            let mut tasks = state.tasks.lock().await;
            tasks.insert(
                "doctor".to_string(),
                TaskStatus {
                    task: "doctor".to_string(),
                    state: "queued".to_string(),
                    started_at_unix_s: None,
                    finished_at_unix_s: None,
                    runs: 1,
                    last_exit: None,
                    last_output: String::new(),
                    history: Vec::new(),
                    last_log_path: None,
                },
            );
        }
        let app = build_router(state.clone());
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/cancel/doctor")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("cancel response");
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let tasks = state.tasks.lock().await;
        let doctor = tasks.get("doctor").expect("doctor state");
        assert_eq!(doctor.state, "canceled");
    }

    #[tokio::test]
    async fn cancel_running_task_requests_cancellation() {
        let root = temp_dir("edgerun-web-cancel-running-test");
        let state = make_state(root, AppConfig::default(), HashMap::new());
        let notify = Arc::new(Notify::new());
        {
            let mut tasks = state.tasks.lock().await;
            tasks.insert(
                "doctor".to_string(),
                TaskStatus {
                    task: "doctor".to_string(),
                    state: "running".to_string(),
                    started_at_unix_s: Some(now_unix_s()),
                    finished_at_unix_s: None,
                    runs: 1,
                    last_exit: None,
                    last_output: String::new(),
                    history: Vec::new(),
                    last_log_path: None,
                },
            );
        }
        {
            let mut running_cancel = state.running_cancel.lock().await;
            running_cancel.insert("doctor".to_string(), notify);
        }
        let app = build_router(state.clone());
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/cancel/doctor")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("cancel response");
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let tasks = state.tasks.lock().await;
        let doctor = tasks.get("doctor").expect("doctor state");
        assert_eq!(doctor.state, "canceling");
    }

    #[tokio::test]
    async fn run_task_is_queued_when_another_task_is_running() {
        let root = temp_dir("edgerun-web-queue-while-running-test");
        let mut tasks = HashMap::new();
        tasks.insert(
            "doctor".to_string(),
            TaskStatus {
                task: "doctor".to_string(),
                state: "running".to_string(),
                started_at_unix_s: Some(now_unix_s()),
                finished_at_unix_s: None,
                runs: 1,
                last_exit: None,
                last_output: String::new(),
                history: Vec::new(),
                last_log_path: None,
            },
        );
        let state = make_state(root, AppConfig::default(), tasks);
        let app = build_router(state.clone());
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/run/setup")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("run response");
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        {
            let tasks = state.tasks.lock().await;
            let setup = tasks.get("setup").expect("setup state");
            assert_eq!(setup.state, "queued");
        }
        let queue = state.queue.lock().await;
        assert_eq!(queue.front().map(String::as_str), Some("setup"));
    }

    #[test]
    fn bind_addr_loopback_policy() {
        assert!(is_loopback_bind_addr("127.0.0.1:8787"));
        assert!(is_loopback_bind_addr("localhost:8787"));
        assert!(is_loopback_bind_addr("[::1]:8787"));
        assert!(!is_loopback_bind_addr("0.0.0.0:8787"));
    }

    #[test]
    fn parse_storage_metrics_lines() {
        let writes = "writes: total=1234567 (250000 ops/s)";
        let reads = "reads: total=7654321 (80000 ops/s) hit_rate=67.50%";
        let comp = "compaction: scheduled=8 completed=7 failed=0 skipped=1 total_ms=321";
        assert_eq!(parse_ops_per_second(writes), Some(250000));
        assert_eq!(parse_ops_per_second(reads), Some(80000));
        assert_eq!(parse_hit_rate_pct(reads), Some(67.50));
        assert_eq!(parse_line_u64(comp, "scheduled"), Some(8));
        assert_eq!(parse_line_u64(comp, "completed"), Some(7));
        assert_eq!(parse_line_u64(comp, "failed"), Some(0));
        assert_eq!(parse_line_u64(comp, "skipped"), Some(1));
        assert_eq!(parse_line_u64(comp, "total_ms"), Some(321));
    }

    #[test]
    fn extract_mode_mbps_from_benchmark_output() {
        let sample = "\
--- Mode: end_to_end ---
Throughput: 432.10 MB/s
--- Mode: io_only ---
Throughput: 1900.55 MB/s
";
        assert_eq!(
            extract_mode_throughput_mbps(sample, "end_to_end"),
            Some(432.10)
        );
        assert_eq!(
            extract_mode_throughput_mbps(sample, "io_only"),
            Some(1900.55)
        );
        assert_eq!(extract_mode_throughput_mbps(sample, "missing"), None);
    }
}
