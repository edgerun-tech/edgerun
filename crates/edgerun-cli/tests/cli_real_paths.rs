// SPDX-License-Identifier: Apache-2.0
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use edgerun_storage::event_bus::{EventBusPolicyV1, PolicyRuleV1, PolicyUpdateRequestV1};
use prost::Message;
use tempfile::tempdir;

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_edgerun")
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn run_ok(args: &[&str]) -> Output {
    let output = Command::new(bin_path())
        .current_dir(workspace_root())
        .args(args)
        .output()
        .expect("spawn edgerun");
    assert!(
        output.status.success(),
        "command failed: {}\nstderr:\n{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[test]
fn event_bus_submit_and_query_real_path() {
    let tmp = tempdir().expect("tempdir");
    let data_dir = tmp.path().join("bus");
    let data_dir_s = data_dir.to_string_lossy().to_string();

    let init = PolicyUpdateRequestV1 {
        schema_version: 1,
        policy: Some(EventBusPolicyV1 {
            version: 1,
            rules: vec![
                PolicyRuleV1 {
                    publisher: "*".to_string(),
                    payload_type: "policy_update_request".to_string(),
                },
                PolicyRuleV1 {
                    publisher: "scheduler".to_string(),
                    payload_type: "job_created".to_string(),
                },
            ],
        }),
    };
    let init_b64 = BASE64.encode(init.encode_to_vec());
    let out_init = run_ok(&[
        "--root",
        ".",
        "event",
        "submit",
        "--data-dir",
        &data_dir_s,
        "--segment",
        "events.seg",
        "--nonce",
        "1",
        "--publisher",
        "scheduler",
        "--signature",
        "sig",
        "--policy-id",
        "p1",
        "--recipient",
        "*",
        "--payload-type",
        "policy_update_request",
        "--payload-base64",
        &init_b64,
    ]);
    let init_stdout = String::from_utf8_lossy(&out_init.stdout);
    assert!(init_stdout.contains("ok=true"));

    let job_payload_b64 = BASE64.encode(br#"{"job_id":"j1"}"#);
    let out_submit = run_ok(&[
        "--root",
        ".",
        "event",
        "submit",
        "--data-dir",
        &data_dir_s,
        "--segment",
        "events.seg",
        "--nonce",
        "2",
        "--publisher",
        "scheduler",
        "--signature",
        "sig",
        "--policy-id",
        "p1",
        "--recipient",
        "worker-a",
        "--payload-type",
        "job_created",
        "--payload-base64",
        &job_payload_b64,
    ]);
    let submit_stdout = String::from_utf8_lossy(&out_submit.stdout);
    assert!(submit_stdout.contains("ok=true"));

    let out_query = run_ok(&[
        "--root",
        ".",
        "event",
        "query",
        "--data-dir",
        &data_dir_s,
        "--segment",
        "events.seg",
        "--limit",
        "20",
        "--publisher",
        "scheduler",
        "--payload-type",
        "job_created",
    ]);
    let query_stdout = String::from_utf8_lossy(&out_query.stdout);
    assert!(query_stdout.contains("events_count=1"));
    assert!(query_stdout.contains("publisher=scheduler"));
    assert!(query_stdout.contains("payload_type=job_created"));
    assert!(query_stdout.contains("payload_base64=eyJqb2JfaWQiOiJqMSJ9"));

    let out_status = run_ok(&[
        "--root",
        ".",
        "event",
        "status",
        "--data-dir",
        &data_dir_s,
        "--segment",
        "events.seg",
    ]);
    let status_stdout = String::from_utf8_lossy(&out_status.stdout);
    assert!(status_stdout.contains("ok=true"));
    assert!(status_stdout.contains("phase=2"));
    assert!(status_stdout.contains("policy_version=1"));
}

#[test]
fn timeline_append_query_text_file_and_stdin_real_path() {
    let tmp = tempdir().expect("tempdir");
    let data_dir = tmp.path().join("timeline");
    let data_dir_s = data_dir.to_string_lossy().to_string();
    let file_path = tmp.path().join("payload.txt");
    std::fs::write(&file_path, "file payload").expect("write payload file");
    let file_s = file_path.to_string_lossy().to_string();

    let out_text = run_ok(&[
        "--root",
        ".",
        "timeline",
        "append",
        "--data-dir",
        &data_dir_s,
        "--segment",
        "interactions.seg",
        "--run-id",
        "run-1",
        "--job-id",
        "job-1",
        "--session-id",
        "sess-1",
        "--actor",
        "user",
        "--actor-id",
        "tester",
        "--kind",
        "user-input",
        "--text",
        "inline payload",
    ]);
    assert!(String::from_utf8_lossy(&out_text.stdout).contains("ok=true"));

    let out_file = run_ok(&[
        "--root",
        ".",
        "timeline",
        "append",
        "--data-dir",
        &data_dir_s,
        "--segment",
        "interactions.seg",
        "--run-id",
        "run-1",
        "--job-id",
        "job-1",
        "--session-id",
        "sess-1",
        "--actor",
        "user",
        "--actor-id",
        "tester",
        "--kind",
        "user-input",
        "--text-file",
        &file_s,
    ]);
    assert!(String::from_utf8_lossy(&out_file.stdout).contains("ok=true"));

    let mut child = Command::new(bin_path())
        .current_dir(workspace_root())
        .args([
            "--root",
            ".",
            "timeline",
            "append",
            "--data-dir",
            &data_dir_s,
            "--segment",
            "interactions.seg",
            "--run-id",
            "run-1",
            "--job-id",
            "job-1",
            "--session-id",
            "sess-1",
            "--actor",
            "user",
            "--actor-id",
            "tester",
            "--kind",
            "user-input",
            "--stdin",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn timeline append --stdin");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(b"stdin payload").expect("write stdin payload");
    }
    let out_stdin = child.wait_with_output().expect("collect output");
    assert!(
        out_stdin.status.success(),
        "stdin append failed: {}",
        String::from_utf8_lossy(&out_stdin.stderr)
    );
    assert!(String::from_utf8_lossy(&out_stdin.stdout).contains("ok=true"));

    let out_query = run_ok(&[
        "--root",
        ".",
        "timeline",
        "query",
        "--data-dir",
        &data_dir_s,
        "--segment",
        "interactions.seg",
        "--run-id",
        "run-1",
        "--actor-id",
        "tester",
        "--kind",
        "user-input",
        "--limit",
        "20",
    ]);
    let query_stdout = String::from_utf8_lossy(&out_query.stdout);
    assert!(query_stdout.contains("events_count=3"));
    assert!(query_stdout.contains("text_base64=aW5saW5lIHBheWxvYWQ="));
    assert!(query_stdout.contains("text_base64=ZmlsZSBwYXlsb2Fk"));
    assert!(query_stdout.contains("text_base64=c3RkaW4gcGF5bG9hZA=="));

    let out_status = run_ok(&[
        "--root",
        ".",
        "timeline",
        "status",
        "--data-dir",
        &data_dir_s,
        "--segment",
        "interactions.seg",
    ]);
    let status_stdout = String::from_utf8_lossy(&out_status.stdout);
    assert!(status_stdout.contains("ok=true"));
    assert!(status_stdout.contains("events_total=3"));
    assert!(status_stdout.contains("unique_run_ids=1"));
    assert!(status_stdout.contains("unique_job_ids=1"));
    assert!(status_stdout.contains("unique_session_ids=1"));
}

#[test]
fn execution_lifecycle_emit_and_query_run_real_path() {
    let tmp = tempdir().expect("tempdir");
    let data_dir = tmp.path().join("exec");
    let data_dir_s = data_dir.to_string_lossy().to_string();

    run_ok(&[
        "--root",
        ".",
        "execution",
        "intent-submitted",
        "--data-dir",
        &data_dir_s,
        "--segment",
        "exec.seg",
        "--run-id",
        "run-l1",
        "--job-id",
        "job-l1",
        "--session-id",
        "sess-l1",
        "--actor-id",
        "planner",
        "--intent-id",
        "intent-l1",
        "--intent-text",
        "compile and run",
    ]);
    run_ok(&[
        "--root",
        ".",
        "execution",
        "execution-started",
        "--data-dir",
        &data_dir_s,
        "--segment",
        "exec.seg",
        "--run-id",
        "run-l1",
        "--job-id",
        "job-l1",
        "--session-id",
        "sess-l1",
        "--actor-id",
        "executor",
        "--intent-id",
        "intent-l1",
        "--executor-id",
        "exec-1",
    ]);
    run_ok(&[
        "--root",
        ".",
        "execution",
        "step-started",
        "--data-dir",
        &data_dir_s,
        "--segment",
        "exec.seg",
        "--run-id",
        "run-l1",
        "--job-id",
        "job-l1",
        "--session-id",
        "sess-l1",
        "--actor-id",
        "executor",
        "--step-id",
        "s1",
    ]);
    run_ok(&[
        "--root",
        ".",
        "execution",
        "step-finished",
        "--data-dir",
        &data_dir_s,
        "--segment",
        "exec.seg",
        "--run-id",
        "run-l1",
        "--job-id",
        "job-l1",
        "--session-id",
        "sess-l1",
        "--actor-id",
        "executor",
        "--step-id",
        "s1",
        "--state",
        "succeeded",
        "--reason",
        "ok",
    ]);
    run_ok(&[
        "--root",
        ".",
        "execution",
        "execution-finished",
        "--data-dir",
        &data_dir_s,
        "--segment",
        "exec.seg",
        "--run-id",
        "run-l1",
        "--job-id",
        "job-l1",
        "--session-id",
        "sess-l1",
        "--actor-id",
        "executor",
        "--state",
        "succeeded",
        "--reason",
        "complete",
    ]);

    let out_query = run_ok(&[
        "--root",
        ".",
        "execution",
        "query-run",
        "--data-dir",
        &data_dir_s,
        "--segment",
        "exec.seg",
        "--run-id",
        "run-l1",
        "--limit",
        "20",
    ]);
    let query_stdout = String::from_utf8_lossy(&out_query.stdout);
    assert!(query_stdout.contains("events_count=5"));
    assert!(query_stdout.contains("payload_type=os.intent_submitted.v1"));
    assert!(query_stdout.contains("payload_type=os.execution_started.v1"));
    assert!(query_stdout.contains("payload_type=os.execution_step_started.v1"));
    assert!(query_stdout.contains("payload_type=os.execution_step_finished.v1"));
    assert!(query_stdout.contains("payload_type=os.execution_finished.v1"));
}

#[test]
fn execution_failed_maps_to_job_failed_event_type() {
    let tmp = tempdir().expect("tempdir");
    let data_dir = tmp.path().join("exec-fail");
    let data_dir_s = data_dir.to_string_lossy().to_string();

    run_ok(&[
        "--root",
        ".",
        "execution",
        "intent-submitted",
        "--data-dir",
        &data_dir_s,
        "--segment",
        "exec.seg",
        "--run-id",
        "run-f1",
        "--job-id",
        "job-f1",
        "--session-id",
        "sess-f1",
        "--actor-id",
        "planner",
        "--intent-id",
        "intent-f1",
        "--intent-text",
        "fail path",
    ]);
    run_ok(&[
        "--root",
        ".",
        "execution",
        "execution-finished",
        "--data-dir",
        &data_dir_s,
        "--segment",
        "exec.seg",
        "--run-id",
        "run-f1",
        "--job-id",
        "job-f1",
        "--session-id",
        "sess-f1",
        "--actor-id",
        "executor",
        "--state",
        "failed",
        "--reason",
        "boom",
    ]);

    let out_query = run_ok(&[
        "--root",
        ".",
        "execution",
        "query-run",
        "--data-dir",
        &data_dir_s,
        "--segment",
        "exec.seg",
        "--run-id",
        "run-f1",
        "--limit",
        "20",
    ]);
    let query_stdout = String::from_utf8_lossy(&out_query.stdout);
    assert!(query_stdout.contains("events_count=2"));
    assert!(query_stdout.contains("payload_type=os.execution_finished.v1"));
    assert!(query_stdout.contains("event_type=8"));
}
