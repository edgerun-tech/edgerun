// SPDX-License-Identifier: Apache-2.0
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use axum::http::{HeaderValue, Method};
use axum::{Json, Router, extract::State, response::IntoResponse, routing::get};
use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::process::Command;
use tokio::time::{sleep, timeout};
use tower_http::cors::{Any, CorsLayer};

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

#[derive(Subcommand, Debug, Clone)]
enum TailscaleCommand {
    Bridge {
        #[arg(long, default_value = "127.0.0.1:49201")]
        listen: SocketAddr,
        #[arg(long, default_value_t = 8080)]
        term_port: u16,
        #[arg(long, default_value_t = false)]
        include_offline: bool,
    },
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

#[derive(Clone, Debug)]
struct TailscaleBridgeState {
    term_port: u16,
    include_offline: bool,
}

#[derive(Debug, Serialize)]
struct TailscaleBridgeDevice {
    name: String,
    base_url: String,
    online: bool,
    source: &'static str,
}

const CLI_BUILD_NUMBER: &str = match option_env!("EDGERUN_BUILD_NUMBER") {
    Some(v) => v,
    None => "dev",
};

fn ci_timeout_duration() -> Duration {
    let secs = std::env::var("EDGERUN_CI_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(30 * 60);
    Duration::from_secs(secs)
}

async fn run_future_with_timeout<T>(
    label: &str,
    fut: impl std::future::Future<Output = Result<T>>,
) -> Result<T> {
    timeout(ci_timeout_duration(), fut)
        .await
        .with_context(|| format!("{label} timed out"))?
}

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
        Commands::CompareReplay { left, right } => compare_replay_profiles(&left, &right)?,
        Commands::ValidateSecurity { path } => {
            let p =
                path.unwrap_or_else(|| root.join("crates/edgerun-runtime/SECURITY_FINDINGS.json"));
            validate_external_security_review(&p)?
        }
        Commands::Storage { command } => run_storage_command(&root, command)?,
        Commands::Tailscale { command } => run_tailscale_command(command).await?,
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

    let act_supported_job = matches!(
        job_name.as_str(),
        "all"
            | "build-timing-main"
            | "rust-checks"
            | "coverage"
            | "integration"
            | "runtime-determinism"
            | "runtime-calibration"
            | "runtime-slo"
            | "runtime-fuzz-sanity"
            | "runtime-ub-safety"
            | "runtime-security"
            | "program-localnet"
    );
    if command_exists("act") && act_supported_job {
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
            run_spdx_check_sync(root)?;
            run_rust_checks_sync(root)?;
            run_matrix_validation_check_sync(root)?;
            run_future_with_timeout(
                "integration scheduler api",
                run_integration_scheduler_api(root),
            )
            .await?;
            run_future_with_timeout(
                "integration e2e lifecycle",
                run_integration_e2e_lifecycle(root),
            )
            .await?;
            run_future_with_timeout(
                "integration policy rotation",
                run_integration_policy_rotation(root),
            )
            .await?;
            run_future_with_timeout(
                "integration abi rollover",
                run_integration_abi_rollover(root),
            )
            .await?;
            run_future_with_timeout(
                "runtime replay corpus",
                run_replay_corpus(root, &load_app_config(root)?),
            )
            .await?;
            validate_external_security_review(
                &root.join("crates/edgerun-runtime/SECURITY_FINDINGS.json"),
            )?;
            Ok(())
        }
        "spdx-check" => run_spdx_check_sync(root),
        "matrix-check" => run_matrix_validation_check_sync(root),
        "clean-artifacts" => run_clean_artifacts_sync(root),
        "build-runtime-web" => run_build_runtime_web_sync(root, true),
        "verify" => run_verify_sync(root),
        "build-timing-main" => run_build_timing_sync(root),
        "rust-checks" => run_rust_checks_sync(root),
        "integration" => {
            run_future_with_timeout(
                "integration scheduler api",
                run_integration_scheduler_api(root),
            )
            .await?;
            run_future_with_timeout(
                "integration e2e lifecycle",
                run_integration_e2e_lifecycle(root),
            )
            .await?;
            run_future_with_timeout(
                "integration policy rotation",
                run_integration_policy_rotation(root),
            )
            .await?;
            run_future_with_timeout(
                "integration abi rollover",
                run_integration_abi_rollover(root),
            )
            .await
        }
        "runtime-determinism" => {
            run_future_with_timeout(
                "runtime replay corpus",
                run_replay_corpus(root, &load_app_config(root)?),
            )
            .await
        }
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
        "spdx-check" => {
            run_spdx_check_sync(root)?;
            Ok(true)
        }
        "matrix-check" => {
            run_matrix_validation_check_sync(root)?;
            Ok(true)
        }
        "clean-artifacts" => {
            run_clean_artifacts_sync(root)?;
            Ok(true)
        }
        "build-runtime-web" => {
            run_build_runtime_web_sync(root, true)?;
            Ok(true)
        }
        "verify" => {
            run_verify_sync(root)?;
            Ok(true)
        }
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

fn remove_dir_if_exists(path: &Path) -> Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)
            .with_context(|| format!("failed to remove directory: {}", path.display()))?;
    }
    Ok(())
}

fn remove_file_if_exists(path: &Path) -> Result<()> {
    if path.is_file() {
        fs::remove_file(path)
            .with_context(|| format!("failed to remove file: {}", path.display()))?;
    }
    Ok(())
}

fn run_clean_artifacts_sync(root: &Path) -> Result<()> {
    remove_dir_if_exists(&root.join("out"))?;
    remove_dir_if_exists(&root.join("target"))?;
    remove_dir_if_exists(&root.join("test-ledger"))?;
    remove_dir_if_exists(&root.join("frontend/test-results"))?;
    remove_dir_if_exists(&root.join("frontend/playwright-report"))?;
    remove_file_if_exists(&root.join("solana-validator.log"))?;
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
    )?;
    run_program_anchor_verify_sync(root)
}

fn run_program_anchor_verify_sync(root: &Path) -> Result<()> {
    let program_root = root.join("program");
    let env = program_tool_env(&program_root);

    ensure(
        command_exists("cargo-build-sbf"),
        "cargo-build-sbf not found on PATH",
    )?;

    let so_path = program_root.join("target/deploy/edgerun.so");
    let manifest = program_root.join("programs/edgerun_program/Cargo.toml");
    let source_dir = program_root.join("programs/edgerun_program/src");

    let needs_sbf = !so_path.is_file()
        || file_is_newer(&manifest, &so_path)?
        || any_file_newer(&source_dir, &so_path)?;
    if needs_sbf {
        run_program_sync_with_env(
            "Build SBF artifact",
            "cargo",
            &[
                "build-sbf",
                "--manifest-path",
                "programs/edgerun_program/Cargo.toml",
                "--sbf-out-dir",
                "target/deploy",
            ],
            &program_root,
            false,
            &env,
        )?;
    }

    let idl_path = program_root.join("target/idl/edgerun_program.json");
    let needs_idl = !idl_path.is_file()
        || file_is_newer(&manifest, &idl_path)?
        || any_file_newer(&source_dir, &idl_path)?;
    if needs_idl {
        fs::create_dir_all(idl_path.parent().unwrap_or(&program_root))?;
        run_program_sync_with_env(
            "Build Program IDL",
            "anchor",
            &[
                "idl",
                "build",
                "-p",
                "edgerun_program",
                "-o",
                "target/idl/edgerun_program.json",
            ],
            &program_root,
            false,
            &env,
        )?;
    }

    let idl_alias = program_root.join("target/idl/edgerun.json");
    if idl_path.is_file() && !idl_alias.exists() {
        #[cfg(unix)]
        std::os::unix::fs::symlink("edgerun_program.json", &idl_alias)
            .with_context(|| format!("failed to symlink {}", idl_alias.display()))?;
    }

    run_program_sync_with_env(
        "Anchor test --skip-build",
        "anchor",
        &["test", "--skip-build"],
        &program_root,
        false,
        &env,
    )
}

fn file_is_newer(candidate: &Path, base: &Path) -> Result<bool> {
    if !candidate.is_file() || !base.is_file() {
        return Ok(false);
    }
    let c = fs::metadata(candidate)?.modified()?;
    let b = fs::metadata(base)?.modified()?;
    Ok(c > b)
}

fn any_file_newer(dir: &Path, base: &Path) -> Result<bool> {
    if !dir.is_dir() || !base.is_file() {
        return Ok(false);
    }
    let base_ts = fs::metadata(base)?.modified()?;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in fs::read_dir(&current)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                stack.push(path);
            } else if entry.file_type()?.is_file() {
                let modified = fs::metadata(&path)?.modified()?;
                if modified > base_ts {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
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
            || p.starts_with("program/")
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

fn run_build_timing_sync(root: &Path) -> Result<()> {
    let start_all = std::time::Instant::now();

    let step = std::time::Instant::now();
    run_program_sync(
        "cargo install edgerun-cli",
        "cargo",
        &[
            "install",
            "--path",
            "crates/edgerun-cli",
            "--locked",
            "--force",
        ],
        root,
        false,
    )?;
    let install_secs = step.elapsed().as_secs();

    let step = std::time::Instant::now();
    run_program_sync(
        "cargo check --workspace",
        "cargo",
        &["check", "--workspace"],
        root,
        false,
    )?;
    let check_secs = step.elapsed().as_secs();

    let step = std::time::Instant::now();
    run_program_sync(
        "cargo build --release --workspace",
        "cargo",
        &["build", "--release", "--workspace"],
        root,
        false,
    )?;
    let release_secs = step.elapsed().as_secs();
    let total_secs = start_all.elapsed().as_secs();

    let out_dir = root.join("out/ci");
    fs::create_dir_all(&out_dir)?;
    let payload = json!({
        "run_id": std::env::var("GITHUB_RUN_ID").unwrap_or_else(|_| "local".to_string()),
        "sha": std::env::var("GITHUB_SHA").unwrap_or_else(|_| "local".to_string()),
        "install_seconds": install_secs,
        "check_seconds": check_secs,
        "release_build_seconds": release_secs,
        "total_seconds": total_secs
    });
    fs::write(
        out_dir.join("build-timings.json"),
        serde_json::to_vec_pretty(&payload)?,
    )?;

    println!("build timings:");
    println!("  install_seconds={install_secs}");
    println!("  check_seconds={check_secs}");
    println!("  release_build_seconds={release_secs}");
    println!("  total_seconds={total_secs}");

    if let Ok(summary_path) = std::env::var("GITHUB_STEP_SUMMARY") {
        let summary = format!(
            "## Build Timings\n\n| Step | Seconds |\n| --- | ---: |\n| cargo install --path crates/edgerun-cli --locked --force | {install_secs} |\n| cargo check --workspace | {check_secs} |\n| cargo build --release --workspace | {release_secs} |\n| Total | {total_secs} |\n"
        );
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(summary_path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, summary.as_bytes()));
    }

    Ok(())
}

fn run_program_anchor_build_sync(root: &Path) -> Result<()> {
    let program_root = root.join("program");
    let env = program_tool_env(&program_root);
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
    let env = program_tool_env(&program_root);
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
        .arg(program_target_dir(&program_root).join("deploy/edgerun.so"))
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

fn program_target_dir(program_root: &Path) -> PathBuf {
    std::env::var_os("EDGERUN_PROGRAM_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| program_root.join("target"))
}

fn program_tool_env(program_root: &Path) -> Vec<(OsString, OsString)> {
    let cargo_home = program_root.join(".cargo-home");
    let cargo_install_root = program_root.join(".cargo");
    let cargo_target_dir = program_target_dir(program_root);
    let cargo_bin_dir = cargo_install_root.join("bin");

    let mut paths = vec![cargo_bin_dir];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    let joined_path = std::env::join_paths(paths).unwrap_or_else(|_| OsString::from(""));

    vec![
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
    ]
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

async fn run_tailscale_command(command: TailscaleCommand) -> Result<()> {
    match command {
        TailscaleCommand::Bridge {
            listen,
            term_port,
            include_offline,
        } => run_tailscale_bridge(listen, term_port, include_offline).await,
    }
}

async fn run_tailscale_bridge(listen: SocketAddr, term_port: u16, include_offline: bool) -> Result<()> {
    if !command_exists("tailscale") {
        return Err(anyhow!(
            "tailscale CLI not found in PATH; install Tailscale first"
        ));
    }

    let state = TailscaleBridgeState {
        term_port,
        include_offline,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::OPTIONS])
        .allow_headers(Any)
        .max_age(Duration::from_secs(600));

    let app = Router::new()
        .route("/v1/tailscale/devices", get(tailscale_devices_handler))
        .with_state(state)
        .layer(cors);

    println!(
        "tailscale bridge listening on http://{listen} (term_port={term_port}, include_offline={include_offline})"
    );
    println!("endpoint: GET /v1/tailscale/devices");

    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .with_context(|| format!("failed to bind tailscale bridge on {listen}"))?;
    axum::serve(listener, app)
        .await
        .context("tailscale bridge server failed")?;
    Ok(())
}

async fn tailscale_devices_handler(
    State(state): State<TailscaleBridgeState>,
) -> impl IntoResponse {
    match discover_tailscale_devices(state.term_port, state.include_offline).await {
        Ok(devices) => (
            [(axum::http::header::CACHE_CONTROL, HeaderValue::from_static("no-store"))],
            Json(json!({
                "ok": true,
                "count": devices.len(),
                "devices": devices
            })),
        )
            .into_response(),
        Err(err) => (
            axum::http::StatusCode::BAD_GATEWAY,
            Json(json!({
                "ok": false,
                "error": err.to_string()
            })),
        )
            .into_response(),
    }
}

async fn discover_tailscale_devices(
    term_port: u16,
    include_offline: bool,
) -> Result<Vec<TailscaleBridgeDevice>> {
    let output = Command::new("tailscale")
        .arg("status")
        .arg("--json")
        .stdin(Stdio::null())
        .output()
        .await
        .context("failed to execute 'tailscale status --json'")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let msg = stderr.trim();
        if msg.is_empty() {
            return Err(anyhow!("tailscale status returned non-zero exit code"));
        }
        return Err(anyhow!(msg.to_string()));
    }

    let doc: Value = serde_json::from_slice(&output.stdout)
        .context("failed to parse tailscale status JSON")?;
    let mut devices = Vec::new();
    append_tailscale_entry(
        &mut devices,
        doc.get("Self"),
        true,
        term_port,
        include_offline,
    );
    if let Some(peers) = doc.get("Peer").and_then(|v| v.as_object()) {
        for peer in peers.values() {
            append_tailscale_entry(
                &mut devices,
                Some(peer),
                false,
                term_port,
                include_offline,
            );
        }
    }
    devices.sort_by(|a, b| a.name.cmp(&b.name));
    devices.dedup_by(|a, b| a.base_url == b.base_url);
    Ok(devices)
}

fn append_tailscale_entry(
    out: &mut Vec<TailscaleBridgeDevice>,
    entry: Option<&Value>,
    is_self: bool,
    term_port: u16,
    include_offline: bool,
) {
    let Some(entry) = entry else {
        return;
    };
    let online = entry
        .get("Online")
        .and_then(|v| v.as_bool())
        .unwrap_or(is_self);
    if !include_offline && !online {
        return;
    }

    let dns_name = entry
        .get("DNSName")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim_end_matches('.')
        .to_string();
    let host_name = entry
        .get("HostName")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let tailnet_name = if !host_name.is_empty() {
        host_name
    } else if !dns_name.is_empty() {
        dns_name.clone()
    } else {
        "Tailscale Device".to_string()
    };
    let base_host = if !dns_name.is_empty() {
        dns_name
    } else {
        let maybe_ip = entry
            .get("TailscaleIPs")
            .and_then(|v| v.as_array())
            .and_then(|ips| ips.first())
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if maybe_ip.is_empty() {
            return;
        }
        maybe_ip.to_string()
    };
    let source = if is_self { "tailscale-self" } else { "tailscale-peer" };
    out.push(TailscaleBridgeDevice {
        name: tailnet_name,
        base_url: format!("http://{base_host}:{term_port}"),
        online,
        source,
    });
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
    let summary = opts.out_dir.join("summary.mdx");
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
        .unwrap_or_else(|_| root.join("out/fuzz-weekly"));
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
    let fuzz_crash_dir = fuzz_dir.join("artifacts");
    if fuzz_crash_dir.exists() {
        std::fs::remove_dir_all(&fuzz_crash_dir)
            .with_context(|| format!("failed to clear {}", fuzz_crash_dir.display()))?;
    }
    std::fs::create_dir_all(&fuzz_crash_dir)?;

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

    let crash_count = count_files_recursive(&fuzz_crash_dir)?;
    if crash_count > 0 {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let out_dir = artifact_dir.join(format!("run-{stamp}"));
        copy_dir_recursive(&fuzz_crash_dir, &out_dir)?;
        return Err(anyhow!(
            "fuzz crashes detected: {crash_count} files (copied to {})",
            out_dir.display()
        ));
    }
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
    let override_var = match package {
        "edgerun-scheduler" => Some("EDGERUN_SCHEDULER_BIN"),
        "edgerun-worker" => Some("EDGERUN_WORKER_BIN"),
        _ => None,
    };

    let mut cmd = if let Some(var) = override_var {
        if let Ok(bin_path) = std::env::var(var) {
            let mut c = Command::new(bin_path);
            c.current_dir(root);
            c
        } else {
            let mut c = Command::new("cargo");
            c.arg("run").arg("-p").arg(package).current_dir(root);
            c
        }
    } else {
        let mut c = Command::new("cargo");
        c.arg("run").arg("-p").arg(package).current_dir(root);
        c
    };
    cmd.stdout(Stdio::from(log_file))
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

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).with_context(|| {
                format!(
                    "failed copying {} to {}",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
        }
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
