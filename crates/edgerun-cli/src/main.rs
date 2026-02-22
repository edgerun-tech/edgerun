// SPDX-License-Identifier: Apache-2.0
use std::ffi::OsString;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;
use serde_json::json;

mod commands;
mod integration_helpers;
mod process_helpers;

use crate::commands::ci::run_ci;
use crate::commands::integration::{
    run_integration_abi_rollover, run_integration_e2e_lifecycle, run_integration_policy_rotation,
    run_integration_scheduler_api,
};
use crate::commands::program::run_program_command;
use crate::commands::runtime_ops::{
    compare_replay_profiles, run_replay_corpus, run_weekly_fuzz, validate_external_security_review,
};
use crate::commands::storage::run_storage_command;
use crate::commands::tailscale::run_tailscale_command;
use crate::commands::tasks::{run_interactive, run_named_task_sync};
use process_helpers::{
    command_exists, run_program_sync, run_program_sync_owned, run_program_sync_with_env,
};

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
    Program {
        #[command(subcommand)]
        command: ProgramCommand,
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
enum ProgramCommand {
    AnalyzeAccounts {
        #[arg(value_enum, long, default_value_t = SolanaCluster::Devnet)]
        cluster: SolanaCluster,
    },
    Deploy {
        #[arg(value_enum, long, default_value_t = SolanaCluster::Devnet)]
        cluster: SolanaCluster,
        #[arg(long, default_value_t = false)]
        skip_build: bool,
        #[arg(long = "final", default_value_t = false)]
        final_immutable: bool,
        #[arg(long)]
        program_id: Option<String>,
        #[arg(long)]
        keypair: Option<PathBuf>,
        #[arg(long)]
        fee_payer: Option<PathBuf>,
        #[arg(long)]
        max_len: Option<u32>,
        #[arg(long, default_value_t = false)]
        no_update_frontend_config: bool,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum SolanaCluster {
    Localnet,
    Devnet,
    Testnet,
    #[value(name = "mainnet-beta")]
    MainnetBeta,
}

impl SolanaCluster {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Localnet => "localhost",
            Self::Devnet => "devnet",
            Self::Testnet => "testnet",
            Self::MainnetBeta => "mainnet-beta",
        }
    }
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
        Commands::Program { command } => run_program_command(&root, command)?,
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

fn is_generated_spdx_exempt(path: &str) -> bool {
    path.starts_with("program/target/")
        || path.starts_with("program/target-local/")
        || path.starts_with("out/")
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

fn run_build_timing_sync(root: &Path) -> Result<()> {
    let start_all = std::time::Instant::now();

    let step = std::time::Instant::now();
    run_program_sync(
        "cargo build --release -p edgerun-cli",
        "cargo",
        &["build", "--release", "-p", "edgerun-cli"],
        root,
        false,
    )?;
    let cli_release_secs = step.elapsed().as_secs();

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
        "cli_release_build_seconds": cli_release_secs,
        "check_seconds": check_secs,
        "release_build_seconds": release_secs,
        "total_seconds": total_secs
    });
    fs::write(
        out_dir.join("build-timings.json"),
        serde_json::to_vec_pretty(&payload)?,
    )?;

    println!("build timings:");
    println!("  cli_release_build_seconds={cli_release_secs}");
    println!("  check_seconds={check_secs}");
    println!("  release_build_seconds={release_secs}");
    println!("  total_seconds={total_secs}");

    if let Ok(summary_path) = std::env::var("GITHUB_STEP_SUMMARY") {
        let summary = format!(
            "## Build Timings\n\n| Step | Seconds |\n| --- | ---: |\n| cargo build --release -p edgerun-cli | {cli_release_secs} |\n| cargo check --workspace | {check_secs} |\n| cargo build --release --workspace | {release_secs} |\n| Total | {total_secs} |\n"
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
    let program_id = "A2ac8yDnTXKfZCHWqcJVYFfR2jv65kezW95XTgrrdbtG";
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

pub(crate) fn program_tool_env(program_root: &Path) -> Vec<(OsString, OsString)> {
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
