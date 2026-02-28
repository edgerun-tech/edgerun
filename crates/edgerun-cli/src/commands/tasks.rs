// SPDX-License-Identifier: Apache-2.0
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

use crate::commands::integration::{
    run_integration_abi_rollover, run_integration_e2e_lifecycle, run_integration_policy_rotation,
    run_integration_scheduler_api,
};
use crate::commands::runtime_ops::{
    run_replay_corpus, run_weekly_fuzz, validate_external_security_review,
};
use crate::commands::storage::run_storage_command;
use crate::{
    load_app_config, run_build_runtime_web_sync, run_clean_artifacts_sync, run_doctor_sync,
    run_matrix_validation_check_sync, run_program_sync, run_rust_checks_sync, run_setup_sync,
    run_spdx_check_sync, run_verify_sync, StorageCommand,
};

pub(crate) async fn run_interactive(root: &Path) -> Result<()> {
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
        ("push-scheduler", "Push code + restart scheduler stack"),
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

pub(crate) fn run_named_task_sync(root: &Path, task: &str) -> Result<()> {
    if run_native_task_sync(root, task)? {
        Ok(())
    } else {
        Err(anyhow!("unknown task: {task}"))
    }
}

pub(crate) async fn run_named_task_async(root: PathBuf, task: String) -> Result<String> {
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

pub(crate) async fn run_named_task_sync_blocking(root: PathBuf, task: String) -> Result<()> {
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
        "dev" => {
            run_rust_checks_sync(root)?;
            Ok(true)
        }
        "push-scheduler" => {
            run_program_sync(
                "Push scheduler",
                "bash",
                &["scripts/push-scheduler.sh"],
                root,
                false,
            )?;
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
