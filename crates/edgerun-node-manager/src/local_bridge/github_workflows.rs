// SPDX-License-Identifier: Apache-2.0
use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sonic_rs::{JsonContainerTrait, JsonValueMutTrait, JsonValueTrait, Value};

const WORKFLOW_RUNNER_RECORDS_PATH: &str = "/var/lib/edgerun/workflow-runner/runs.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LocalWorkflowRunnerRunRecord {
    pub(crate) id: String,
    pub(crate) workflow_id: String,
    pub(crate) status: String,
    pub(crate) started_unix_ms: u64,
    pub(crate) completed_unix_ms: u64,
    pub(crate) duration_ms: u64,
    pub(crate) message: String,
}

pub(crate) fn github_token_from(raw: &str) -> Result<String> {
    let token = raw.trim();
    if token.len() < 20 {
        return Err(anyhow!("github personal access token is missing or invalid"));
    }
    Ok(token.to_string())
}

fn github_runner_records_path() -> PathBuf {
    PathBuf::from(WORKFLOW_RUNNER_RECORDS_PATH)
}

pub(crate) fn load_github_runner_records() -> Vec<LocalWorkflowRunnerRunRecord> {
    let path = github_runner_records_path();
    let content = fs::read_to_string(path).ok().unwrap_or_default();
    sonic_rs::from_str::<Vec<LocalWorkflowRunnerRunRecord>>(&content)
        .ok()
        .unwrap_or_default()
}

fn save_github_runner_records(records: &[LocalWorkflowRunnerRunRecord]) -> Result<()> {
    let path = github_runner_records_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create workflow runner directory {}", parent.display()))?;
    }
    let encoded = sonic_rs::to_string(records).context("failed to encode workflow runner records")?;
    fs::write(&path, encoded)
        .with_context(|| format!("failed to write workflow runner records {}", path.display()))?;
    Ok(())
}

pub(crate) fn append_github_runner_record(record: LocalWorkflowRunnerRunRecord) -> Result<()> {
    let mut records = load_github_runner_records();
    records.insert(0, record);
    if records.len() > 120 {
        records.truncate(120);
    }
    save_github_runner_records(&records)
}

pub(crate) fn workflow_runner_command_for(workflow_id: &str) -> Option<&'static str> {
    match workflow_id {
        "intent-ui-ci" => Some("cd /workspace && ./scripts/workflow-runner-intent-ui-ci.sh"),
        _ => None,
    }
}

async fn github_api_request(token: &str, url: &str) -> Result<Value> {
    let client = Client::new();
    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .with_context(|| format!("github request failed: {url}"))?;
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .with_context(|| format!("failed to read github response: {url}"))?;
    if !status.is_success() {
        let detail = String::from_utf8_lossy(&bytes).to_string();
        return Err(anyhow!("github request failed ({status}): {detail}"));
    }
    sonic_rs::from_slice(&bytes).with_context(|| format!("failed to parse github response: {url}"))
}

pub(crate) async fn collect_github_workflow_runs(token: &str, per_page: usize) -> Result<Vec<Value>> {
    let repos_url = "https://api.github.com/user/repos?sort=updated&per_page=8";
    let repos_payload = github_api_request(token, repos_url).await?;
    let repos = repos_payload.as_array().cloned().unwrap_or_default();
    let mut runs = Vec::new();
    let per_repo = per_page.clamp(1, 10).min(5);
    for repo in repos.iter().take(6) {
        let owner = repo["owner"]["login"].as_str().unwrap_or_default().trim();
        let name = repo["name"].as_str().unwrap_or_default().trim();
        let full_name = repo["full_name"].as_str().unwrap_or_default().trim();
        if owner.is_empty() || name.is_empty() {
            continue;
        }
        let url = format!("https://api.github.com/repos/{owner}/{name}/actions/runs?per_page={per_repo}");
        let payload = match github_api_request(token, &url).await {
            Ok(value) => value,
            Err(_) => continue,
        };
        let repo_runs = payload["workflow_runs"].as_array().cloned().unwrap_or_default();
        for run in repo_runs {
            let mut next = run.clone();
            if let Some(obj) = next.as_object_mut() {
                obj.insert("repo_full_name", sonic_rs::json!(full_name));
            }
            runs.push(next);
        }
    }
    runs.sort_by(|a, b| {
        let left = a["created_at"].as_str().unwrap_or_default();
        let right = b["created_at"].as_str().unwrap_or_default();
        right.cmp(left)
    });
    runs.truncate(per_page.clamp(1, 60));
    Ok(runs)
}
