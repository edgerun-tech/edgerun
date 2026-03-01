// SPDX-License-Identifier: Apache-2.0
use axum::extract::{Json, Query};
use axum::http::StatusCode as AxumStatusCode;
use axum::response::Response;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct LocalGithubWorkflowRunsQuery {
    token: String,
    #[serde(default)]
    per_page: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LocalGithubWorkflowRunnerRunRequest {
    workflow_id: String,
}

async fn execute_local_workflow_runner(
    workflow_id: &str,
) -> super::github_workflows::LocalWorkflowRunnerRunRecord {
    let started_unix_ms = crate::now_unix_ms();
    let run_id = format!("local-{}-{}", workflow_id, started_unix_ms);
    let mut status = "success".to_string();
    let mut message = "local workflow runner completed".to_string();
    let command = match super::github_workflows::workflow_runner_command_for(workflow_id) {
        Some(value) => value,
        None => {
            return super::github_workflows::LocalWorkflowRunnerRunRecord {
                id: run_id,
                workflow_id: workflow_id.to_string(),
                status: "error".to_string(),
                started_unix_ms,
                completed_unix_ms: crate::now_unix_ms(),
                duration_ms: 0,
                message: "unsupported workflow id".to_string(),
            }
        }
    };
    let exec_result = crate::run_command_capture(
        "docker",
        &["exec", "edgerun-osdev-frontend", "/bin/sh", "-lc", command],
    );
    if let Err(err) = exec_result {
        status = "failure".to_string();
        message = err.to_string();
    }
    let completed_unix_ms = crate::now_unix_ms();
    super::github_workflows::LocalWorkflowRunnerRunRecord {
        id: run_id,
        workflow_id: workflow_id.to_string(),
        status,
        started_unix_ms,
        completed_unix_ms,
        duration_ms: completed_unix_ms.saturating_sub(started_unix_ms),
        message,
    }
}

pub(crate) async fn handle_local_github_workflow_runs(
    Query(query): Query<LocalGithubWorkflowRunsQuery>,
) -> Response {
    let token = match super::github_workflows::github_token_from(&query.token) {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let per_page = query.per_page.unwrap_or(20).clamp(1, 60);
    let runs = match super::github_workflows::collect_github_workflow_runs(&token, per_page).await {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string()),
    };
    crate::local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "runs": runs,
            "count": runs.len(),
        }),
    )
}

pub(crate) async fn handle_local_github_workflow_runner_runs() -> Response {
    let runs = super::github_workflows::load_github_runner_records();
    crate::local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "runs": runs,
            "count": runs.len(),
        }),
    )
}

pub(crate) async fn handle_local_github_workflow_runner_run(
    Json(body): Json<LocalGithubWorkflowRunnerRunRequest>,
) -> Response {
    let workflow_id = body.workflow_id.trim().to_string();
    if super::github_workflows::workflow_runner_command_for(&workflow_id).is_none() {
        return crate::local_json_error(AxumStatusCode::BAD_REQUEST, "unsupported workflow_id");
    }
    let run = execute_local_workflow_runner(&workflow_id).await;
    if let Err(err) = super::github_workflows::append_github_runner_record(run.clone()) {
        return crate::local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
    }
    crate::local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "run": run,
        }),
    )
}
