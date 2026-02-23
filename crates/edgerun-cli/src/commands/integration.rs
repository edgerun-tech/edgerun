// SPDX-License-Identifier: Apache-2.0
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use anyhow::{Context, Result};
use edgerun_types::control_plane::{
    ControlWsRequestPayload, JobCreateRequest, JobStatusResponse, ReplayArtifactPayload,
    WorkerFailureReport, WorkerReplayArtifactReport, WorkerResultReport,
};
use edgerun_types::Limits;
use tokio::process::Command;
use tokio::time::sleep;

use crate::integration_helpers::{
    control_ws_request, create_assigned_job, create_assigned_job_with_abi, create_temp_dir,
    fetch_job_status, kill_child, pick_free_port, submit_worker_failure, submit_worker_replay,
    submit_worker_result, wait_for_failure_phase, wait_for_health,
    wait_for_runtime_execute_failure,
};
use crate::process_helpers::run_program_sync;
use crate::{ensure, integration_flag_env, load_app_config};

static LOCAL_BIN_CACHE: OnceLock<Mutex<HashMap<String, PathBuf>>> = OnceLock::new();

pub(crate) async fn run_integration_scheduler_api(root: &Path) -> Result<()> {
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
    let create_request = JobCreateRequest {
        runtime_id: runtime_id.to_string(),
        wasm_base64: "AA==".to_string(),
        input_base64: String::new(),
        abi_version: None,
        limits: Limits {
            max_memory_bytes: 1_048_576,
            max_instructions: 10_000,
        },
        escrow_lamports: 100,
        assignment_worker_pubkey: Some("worker-a".to_string()),
        client_pubkey: None,
        client_signed_at_unix_s: None,
        client_signature: None,
    };
    let create = match control_ws_request(
        &sched_url,
        ControlWsRequestPayload::JobCreate(create_request),
    )
    .await?
    {
        edgerun_types::control_plane::ControlWsResponsePayload::JobCreate(v) => v,
        other => anyhow::bail!("unexpected payload for job.create: {other:?}"),
    };
    let job_id = create.job_id;
    let bundle_hash = create.bundle_hash;

    let output_hash_1 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let output_hash_2 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let output_hash_3 = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    let r1 = WorkerResultReport {
        idempotency_key: "k-result-1".to_string(),
        worker_pubkey: "worker-a".to_string(),
        job_id: job_id.clone(),
        bundle_hash: bundle_hash.clone(),
        output_hash: output_hash_1.to_string(),
        output_len: 10,
        attestation_sig: None,
        attestation_claim: None,
        signature: None,
    };
    ensure(
        !submit_worker_result(&sched_url, r1.clone()).await?,
        "result first submit should not duplicate",
    )?;
    ensure(
        submit_worker_result(&sched_url, r1).await?,
        "result second submit should duplicate",
    )?;

    let f1 = WorkerFailureReport {
        idempotency_key: "k-failure-1".to_string(),
        worker_pubkey: "worker-a".to_string(),
        job_id: job_id.clone(),
        bundle_hash: bundle_hash.clone(),
        phase: "runtime_execute".to_string(),
        error_code: "InstructionLimitExceeded".to_string(),
        error_message: "out of fuel".to_string(),
        signature: None,
    };
    ensure(
        !submit_worker_failure(&sched_url, f1.clone()).await?,
        "failure first submit should not duplicate",
    )?;
    ensure(
        submit_worker_failure(&sched_url, f1).await?,
        "failure second submit should duplicate",
    )?;

    submit_worker_result(
        &sched_url,
        WorkerResultReport {
            idempotency_key: "k-result-2".to_string(),
            worker_pubkey: "worker-a".to_string(),
            job_id: job_id.clone(),
            bundle_hash: bundle_hash.clone(),
            output_hash: output_hash_2.to_string(),
            output_len: 20,
            attestation_sig: None,
            attestation_claim: None,
            signature: None,
        },
    )
    .await?;
    submit_worker_result(
        &sched_url,
        WorkerResultReport {
            idempotency_key: "k-result-3".to_string(),
            worker_pubkey: "worker-a".to_string(),
            job_id: job_id.clone(),
            bundle_hash: bundle_hash.clone(),
            output_hash: output_hash_3.to_string(),
            output_len: 30,
            attestation_sig: None,
            attestation_claim: None,
            signature: None,
        },
    )
    .await?;
    submit_worker_failure(
        &sched_url,
        WorkerFailureReport {
            idempotency_key: "k-failure-2".to_string(),
            worker_pubkey: "worker-a".to_string(),
            job_id: job_id.clone(),
            bundle_hash: bundle_hash.clone(),
            phase: "post_execution_verify".to_string(),
            error_code: "BundleHashMismatch".to_string(),
            error_message: "mismatch".to_string(),
            signature: None,
        },
    )
    .await?;
    submit_worker_failure(
        &sched_url,
        WorkerFailureReport {
            idempotency_key: "k-failure-3".to_string(),
            worker_pubkey: "worker-a".to_string(),
            job_id: job_id.clone(),
            bundle_hash: bundle_hash.clone(),
            phase: "runtime_execute".to_string(),
            error_code: "Trap".to_string(),
            error_message: "trap".to_string(),
            signature: None,
        },
    )
    .await?;
    submit_worker_replay(
        &sched_url,
        WorkerReplayArtifactReport {
            idempotency_key: "k-replay-2".to_string(),
            worker_pubkey: "worker-a".to_string(),
            job_id: job_id.clone(),
            artifact: ReplayArtifactPayload {
                bundle_hash: bundle_hash.clone(),
                ok: true,
                abi_version: Some(1),
                runtime_id: Some(runtime_id.to_string()),
                output_hash: Some(output_hash_2.to_string()),
                output_len: Some(20),
                input_len: Some(3),
                max_memory_bytes: Some(1024),
                max_instructions: Some(1000),
                fuel_limit: Some(1000),
                fuel_remaining: Some(900),
                error_code: None,
                error_message: None,
                trap_code: None,
            },
            signature: None,
        },
    )
    .await?;
    submit_worker_replay(
        &sched_url,
        WorkerReplayArtifactReport {
            idempotency_key: "k-replay-3".to_string(),
            worker_pubkey: "worker-a".to_string(),
            job_id: job_id.clone(),
            artifact: ReplayArtifactPayload {
                bundle_hash: bundle_hash.clone(),
                ok: true,
                abi_version: Some(1),
                runtime_id: Some(runtime_id.to_string()),
                output_hash: Some(output_hash_3.to_string()),
                output_len: Some(30),
                input_len: Some(3),
                max_memory_bytes: Some(1024),
                max_instructions: Some(1000),
                fuel_limit: Some(1000),
                fuel_remaining: Some(800),
                error_code: None,
                error_message: None,
                trap_code: None,
            },
            signature: None,
        },
    )
    .await?;

    let status = fetch_job_status(&sched_url, &job_id).await?;

    let reports = status.reports;
    let failures = status.failures;
    let replays = status.replay_artifacts;
    ensure(reports.len() == 2, "expected 2 reports")?;
    ensure(failures.len() == 2, "expected 2 failures")?;
    ensure(replays.len() == 2, "expected 2 replay artifacts")?;
    ensure(
        reports
            .last()
            .map(|x| x.output_hash.as_str())
            .unwrap_or_default()
            == output_hash_3,
        "expected newest result output_hash=o3",
    )?;
    ensure(
        failures
            .last()
            .map(|x| x.idempotency_key.as_str())
            .unwrap_or_default()
            == "k-failure-3",
        "expected newest failure idempotency key",
    )?;
    ensure(
        replays
            .last()
            .map(|x| x.idempotency_key.as_str())
            .unwrap_or_default()
            == "k-replay-3",
        "expected newest replay idempotency key",
    )?;

    kill_child(&mut scheduler).await;
    let _ = std::fs::remove_dir_all(tmp_dir);
    Ok(())
}

pub(crate) async fn run_integration_e2e_lifecycle(root: &Path) -> Result<()> {
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

    let create_request = JobCreateRequest {
        runtime_id: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        wasm_base64: "AA==".to_string(),
        input_base64: String::new(),
        abi_version: None,
        limits: Limits {
            max_memory_bytes: 1_048_576,
            max_instructions: 10_000,
        },
        escrow_lamports: 100,
        assignment_worker_pubkey: Some(worker_pubkey.to_string()),
        client_pubkey: None,
        client_signed_at_unix_s: None,
        client_signature: None,
    };
    let create = match control_ws_request(
        &sched_url,
        ControlWsRequestPayload::JobCreate(create_request),
    )
    .await?
    {
        edgerun_types::control_plane::ControlWsResponsePayload::JobCreate(v) => v,
        other => anyhow::bail!("unexpected payload for job.create: {other:?}"),
    };
    let job_id = create.job_id;

    let mut success = false;
    for _ in 0..240 {
        if worker.try_wait()?.is_some() {
            break;
        }
        if scheduler.try_wait()?.is_some() {
            break;
        }
        let status: JobStatusResponse = fetch_job_status(&sched_url, &job_id).await?;
        let has_fail = !status.failures.is_empty();
        let has_replay = !status.replay_artifacts.is_empty();
        if has_fail && has_replay {
            let artifact_ok = status
                .replay_artifacts
                .last()
                .map(|last| last.artifact.ok)
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

pub(crate) async fn run_integration_policy_rotation(root: &Path) -> Result<()> {
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
            (
                "EDGERUN_SCHEDULER_REQUIRE_POLICY_SESSION",
                "false".to_string(),
            ),
        ],
    )
    .await?;
    wait_for_health(&client, &sched_url, &mut scheduler).await?;

    let policy: edgerun_types::control_plane::PolicyInfoResponse = client
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

pub(crate) async fn run_integration_abi_rollover(root: &Path) -> Result<()> {
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

    let unsupported_err = control_ws_request(
        &sched_url,
        ControlWsRequestPayload::JobCreate(JobCreateRequest {
            runtime_id: "0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            wasm_base64: "AA==".to_string(),
            input_base64: String::new(),
            abi_version: Some(3),
            limits: Limits {
                max_memory_bytes: 1_048_576,
                max_instructions: 10_000,
            },
            escrow_lamports: 100,
            assignment_worker_pubkey: Some(worker_pubkey.to_string()),
            client_pubkey: None,
            client_signed_at_unix_s: None,
            client_signature: None,
        }),
    )
    .await
    .expect_err("unsupported ABI must fail");
    ensure(
        unsupported_err.to_string().contains("(400)")
            || unsupported_err.to_string().contains("abi_version"),
        "expected bad request error for unsupported ABI",
    )?;

    kill_child(&mut worker).await;
    kill_child(&mut scheduler).await;
    let _ = std::fs::remove_dir_all(tmp_dir);
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
            let bin_path = ensure_local_bin(root, package)?;
            let mut c = Command::new(bin_path);
            c.current_dir(root);
            c
        }
    } else {
        let bin_path = ensure_local_bin(root, package)?;
        let mut c = Command::new(bin_path);
        c.current_dir(root);
        c
    };
    cmd.stdout(Stdio::from(log_file))
        .stderr(Stdio::from(log_file_err));
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.spawn()
        .with_context(|| format!("failed to spawn {package}"))
}

fn ensure_local_bin(root: &Path, package: &str) -> Result<PathBuf> {
    let cache = LOCAL_BIN_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(cached) = cache.lock().expect("lock poisoned").get(package).cloned() {
        if cached.is_file() {
            return Ok(cached);
        }
    }

    run_program_sync(
        "Build integration binary",
        "cargo",
        &["build", "-p", package],
        root,
        false,
    )?;

    let target_dir = resolve_target_dir(root);
    let exe_name = if cfg!(windows) {
        format!("{package}.exe")
    } else {
        package.to_string()
    };
    let bin_path = target_dir.join("debug").join(exe_name);
    ensure(
        bin_path.is_file(),
        &format!("expected built binary at {}", bin_path.display()),
    )?;

    cache
        .lock()
        .expect("lock poisoned")
        .insert(package.to_string(), bin_path.clone());
    Ok(bin_path)
}

fn resolve_target_dir(root: &Path) -> PathBuf {
    if let Some(dir) = std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from) {
        return dir;
    }

    let output = std::process::Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--no-deps")
        .current_dir(root)
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if let Some(dir) = extract_json_string_field(&stdout, "target_directory") {
                return PathBuf::from(dir);
            }
        }
    }

    root.join("target")
}

fn extract_json_string_field(raw: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\":\"");
    let start = raw.find(&needle)? + needle.len();
    let rest = &raw[start..];
    let end_rel = rest.find('"')?;
    Some(rest[..end_rel].to_string())
}
