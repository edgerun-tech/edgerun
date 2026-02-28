// SPDX-License-Identifier: Apache-2.0
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;

mod commands;
mod integration_helpers;
mod process_helpers;

use crate::commands::ci::run_ci;
use crate::commands::event_bus::run_event_bus_command;
use crate::commands::execution::run_execution_command;
use crate::commands::integration::{
    run_integration_abi_rollover, run_integration_e2e_lifecycle, run_integration_policy_rotation,
    run_integration_scheduler_api,
};
use crate::commands::observer::run_observer_command;
use crate::commands::runtime_ops::{
    compare_replay_profiles, run_replay_corpus, run_weekly_fuzz, validate_external_security_review,
};
use crate::commands::storage::run_storage_command;
use crate::commands::tailscale::run_tailscale_command;
use crate::commands::tasks::{run_interactive, run_named_task_sync};
use crate::commands::timeline::run_timeline_command;
use process_helpers::{command_exists, run_program_sync, run_program_sync_owned};

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
    Tailscale {
        #[command(subcommand)]
        command: TailscaleCommand,
    },
    Observe {
        #[command(subcommand)]
        command: ObserveCommand,
    },
    Event {
        #[command(subcommand)]
        command: EventBusCommand,
    },
    Timeline {
        #[command(subcommand)]
        command: TimelineCommand,
    },
    Execution {
        #[command(subcommand)]
        command: ExecutionCommand,
    },
}

#[derive(Clone, Debug, ValueEnum)]
enum BuildTarget {
    Workspace,
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

#[derive(Subcommand, Debug, Clone)]
enum TailscaleCommand {
    Bridge {
        #[arg(long, default_value = "127.0.0.1:49201")]
        listen: SocketAddr,
        #[arg(long, default_value_t = 5577)]
        term_port: u16,
        #[arg(long, default_value_t = false)]
        include_offline: bool,
    },
    Dev {
        #[arg(long, default_value = "127.0.0.1:49201")]
        bridge_listen: SocketAddr,
        #[arg(long, default_value_t = 5180)]
        port: u16,
        #[arg(long)]
        web_root: Option<PathBuf>,
        #[arg(long, default_value = "allow-software")]
        hardware_mode: String,
        #[arg(long, default_value_t = false)]
        include_offline: bool,
        #[arg(long, default_value_t = false)]
        tailscale_serve: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
enum ObserveCommand {
    Append {
        #[arg(long)]
        job_id: String,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        actor: String,
        #[arg(long)]
        event_type: String,
        #[arg(long)]
        payload_json: Option<String>,
        #[arg(long)]
        payload_file: Option<PathBuf>,
        #[arg(long)]
        prev_event_hash: Option<String>,
        #[arg(long)]
        data_dir: Option<PathBuf>,
        #[arg(long, default_value = "long-jobs.seg")]
        segment: String,
        #[arg(value_enum, long, default_value_t = ObserveDurability::Local)]
        durability: ObserveDurability,
    },
    IngestStdio {
        #[arg(long)]
        job_id: String,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        actor: String,
        #[arg(long)]
        event_type: String,
        #[arg(long)]
        data_dir: Option<PathBuf>,
        #[arg(long, default_value = "long-jobs.seg")]
        segment: String,
        #[arg(value_enum, long, default_value_t = ObserveDurability::Local)]
        durability: ObserveDurability,
    },
    IngestGitChanges {
        #[arg(long)]
        job_id: String,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        actor: String,
        #[arg(long, default_value = "fs.changed")]
        event_type: String,
        #[arg(long, default_value = "HEAD")]
        base_ref: String,
        #[arg(long)]
        repo_root: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        include_untracked: bool,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        #[arg(long)]
        data_dir: Option<PathBuf>,
        #[arg(long, default_value = "long-jobs.seg")]
        segment: String,
        #[arg(value_enum, long, default_value_t = ObserveDurability::Local)]
        durability: ObserveDurability,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ObserveDurability {
    Buffered,
    Local,
    Durable,
    Checkpointed,
}

#[derive(Subcommand, Debug, Clone)]
pub enum EventBusCommand {
    Submit {
        #[arg(long)]
        data_dir: Option<PathBuf>,
        #[arg(long, default_value = "events.seg")]
        segment: String,
        #[arg(long)]
        nonce: u64,
        #[arg(long)]
        publisher: String,
        #[arg(long)]
        signature: String,
        #[arg(long)]
        policy_id: String,
        #[arg(long = "recipient")]
        recipients: Vec<String>,
        #[arg(long)]
        payload_type: String,
        #[arg(long)]
        payload_base64: String,
    },
    Query {
        #[arg(long)]
        data_dir: Option<PathBuf>,
        #[arg(long, default_value = "events.seg")]
        segment: String,
        #[arg(long, default_value_t = 100)]
        limit: usize,
        #[arg(long, default_value_t = 0)]
        cursor_offset: u64,
        #[arg(long)]
        publisher: Option<String>,
        #[arg(long)]
        payload_type: Option<String>,
    },
    Status {
        #[arg(long)]
        data_dir: Option<PathBuf>,
        #[arg(long, default_value = "events.seg")]
        segment: String,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum TimelineCommand {
    Append {
        #[arg(long)]
        data_dir: Option<PathBuf>,
        #[arg(long, default_value = "interactions.seg")]
        segment: String,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        job_id: Option<String>,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(value_enum, long, default_value_t = TimelineActor::User)]
        actor: TimelineActor,
        #[arg(long, default_value = "interactive")]
        actor_id: String,
        #[arg(value_enum, long)]
        kind: TimelineEventKind,
        #[arg(long)]
        text: Option<String>,
        #[arg(long)]
        text_file: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        stdin: bool,
    },
    Query {
        #[arg(long)]
        data_dir: Option<PathBuf>,
        #[arg(long, default_value = "interactions.seg")]
        segment: String,
        #[arg(long, default_value_t = 100)]
        limit: usize,
        #[arg(long, default_value_t = 0)]
        cursor_offset: u64,
        #[arg(value_enum, long)]
        kind: Option<TimelineEventKind>,
        #[arg(long)]
        run_id: Option<String>,
        #[arg(long)]
        job_id: Option<String>,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long)]
        actor_id: Option<String>,
        #[arg(long)]
        payload_type: Option<String>,
    },
    Status {
        #[arg(long)]
        data_dir: Option<PathBuf>,
        #[arg(long, default_value = "interactions.seg")]
        segment: String,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum TimelineActor {
    User,
    Agent,
    System,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum TimelineEventKind {
    UserInput,
    AgentOutput,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ExecutionCommand {
    IntentSubmitted {
        #[arg(long)]
        data_dir: Option<PathBuf>,
        #[arg(long, default_value = "interactions.seg")]
        segment: String,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        job_id: Option<String>,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long, default_value = "system")]
        actor_id: String,
        #[arg(long)]
        intent_id: String,
        #[arg(long)]
        intent_text: String,
    },
    ExecutionStarted {
        #[arg(long)]
        data_dir: Option<PathBuf>,
        #[arg(long, default_value = "interactions.seg")]
        segment: String,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        job_id: Option<String>,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long, default_value = "executor")]
        actor_id: String,
        #[arg(long)]
        intent_id: String,
        #[arg(long)]
        executor_id: String,
    },
    StepStarted {
        #[arg(long)]
        data_dir: Option<PathBuf>,
        #[arg(long, default_value = "interactions.seg")]
        segment: String,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        job_id: Option<String>,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long, default_value = "executor")]
        actor_id: String,
        #[arg(long)]
        step_id: String,
    },
    StepFinished {
        #[arg(long)]
        data_dir: Option<PathBuf>,
        #[arg(long, default_value = "interactions.seg")]
        segment: String,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        job_id: Option<String>,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long, default_value = "executor")]
        actor_id: String,
        #[arg(long)]
        step_id: String,
        #[arg(value_enum, long)]
        state: ExecutionStateArg,
        #[arg(long)]
        reason: Option<String>,
    },
    ExecutionFinished {
        #[arg(long)]
        data_dir: Option<PathBuf>,
        #[arg(long, default_value = "interactions.seg")]
        segment: String,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        job_id: Option<String>,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long, default_value = "executor")]
        actor_id: String,
        #[arg(value_enum, long)]
        state: ExecutionStateArg,
        #[arg(long)]
        reason: Option<String>,
    },
    QueryRun {
        #[arg(long)]
        data_dir: Option<PathBuf>,
        #[arg(long, default_value = "interactions.seg")]
        segment: String,
        #[arg(long)]
        run_id: String,
        #[arg(long, default_value_t = 200)]
        limit: usize,
        #[arg(long, default_value_t = 0)]
        cursor_offset: u64,
        #[arg(long, default_value_t = false)]
        protobuf: bool,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ExecutionStateArg {
    Pending,
    Running,
    Succeeded,
    Failed,
    Halted,
}

#[derive(Clone, Debug, Deserialize, Default)]
struct AppConfig {
    #[serde(default)]
    runtime: RuntimeConfig,
    #[serde(default)]
    integration: IntegrationConfig,
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

const CLI_BUILD_NUMBER: &str = match option_env!("EDGERUN_BUILD_NUMBER") {
    Some(v) => v,
    None => "dev",
};

#[tokio::main]
async fn main() -> Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();

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
        Commands::CompareReplay { left, right } => compare_replay_profiles(&left, &right)?,
        Commands::ValidateSecurity { path } => {
            let p =
                path.unwrap_or_else(|| root.join("crates/edgerun-runtime/SECURITY_FINDINGS.json"));
            validate_external_security_review(&p)?
        }
        Commands::Storage { command } => run_storage_command(&root, command)?,
        Commands::Tailscale { command } => run_tailscale_command(&root, command).await?,
        Commands::Observe { command } => run_observer_command(&root, command)?,
        Commands::Event { command } => run_event_bus_command(&root, command)?,
        Commands::Timeline { command } => run_timeline_command(&root, command)?,
        Commands::Execution { command } => run_execution_command(&root, command)?,
    }

    Ok(())
}

fn run_build_target(root: &Path, target: BuildTarget) -> Result<()> {
    match target {
        BuildTarget::Workspace => run_named_task_sync(root, "build-workspace"),
        BuildTarget::All => run_named_task_sync(root, "build-workspace"),
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

fn run_doctor_sync(root: &Path) -> Result<()> {
    run_program_sync("Rust cargo", "cargo", &["--version"], root, false)?;
    run_program_sync("Rust compiler", "rustc", &["--version"], root, false)?;
    run_program_sync("Rustup", "rustup", &["--version"], root, false)?;
    run_program_sync("Bun", "bun", &["--version"], root, true)?;
    run_program_sync("cargo-fuzz", "cargo-fuzz", &["--version"], root, true)?;
    run_program_sync("Python3", "python3", &["--version"], root, false)?;
    run_program_sync("curl", "curl", &["--version"], root, false)?;
    Ok(())
}

fn run_setup_sync(root: &Path, install_missing: bool) -> Result<()> {
    run_program_sync("Cargo fetch", "cargo", &["fetch", "--locked"], root, false)?;

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

fn remove_dir_if_exists(path: &Path) -> Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)
            .with_context(|| format!("failed to remove directory: {}", path.display()))?;
    }
    Ok(())
}

fn run_clean_artifacts_sync(root: &Path) -> Result<()> {
    remove_dir_if_exists(&root.join("out"))?;
    remove_dir_if_exists(&root.join("target"))?;
    remove_dir_if_exists(&root.join("test-ledger"))?;
    remove_dir_if_exists(&root.join("frontend/test-results"))?;
    remove_dir_if_exists(&root.join("frontend/playwright-report"))?;
    println!("cleaned artifacts");
    Ok(())
}

fn run_build_runtime_web_sync(root: &Path, sync_frontend: bool) -> Result<()> {
    let crate_dir = root.join("crates/edgerun-runtime-web");
    let pkg_dir = crate_dir.join("pkg-web");
    let frontend_out = root.join("frontend/public/wasm/edgerun-runtime-web");

    ensure(
        command_exists("wasm-pack"),
        "wasm-pack is required (cargo install wasm-pack)",
    )?;
    fs::create_dir_all(&pkg_dir)?;
    run_program_sync(
        "Build runtime web wasm package",
        "wasm-pack",
        &[
            "build",
            "--target",
            "web",
            "--release",
            "--out-dir",
            "pkg-web",
        ],
        &crate_dir,
        false,
    )?;

    if sync_frontend && root.join("frontend").is_dir() {
        fs::create_dir_all(&frontend_out)?;
        copy_dir_contents(&pkg_dir, &frontend_out)?;
        println!("synced wasm package to: {}", frontend_out.display());
    }

    Ok(())
}

fn copy_dir_contents(src: &Path, dst: &Path) -> Result<()> {
    for entry in fs::read_dir(src).with_context(|| format!("failed to read {}", src.display()))? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_dir_contents(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).with_context(|| {
                format!(
                    "failed to copy {} -> {}",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn run_verify_sync(root: &Path) -> Result<()> {
    run_program_sync(
        "Run worker/scheduler Rust tests",
        "cargo",
        &[
            "test",
            "-p",
            "edgerun-worker",
            "-p",
            "edgerun-scheduler",
            "--quiet",
        ],
        root,
        false,
    )
}

fn run_matrix_validation_check_sync(root: &Path) -> Result<()> {
    let matrix_file = root.join("docs/WHITEPAPER_IMPLEMENTATION_MATRIX.mdx");
    let content = fs::read_to_string(&matrix_file)
        .with_context(|| format!("missing {}", matrix_file.display()))?;

    let mut in_section = false;
    let mut bad = Vec::new();
    for line in content.lines() {
        if line.starts_with("## ") {
            in_section = line == "## Test Coverage (Implemented)";
            continue;
        }
        if !in_section || !line.starts_with('|') {
            continue;
        }
        if line.starts_with("| Scenario |")
            || line
                .chars()
                .all(|c| c == '|' || c == '-' || c == ' ' || c == '\t')
        {
            continue;
        }
        let parts: Vec<&str> = line.split('|').map(str::trim).collect();
        if parts.len() < 5 {
            bad.push(format!("malformed row: {line}"));
            continue;
        }
        let status = parts[2];
        let validation = parts[3];
        if status == "Implemented" && validation.is_empty() {
            bad.push(format!("missing Validation for Implemented row: {line}"));
        }
    }

    if bad.is_empty() {
        println!("matrix validation OK: all Implemented test-coverage rows include Validation references");
        Ok(())
    } else {
        for issue in bad {
            eprintln!("error: {issue}");
        }
        Err(anyhow!("matrix validation failed"))
    }
}

fn expected_spdx_for_path(path: &str) -> Option<&'static str> {
    match path {
        p if p.starts_with("crates/edgerun-scheduler/") => Some("LicenseRef-Edgerun-Proprietary"),
        p if p.starts_with("crates/edgerun-storage/") => Some("GPL-2.0-only"),
        p if p.starts_with("crates/edgerun-cli/")
            || p.starts_with("crates/edgerun-runtime/")
            || p.starts_with("crates/edgerun-worker/")
            || p.starts_with("docs/")
            || p.starts_with("scripts/") =>
        {
            Some("Apache-2.0")
        }
        _ => None,
    }
}

fn is_supported_spdx_source(path: &str) -> bool {
    [
        ".rs", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".sh", ".bash", ".zsh", ".py", ".yml",
        ".yaml", ".toml",
    ]
    .iter()
    .any(|ext| path.ends_with(ext))
}

fn is_generated_spdx_exempt(path: &str) -> bool {
    path.starts_with("out/")
}

fn run_spdx_check_sync(root: &Path) -> Result<()> {
    let output = std::process::Command::new("git")
        .arg("ls-files")
        .current_dir(root)
        .output()
        .context("failed to run git ls-files")?;
    ensure(output.status.success(), "git ls-files failed")?;

    let files = String::from_utf8(output.stdout).context("git ls-files output was not utf8")?;
    let mut errors = 0usize;

    for path in files.lines() {
        if path.is_empty() || path.ends_with("/LICENSE") || path == "LICENSE" {
            continue;
        }
        if is_generated_spdx_exempt(path) {
            continue;
        }
        if !is_supported_spdx_source(path) {
            continue;
        }
        let Some(expected) = expected_spdx_for_path(path) else {
            continue;
        };

        let full = root.join(path);
        let content = fs::read_to_string(&full).unwrap_or_default();
        let actual = content
            .lines()
            .take(5)
            .find(|line| line.contains("SPDX-License-Identifier:"));
        match actual {
            None => {
                eprintln!("missing SPDX header: {path} (expected {expected})");
                errors += 1;
            }
            Some(line) if !line.contains(&format!("SPDX-License-Identifier: {expected}")) => {
                eprintln!("wrong SPDX header: {path}");
                eprintln!("  expected: SPDX-License-Identifier: {expected}");
                eprintln!("  actual:   {line}");
                errors += 1;
            }
            _ => {}
        }
    }

    if errors > 0 {
        Err(anyhow!("SPDX check failed with {errors} issue(s)"))
    } else {
        println!("SPDX check passed.");
        Ok(())
    }
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

fn ensure(ok: bool, msg: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(anyhow!(msg.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn now_unix_s() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

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

    #[test]
    fn config_loads_from_toml() {
        let root = temp_dir("edgerun-config-test");
        std::fs::write(
            root.join("edgerun.toml"),
            r#"
[runtime]
replay_runs = 7
fuzz_seconds_per_target = 120

[integration]
require_worker_signatures = true
"#,
        )
        .expect("write config");

        let cfg = load_app_config(&root).expect("load config");
        assert_eq!(cfg.runtime.replay_runs, Some(7));
        assert_eq!(cfg.runtime.fuzz_seconds_per_target, Some(120));
        assert_eq!(cfg.integration.require_worker_signatures, Some(true));
    }
}
