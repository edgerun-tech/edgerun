//! Image config types (from the config blob) with serde-based serialization.

use std::collections::HashMap;

use edgerun_json::from_slice;
use serde::{Deserialize, Serialize};

use super::manifest::{ImageIndex, ImageManifest, SingleManifest};

/// Parsed image config.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architecture: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<ImageConfigInner>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rootfs: Option<RootFs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<Vec<HistoryEntry>>,
}

/// Inner image config (Cmd, Env, WorkingDir, etc.).
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ImageConfigInner {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entrypoint: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmd: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exposed_ports: Option<HashMap<String, edgerun_json::JsonValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volumes: Option<HashMap<String, edgerun_json::JsonValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_signal: Option<String>,
}

/// Root filesystem info.
#[derive(Debug, Serialize, Deserialize)]
pub struct RootFs {
    pub r#type: String,
    pub diff_ids: Vec<String>,
}

/// History entry (build layer info, from OCI image config).
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct HistoryEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub empty_layer: Option<bool>,
}

// ===========================================================================
// Public API
// ===========================================================================

/// Parse an image config from JSON bytes.
pub fn parse_image_config(data: &[u8]) -> Result<ImageConfig, String> {
    from_slice(data).map_err(|e| e.to_string())
}

/// Parse JSON bytes into a JsonValue (for ad-hoc inspection).
pub fn parse_json_bytes(data: &[u8]) -> Result<edgerun_json::JsonValue, String> {
    edgerun_json::from_slice(data).map_err(|e| e.to_string())
}

// ===========================================================================
// Manifest parsing via serde
// ===========================================================================

/// Parse a raw JSON blob into an ImageManifest (index or single).
pub fn parse_manifest(data: &[u8]) -> Result<ImageManifest, String> {
    // Try as index first (has "manifests" array)
    if let Ok(index) = from_slice::<ImageIndex>(data) {
        if !index.manifests.is_empty() {
            return Ok(ImageManifest::Index(index));
        }
    }

    // Fall back to single manifest
    from_slice::<SingleManifest>(data)
        .map(ImageManifest::Single)
        .map_err(|e| e.to_string())
}

/// Parse a raw JSON blob as a single manifest.
pub fn parse_single_manifest(data: &[u8]) -> Result<SingleManifest, String> {
    from_slice(data).map_err(|e| e.to_string())
}
