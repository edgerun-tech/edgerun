// SPDX-License-Identifier: Apache-2.0
use anyhow::Result;
use axum::extract::Json;
use axum::http::header::{
    ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN,
    CACHE_CONTROL, CONTENT_TYPE as AXUM_CONTENT_TYPE,
};
use axum::http::HeaderValue;
use axum::http::StatusCode as AxumStatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct LocalDockerSummaryResponse {
    ok: bool,
    error: String,
    swarm_active: bool,
    swarm_node_id: String,
    services: Vec<LocalDockerService>,
    containers: Vec<LocalDockerContainer>,
}

#[derive(Debug, Serialize)]
struct LocalDockerService {
    id: String,
    name: String,
    mode: String,
    replicas: String,
    image: String,
    ports: String,
}

#[derive(Debug, Serialize)]
struct LocalDockerContainer {
    id: String,
    name: String,
    image: String,
    status: String,
    state: String,
    ports: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LocalDockerContainerStateRequest {
    container: String,
    action: String,
}

pub(crate) async fn handle_local_docker_summary() -> Response {
    let summary = match collect_local_docker_summary() {
        Ok(summary) => summary,
        Err(err) => LocalDockerSummaryResponse {
            ok: false,
            error: err.to_string(),
            swarm_active: false,
            swarm_node_id: String::new(),
            services: Vec::new(),
            containers: Vec::new(),
        },
    };
    let payload = sonic_rs::to_string(&summary)
        .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode failed\"}".to_string());
    (
        AxumStatusCode::OK,
        [
            (
                AXUM_CONTENT_TYPE,
                HeaderValue::from_static("application/json; charset=utf-8"),
            ),
            (CACHE_CONTROL, HeaderValue::from_static("no-store")),
            (ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*")),
            (
                ACCESS_CONTROL_ALLOW_HEADERS,
                HeaderValue::from_static("content-type"),
            ),
            (
                ACCESS_CONTROL_ALLOW_METHODS,
                HeaderValue::from_static("GET, OPTIONS"),
            ),
        ],
        payload,
    )
        .into_response()
}

fn docker_container_state(selector: &str) -> Result<String> {
    let output = crate::run_command_capture(
        "docker",
        &["inspect", selector, "--format", "{{.State.Status}}"],
    )?;
    Ok(output.lines().next().unwrap_or_default().trim().to_string())
}

pub(crate) async fn handle_local_docker_container_state(
    Json(body): Json<LocalDockerContainerStateRequest>,
) -> Response {
    let selector = match super::docker_local::docker_container_selector_from(&body.container) {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let action = match super::docker_local::docker_container_action_from(&body.action) {
        Ok(value) => value,
        Err(err) => return crate::local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    if let Err(err) = crate::run_command_capture("docker", &["container", &action, &selector]) {
        return crate::local_json_error(AxumStatusCode::BAD_GATEWAY, &err.to_string());
    }
    let state = docker_container_state(&selector).unwrap_or_default();
    crate::local_json_value(
        AxumStatusCode::OK,
        sonic_rs::json!({
            "ok": true,
            "container": selector,
            "action": action,
            "state": state,
            "message": "container state updated",
        }),
    )
}

fn collect_local_docker_summary() -> Result<LocalDockerSummaryResponse> {
    let swarm_info = crate::run_command_capture(
        "docker",
        &[
            "info",
            "--format",
            "{{.Swarm.LocalNodeState}}\t{{.Swarm.NodeID}}",
        ],
    )?;
    let mut swarm_state = String::new();
    let mut swarm_node_id = String::new();
    if let Some(line) = swarm_info.lines().next() {
        let cols: Vec<&str> = line.split('\t').collect();
        swarm_state = cols.first().copied().unwrap_or_default().trim().to_string();
        swarm_node_id = cols.get(1).copied().unwrap_or_default().trim().to_string();
    }
    let swarm_active = swarm_state == "active";

    let mut services = Vec::new();
    if swarm_active {
        let rows = crate::run_command_capture(
            "docker",
            &[
                "service",
                "ls",
                "--format",
                "{{.ID}}\t{{.Name}}\t{{.Mode}}\t{{.Replicas}}\t{{.Image}}\t{{.Ports}}",
            ],
        )?;
        for line in rows.lines() {
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() < 5 {
                continue;
            }
            services.push(LocalDockerService {
                id: cols.first().copied().unwrap_or_default().trim().to_string(),
                name: cols.get(1).copied().unwrap_or_default().trim().to_string(),
                mode: cols.get(2).copied().unwrap_or_default().trim().to_string(),
                replicas: cols.get(3).copied().unwrap_or_default().trim().to_string(),
                image: cols.get(4).copied().unwrap_or_default().trim().to_string(),
                ports: cols.get(5).copied().unwrap_or_default().trim().to_string(),
            });
        }
    }

    let rows = crate::run_command_capture(
        "docker",
        &[
            "ps",
            "--format",
            "{{.ID}}\t{{.Names}}\t{{.Image}}\t{{.Status}}\t{{.State}}\t{{.Ports}}",
        ],
    )?;
    let mut containers = Vec::new();
    for line in rows.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 5 {
            continue;
        }
        containers.push(LocalDockerContainer {
            id: cols.first().copied().unwrap_or_default().trim().to_string(),
            name: cols.get(1).copied().unwrap_or_default().trim().to_string(),
            image: cols.get(2).copied().unwrap_or_default().trim().to_string(),
            status: cols.get(3).copied().unwrap_or_default().trim().to_string(),
            state: cols.get(4).copied().unwrap_or_default().trim().to_string(),
            ports: cols.get(5).copied().unwrap_or_default().trim().to_string(),
        });
    }

    Ok(LocalDockerSummaryResponse {
        ok: true,
        error: String::new(),
        swarm_active,
        swarm_node_id,
        services,
        containers,
    })
}
