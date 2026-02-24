// SPDX-License-Identifier: Apache-2.0
use std::path::Path;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use tokio::time::timeout;

use crate::{
    command_exists,
    commands::integration::{
        run_integration_abi_rollover, run_integration_e2e_lifecycle,
        run_integration_policy_rotation, run_integration_scheduler_api,
    },
    commands::runtime_ops::{run_replay_corpus, validate_external_security_review},
    load_app_config, run_build_runtime_web_sync, run_build_timing_sync, run_clean_artifacts_sync,
    run_matrix_validation_check_sync, run_program_sync_owned, run_rust_checks_sync,
    run_spdx_check_sync, run_verify_sync,
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

pub async fn run_ci(root: &Path, job: Option<String>, event: String, dry_run: bool) -> Result<()> {
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
        "push-scheduler" => run_program_sync_owned(
            "Push scheduler",
            "bash",
            &["scripts/push-scheduler.sh".to_string()],
            root,
            false,
        ),
        other => Err(anyhow!("unsupported ci job in native mode: {other}")),
    }
}
