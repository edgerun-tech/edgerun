use schemars::JsonSchema;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct FeedbackUploadParams {
    pub classification: String,
    pub reason: Option<String>,
    pub thread_id: Option<String>,
    pub include_logs: bool,
    pub extra_log_files: Option<Vec<PathBuf>>,
    pub tags: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, JsonSchema, edgerun_json::ToJson, edgerun_json::FromJson)]
#[schemars(rename_all = "camelCase")]
pub struct FeedbackUploadResponse {
    pub thread_id: String,
}
